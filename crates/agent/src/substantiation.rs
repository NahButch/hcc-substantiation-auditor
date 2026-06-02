//! Substantiation job: for one engine-triggered HCC, judge whether the clinical
//! note supports the coded diagnosis under CMS M.E.A.T./specificity standards.

use crate::llm::{LlmError, LlmProvider, Prompt};
use serde::Deserialize;

const SYSTEM: &str = include_str!("../prompts/substantiation.txt");

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Judgment {
    /// "supported" | "risky" | "unsupported".
    pub status: String,
    #[serde(default)]
    pub meat_present: Vec<String>,
    #[serde(default)]
    pub specificity_supported: bool,
    #[serde(default)]
    pub citation: String,
    #[serde(default)]
    pub documentation_gap: String,
    #[serde(default)]
    pub rationale: String,
    /// Model's self-reported confidence in this judgment (0.0–1.0), if provided.
    #[serde(default)]
    pub confidence: Option<f64>,
}

impl Judgment {
    pub fn is_supported(&self) -> bool {
        self.status.eq_ignore_ascii_case("supported")
    }
    pub fn is_flagged(&self) -> bool {
        !self.is_supported()
    }
}

/// A coded HCC to be substantiated, plus the diagnoses that triggered it.
pub struct Target<'a> {
    pub hcc: u32,
    pub diagnoses: &'a [(String, String)], // (icd10, description)
}

/// Ask the LLM to judge substantiation of one coded HCC against the note.
pub fn judge(
    provider: &dyn LlmProvider,
    note: &str,
    target: &Target,
) -> Result<Judgment, LlmError> {
    let dx = target
        .diagnoses
        .iter()
        .map(|(c, d)| if d.is_empty() { c.clone() } else { format!("{c} ({d})") })
        .collect::<Vec<_>>()
        .join(", ");
    let user = format!(
        "Coded for risk adjustment: HCC {} — triggered by diagnosis code(s): {}.\n\n\
         Clinical note (the full available documentation):\n\"\"\"\n{}\n\"\"\"\n\n\
         Does this note substantiate the coded diagnosis above to CMS standards?",
        target.hcc, dx, note
    );
    let prompt = Prompt { system: SYSTEM, user: &user, json: true };
    let raw = provider.complete(&prompt)?;
    let json = crate::json::extract_object(&raw)
        .ok_or_else(|| LlmError::BadResponse(format!("no JSON object in: {raw}")))?;
    serde_json::from_str(json).map_err(|e| LlmError::BadResponse(format!("{e}: {json}")))
}
