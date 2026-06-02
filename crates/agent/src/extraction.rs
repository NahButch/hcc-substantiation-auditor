//! Extraction job: clinical note → documented conditions with supporting spans.

use crate::llm::{LlmError, LlmProvider, Prompt};
use serde::Deserialize;

const SYSTEM: &str = include_str!("../prompts/extraction.txt");

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ExtractedCondition {
    pub name: String,
    #[serde(default)]
    pub evidence: String,
    #[serde(default)]
    pub section: String,
    #[serde(default)]
    pub icd10: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExtractionResponse {
    #[serde(default)]
    conditions: Vec<ExtractedCondition>,
}

/// Extract documented conditions from a clinical note via the LLM.
pub fn extract(
    provider: &dyn LlmProvider,
    note: &str,
) -> Result<Vec<ExtractedCondition>, LlmError> {
    let prompt = Prompt { system: SYSTEM, user: note, json: true };
    let raw = provider.complete(&prompt)?;
    let json = crate::json::extract_object(&raw)
        .ok_or_else(|| LlmError::BadResponse(format!("no JSON object in: {raw}")))?;
    let parsed: ExtractionResponse =
        serde_json::from_str(json).map_err(|e| LlmError::BadResponse(format!("{e}: {json}")))?;
    let mut conditions = parsed.conditions;
    // Backfill a target-family ICD-10 when the model omitted one, so downstream
    // (and the extraction metric) can link the condition to an HCC.
    for c in &mut conditions {
        let empty = c.icd10.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true);
        if empty {
            c.icd10 = backfill_icd10(&c.name);
        }
    }
    Ok(conditions)
}

/// Deterministic keyword → ICD-10-CM resolver for the four target families.
/// Used only to backfill when the LLM omits a code; order is significant
/// (most-specific first). Returns the dotless code, or `None` if out of scope.
pub fn backfill_icd10(name: &str) -> Option<String> {
    let n = name.to_lowercase();
    let has = |kws: &[&str]| kws.iter().any(|k| n.contains(k));
    let code = if has(&["prediabet"]) {
        "R7303"
    } else if has(&["diabet"]) && has(&["kidney", "nephropath", "ckd", "renal"]) {
        "E1122"
    } else if has(&["diabet"]) && has(&["neuropath"]) {
        "E1140"
    } else if has(&["diabet"]) && has(&["retinopath"]) {
        "E11319"
    } else if has(&["diabet"]) {
        "E119"
    } else if has(&["end stage renal", "end-stage renal", "esrd"])
        || (has(&["kidney", "ckd"]) && has(&["stage 5", "stage v"]))
    {
        "N186"
    } else if has(&["kidney", "ckd", "renal"]) && has(&["stage 4", "stage iv"]) {
        "N184"
    } else if has(&["kidney", "ckd", "renal"]) && has(&["stage 3", "stage iii"]) {
        "N1830"
    } else if has(&["kidney", "ckd", "renal"]) && has(&["stage 2", "stage ii"]) {
        "N182" // near-miss (no HCC) — preserved so it's scored honestly
    } else if has(&["kidney", "ckd", "renal"]) && has(&["stage 1", "stage i"]) {
        "N181" // near-miss
    } else if has(&["heart failure", "chf", "hfref", "hfpef", "cardiac failure"]) {
        "I509"
    } else if has(&["emphysema", "copd", "chronic obstructive"]) {
        "J439"
    } else {
        return None;
    };
    Some(code.to_string())
}

#[cfg(test)]
mod tests {
    use super::backfill_icd10;

    #[test]
    fn backfill_maps_families() {
        assert_eq!(backfill_icd10("Type 2 diabetes with chronic kidney disease").as_deref(), Some("E1122"));
        assert_eq!(backfill_icd10("Type 2 diabetes mellitus").as_deref(), Some("E119"));
        assert_eq!(backfill_icd10("Prediabetes").as_deref(), Some("R7303"));
        assert_eq!(backfill_icd10("Chronic kidney disease, stage 4").as_deref(), Some("N184"));
        assert_eq!(backfill_icd10("Chronic congestive heart failure").as_deref(), Some("I509"));
        assert_eq!(backfill_icd10("Pulmonary emphysema").as_deref(), Some("J439"));
        assert_eq!(backfill_icd10("Ankle sprain"), None);
    }
}

