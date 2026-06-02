//! VERIFY stage: cross-check the LLM's single-pass judgments against the engine
//! oracle and the note extraction, and surface disagreements for self-correction.
//!
//! The engine's coded HCC set and score are AUTHORITATIVE — verification never
//! adds, drops, or re-scores an HCC. It only questions whether the LLM's
//! *substantiation status* is consistent with the documentary evidence the
//! extraction actually found (verbatim) in the note.
//!
//! Two contradictions are detected per engine-triggered HCC:
//!   * `SupportedNoEvidence` — the LLM called it "supported", yet no extracted
//!     condition that maps to this HCC has a span actually present in the note.
//!     (The "supported but the evidence span isn't in the note" case.)
//!   * `FlaggedWithEvidence` — the LLM flagged it (risky/unsupported), yet the
//!     extraction *did* surface a documented condition, with a verbatim note span,
//!     that maps to this HCC. (The note may substantiate it after all.)
//!
//! It also reports the informational `ExtractionImpliesUntriggered` case
//! (extraction implies an HCC the engine did NOT trigger): logged for visibility,
//! but never promoted into the coded set — the engine remains the source of truth.

use crate::crosswalk::Crosswalk;
use crate::extraction::ExtractedCondition;
use crate::substantiation::Judgment;
use engine::HccAssignment;

/// Why an engine-triggered HCC's LLM judgment is being re-examined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisagreementKind {
    /// LLM marked "supported", but no in-note documented condition maps to this HCC.
    SupportedNoEvidence,
    /// LLM flagged it, but extraction found a documented condition for it in the note.
    FlaggedWithEvidence,
}

impl DisagreementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DisagreementKind::SupportedNoEvidence => "supported_no_evidence",
            DisagreementKind::FlaggedWithEvidence => "flagged_with_evidence",
        }
    }
}

/// A detected contradiction on one engine-triggered HCC, with the engine and
/// extraction facts to surface back to the model during self-correction.
#[derive(Debug, Clone)]
pub struct Disagreement {
    pub hcc: u32,
    pub kind: DisagreementKind,
    /// The LLM's initial status that is being questioned.
    pub initial_status: String,
    /// The note span(s) the extraction tied to this HCC ("none" if unsupported).
    pub note_evidence: String,
    /// Human-readable framing of the contradiction (also fed to the re-prompt).
    pub detail: String,
}

/// Whether a documented condition with a verbatim note span maps to `hcc`,
/// and (if so) the supporting span/section. The extraction's evidence must be a
/// real substring of the note — a hallucinated span does not count as documented.
pub fn note_documents_hcc(
    hcc: &HccAssignment,
    extracted: &[ExtractedCondition],
    note: &str,
    xwalk: &Crosswalk,
) -> Option<String> {
    let note_norm = normalize(note);
    for c in extracted {
        if !condition_maps_to_hcc(c, hcc, xwalk) {
            continue;
        }
        // The span must actually appear in the note (anti-hallucination guard).
        let span = if c.evidence.trim().is_empty() { &c.name } else { &c.evidence };
        if note_norm.contains(&normalize(span)) {
            let section = if c.section.trim().is_empty() { "" } else { &c.section };
            return Some(if section.is_empty() {
                format!("\"{}\"", span.trim())
            } else {
                format!("\"{}\" ({})", span.trim(), section.trim())
            });
        }
    }
    None
}

/// Does this extracted condition imply the given engine HCC? Tries, in order:
/// the extracted ICD-10 is one of the HCC's triggering codes; the crosswalk maps
/// the extracted ICD-10 to this HCC; or a meaningful clinical term is shared
/// between the condition name and a triggering-diagnosis description.
fn condition_maps_to_hcc(c: &ExtractedCondition, hcc: &HccAssignment, xwalk: &Crosswalk) -> bool {
    if let Some(icd10) = &c.icd10 {
        let norm = engine::types::normalize_icd10(icd10);
        if hcc.triggering_diagnoses.iter().any(|t| t.eq_ignore_ascii_case(&norm)) {
            return true;
        }
        if xwalk.hcc_for_icd10(&norm) == Some(hcc.hcc) {
            return true;
        }
    }
    // Name/description term overlap as a fallback when no (matching) code is given.
    let name_terms = clinical_terms(&c.name);
    hcc.triggering_diagnoses.iter().any(|t| {
        xwalk
            .describe_icd10(t)
            .map(|desc| {
                let dterms = clinical_terms(desc);
                name_terms.iter().any(|w| dterms.contains(w))
            })
            .unwrap_or(false)
    })
}

