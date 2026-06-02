//! Record types + JSONL I/O for the evaluation harness (ported from `hcceval/schema.py`).
//!
//! Three record types cross the harness boundary: [`AuditRecord`] (the agent's
//! output / metrics INPUT contract), [`GoldLabel`] (substantiation ground truth),
//! and [`Candidate`] (engine-eligible HCCs to be labeled). Loading is lenient
//! about unknown keys (forward-compatible with the agent contract) and tolerant of
//! single-pass output that carries only `status`.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A status denotes a substantiated code only when it is exactly "supported";
/// "risky" and "unsupported" both collapse to *flagged*.
pub fn is_supported(status: &str) -> bool {
    status.trim().eq_ignore_ascii_case("supported")
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedCondition {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub evidence: String,
    #[serde(default)]
    pub section: String,
    #[serde(default)]
    pub icd10: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HccAudit {
    pub hcc: u32,
    #[serde(default)]
    pub coefficient: f64,
    #[serde(default)]
    pub triggering_diagnoses: Vec<String>,
    #[serde(default)]
    pub initial_status: String,
    #[serde(default)]
    pub final_status: String,
    #[serde(default)]
    pub status: String, // single-pass fallback
    #[serde(default)]
    pub corrected: bool,
    #[serde(default)]
    pub correction_reason: String,
    #[serde(default)]
    pub meat_present: Vec<String>,
    #[serde(default)]
    pub specificity_supported: bool,
    #[serde(default)]
    pub citation: String,
    #[serde(default)]
    pub documentation_gap: String,
    #[serde(default)]
    pub confidence: Option<f64>,
}

impl HccAudit {
    /// Final status, falling back to `status` for single-pass output.
    pub fn final_status(&self) -> &str {
        if !self.final_status.is_empty() {
            &self.final_status
        } else {
            &self.status
        }
    }
    pub fn initial_status(&self) -> &str {
        if !self.initial_status.is_empty() {
            &self.initial_status
        } else {
            self.final_status()
        }
    }
    pub fn final_supported(&self) -> bool {
        is_supported(self.final_status())
    }
    pub fn initial_supported(&self) -> bool {
        is_supported(self.initial_status())
    }
    /// The oracle loop moved this code across the supported/flagged line.
    pub fn changed(&self) -> bool {
        let (i, f) = (self.initial_status(), self.final_status());
        !i.is_empty() && !f.is_empty() && (self.initial_supported() != self.final_supported())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditRecord {
    pub patient_id: String,
    #[serde(default)]
    pub age: i32,
    #[serde(default)]
    pub segment: String,
    #[serde(default)]
    pub raw_score: f64,
    #[serde(default)]
    pub extracted: Vec<ExtractedCondition>,
    #[serde(default)]
    pub hccs: Vec<HccAudit>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoldLabel {
    pub patient_id: String,
    pub hcc: u32,
    pub gold_status: String, // "supported" | "unsupported"
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub holdout: bool,
}

fn default_source() -> String {
    "manual".to_string()
}

impl GoldLabel {
    pub fn supported(&self) -> bool {
        self.gold_status.trim().eq_ignore_ascii_case("supported")
    }
    pub fn key(&self) -> (String, u32) {
        (self.patient_id.clone(), self.hcc)
    }
    /// Validate the status/source vocabulary (mirrors the Python loader's checks).
    pub fn validate(&self) -> Result<(), String> {
        let s = self.gold_status.trim().to_lowercase();
        if s != "supported" && s != "unsupported" {
            return Err(format!("bad gold_status {:?} ({},{})", self.gold_status, self.patient_id, self.hcc));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Candidate {
    pub patient_id: String,
    pub hcc: u32,
    #[serde(default)]
    pub triggering_icd10: Vec<String>,
    #[serde(default)]
    pub triggering_snomed: Vec<String>,
}

impl Candidate {
    pub fn key(&self) -> (String, u32) {
        (self.patient_id.clone(), self.hcc)
    }
}

/// Parse a JSONL file into a vector of `T`, skipping blank lines.
pub fn read_jsonl<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> std::io::Result<Vec<T>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: T = serde_json::from_str(line).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, format!("line {}: {e}", n + 1))
        })?;
        out.push(v);
    }
    Ok(out)
}

/// Write serializable rows to a JSONL file. Returns the row count.
pub fn write_jsonl<T: Serialize>(path: impl AsRef<Path>, rows: &[T]) -> std::io::Result<usize> {
    let mut s = String::new();
    for r in rows {
        s.push_str(&serde_json::to_string(r).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?);
        s.push('\n');
    }
    std::fs::write(path, s)?;
    Ok(rows.len())
}

pub fn load_audits(path: impl AsRef<Path>) -> std::io::Result<Vec<AuditRecord>> {
    read_jsonl(path)
}
pub fn load_gold(path: impl AsRef<Path>) -> std::io::Result<Vec<GoldLabel>> {
    let g: Vec<GoldLabel> = read_jsonl(path)?;
    for label in &g {
        label.validate().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    }
    Ok(g)
}
pub fn load_candidates(path: impl AsRef<Path>) -> std::io::Result<Vec<Candidate>> {
    read_jsonl(path)
}
