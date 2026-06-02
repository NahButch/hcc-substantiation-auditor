//! SELF-CORRECT stage: re-prompt the LLM on one specific HCC with the verification
//! disagreement surfaced, and adopt the re-examined judgment.
//!
//! The model is told the engine fact (this HCC *was* coded — authoritative) and the
//! exact contradiction, then asked to re-derive substantiation from the note. We
//! accept neither the prior answer nor a naive flip blindly: the re-prompt forbids
//! both and requires justification against the note + engine facts.

use crate::llm::{LlmError, LlmProvider, Prompt};
use crate::substantiation::{Judgment, Target};
use crate::verify::Disagreement;
use serde::Deserialize;

const SYSTEM: &str = include_str!("../prompts/self_correct.txt");

/// The re-examined judgment plus the model's stated reason for the re-examination.
#[derive(Debug, Clone, Deserialize)]
struct Recheck {
    #[serde(flatten)]
    judgment: Judgment,
    #[serde(default)]
    correction_reason: String,
}

/// Re-judge one HCC with the disagreement surfaced. Returns the revised judgment
/// and the model's one-line reason for the re-examination.
pub fn recheck(
    provider: &dyn LlmProvider,
    note: &str,
    target: &Target,
    disagreement: &Disagreement,
) -> Result<(Judgment, String), LlmError> {
    let dx = target
        .diagnoses
        .iter()
        .map(|(c, d)| if d.is_empty() { c.clone() } else { format!("{c} ({d})") })
        .collect::<Vec<_>>()
        .join(", ");
    let user = format!(
        "HCC {hcc} was coded for risk adjustment, triggered by diagnosis code(s): {dx}.\n\
         This fact is authoritative and cannot be changed.\n\n\
         VERIFICATION FINDING ({kind}):\n{detail}\n\n\
         Your initial status for this HCC was: {initial}.\n\
         Note evidence the verifier tied to this HCC: {evidence}.\n\n\
         Clinical note (the full available documentation):\n\"\"\"\n{note}\n\"\"\"\n\n\
         Re-examine: does THIS note substantiate the coded diagnosis to CMS standards? \
         Justify against the note and the verification finding above.",
        hcc = target.hcc,
        dx = dx,
        kind = disagreement.kind.as_str(),
        detail = disagreement.detail,
        initial = disagreement.initial_status,
        evidence = disagreement.note_evidence,
        note = note,
    );
    let prompt = Prompt { system: SYSTEM, user: &user, json: true };
    let raw = provider.complete(&prompt)?;
    let json = crate::json::extract_object(&raw)
        .ok_or_else(|| LlmError::BadResponse(format!("no JSON object in: {raw}")))?;
    let rc: Recheck =
        serde_json::from_str(json).map_err(|e| LlmError::BadResponse(format!("{e}: {json}")))?;
    Ok((rc.judgment, rc.correction_reason))
}