/// Detect disagreements across all engine-triggered HCCs given their judgments.
/// `judged` pairs each engine HCC with the LLM's single-pass judgment, in order.
pub fn detect(
    judged: &[(&HccAssignment, &Judgment)],
    extracted: &[ExtractedCondition],
    note: &str,
    xwalk: &Crosswalk,
) -> Vec<Disagreement> {
    let mut out = Vec::new();
    for (hcc, j) in judged {
        let documented = note_documents_hcc(hcc, extracted, note, xwalk);
        match (j.is_supported(), &documented) {
            (true, None) => out.push(Disagreement {
                hcc: hcc.hcc,
                kind: DisagreementKind::SupportedNoEvidence,
                initial_status: j.status.clone(),
                note_evidence: "none".into(),
                detail: format!(
                    "The engine coded HCC {} (from {}), and you judged it SUPPORTED — but \
                     the note extraction surfaced no documented condition for it whose \
                     evidence text is actually present in the note. A 'supported' status \
                     therefore has no verbatim note evidence behind it.",
                    hcc.hcc,
                    hcc.triggering_diagnoses.join(", "),
                ),
            }),
            (false, Some(span)) => out.push(Disagreement {
                hcc: hcc.hcc,
                kind: DisagreementKind::FlaggedWithEvidence,
                initial_status: j.status.clone(),
                note_evidence: span.clone(),
                detail: format!(
                    "The engine coded HCC {} (from {}), and you flagged it as '{}' — but \
                     the note extraction found a documented condition for it with a verbatim \
                     span in the note: {}. Re-check whether that span (with M.E.A.T.) actually \
                     substantiates the code.",
                    hcc.hcc,
                    hcc.triggering_diagnoses.join(", "),
                    j.status,
                    span,
                ),
            }),
            _ => {}
        }
    }
    out
}

/// HCCs that the *extraction* implies (via crosswalk) but the engine did NOT
/// trigger. Informational only — the engine's coded set is authoritative, so these
/// are surfaced for visibility, never added to the audit.
pub fn extraction_implied_untriggered(
    extracted: &[ExtractedCondition],
    engine_hccs: &[HccAssignment],
    note: &str,
    xwalk: &Crosswalk,
) -> Vec<u32> {
    let note_norm = normalize(note);
    let triggered: std::collections::HashSet<u32> = engine_hccs.iter().map(|h| h.hcc).collect();
    let mut implied: Vec<u32> = extracted
        .iter()
        .filter_map(|c| {
            let icd10 = c.icd10.as_ref()?;
            let hcc = xwalk.hcc_for_icd10(icd10)?;
            let span = if c.evidence.trim().is_empty() { &c.name } else { &c.evidence };
            (!triggered.contains(&hcc) && note_norm.contains(&normalize(span))).then_some(hcc)
        })
        .collect();
    implied.sort_unstable();
    implied.dedup();
    implied
}

/// Lowercase and collapse internal whitespace for tolerant substring matching.
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Meaningful clinical terms in a string (length ≥ 4, minus generic qualifiers),
/// for coarse name↔description matching when codes are absent.
fn clinical_terms(s: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "type", "with", "without", "mellitus", "disease", "chronic", "acute",
        "unspecified", "other", "stage", "moderate", "severe", "mild", "disorder",
        "due", "and", "the", "right", "left", "bilateral",
    ];
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4 && !STOP.contains(w))
        .map(|w| w.to_string())
        .collect()
}
