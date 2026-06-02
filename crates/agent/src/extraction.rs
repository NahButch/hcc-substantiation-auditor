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
    Ok(parsed.conditions)
}
