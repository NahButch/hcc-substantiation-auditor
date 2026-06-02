//! Single-pass audit orchestration: coded ground truth (engine) + note extraction
//! + per-HCC substantiation judgment → a flagged-code report.

use crate::crosswalk::Crosswalk;
use crate::extraction::{self, ExtractedCondition};
use crate::fhir::PatientRecord;
use crate::groundtruth;
use crate::llm::{LlmError, LlmProvider};
use crate::substantiation::{self, Judgment, Target};
use engine::{Engine, ModelVersion};
use std::fmt::Write as _;

/// The substantiation result for one engine-triggered HCC.
#[derive(Debug, Clone)]
pub struct HccAudit {
    pub hcc: u32,
    pub coefficient: f64,
    pub diagnoses: Vec<(String, String)>, // (icd10, description)
    pub judgment: Judgment,
}

/// The full single-pass audit for one patient.
#[derive(Debug, Clone)]
pub struct PatientAudit {
    pub patient_id: String,
    pub name: String,
    pub age: i32,
    pub raw_score: f64,
    pub model: &'static str,
    pub extracted: Vec<ExtractedCondition>,
    pub hccs: Vec<HccAudit>,
}

impl PatientAudit {
    /// HCCs whose documentation the auditor flagged (not "supported").
    pub fn flagged(&self) -> impl Iterator<Item = &HccAudit> {
        self.hccs.iter().filter(|h| h.judgment.is_flagged())
    }
}

/// Run the single-pass audit for one patient.
pub fn audit_patient<V: ModelVersion>(
    engine: &Engine<V>,
    provider: &dyn LlmProvider,
    xwalk: &Crosswalk,
    rec: &PatientRecord,
) -> Result<PatientAudit, LlmError> {
    // 1. Coded ground truth from the engine oracle.
    let score = groundtruth::ground_truth(engine, rec, xwalk);

    // 2. Extract documented conditions from the note (LLM job 1).
    let extracted = extraction::extract(provider, &rec.note)?;

    // 3. Substantiate each engine-triggered HCC against the note (LLM job 2).
    let mut hccs = Vec::new();
    for h in &score.hccs {
        let diagnoses: Vec<(String, String)> = h
            .triggering_diagnoses
            .iter()
            .map(|c| (c.clone(), xwalk.describe_icd10(c).unwrap_or("").to_string()))
            .collect();
        let target = Target { hcc: h.hcc, diagnoses: &diagnoses };
        let judgment = substantiation::judge(provider, &rec.note, &target)?;
        hccs.push(HccAudit { hcc: h.hcc, coefficient: h.coefficient, diagnoses, judgment });
    }

    Ok(PatientAudit {
        patient_id: rec.id.clone(),
        name: rec.name.clone(),
        age: score.age,
        raw_score: score.raw_score,
        model: score.model,
        extracted,
        hccs,
    })
}

/// Render a human-readable flagged-code report.
pub fn render_report(a: &PatientAudit) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "Patient {} ({}), age {} — {} risk score {:.3}",
        a.patient_id, a.name, a.age, a.model, a.raw_score
    );
    let _ = writeln!(s, "Engine-triggered HCCs: {}", a.hccs.len());
    let flagged: Vec<_> = a.flagged().collect();
    let _ = writeln!(s, "Flagged (not substantiated): {}\n", flagged.len());

    for h in &a.hccs {
        let mark = if h.judgment.is_supported() { "OK " } else { "FLAG" };
        let dx = h
            .diagnoses
            .iter()
            .map(|(c, d)| if d.is_empty() { c.clone() } else { format!("{c} {d}") })
            .collect::<Vec<_>>()
            .join("; ");
        let _ = writeln!(
            s,
            "[{}] HCC {} (coef {:.3}) — {}",
            mark, h.hcc, h.coefficient, dx
        );
        let _ = writeln!(s, "      status: {}", h.judgment.status);
        if !h.judgment.meat_present.is_empty() {
            let _ = writeln!(s, "      M.E.A.T.: {}", h.judgment.meat_present.join(", "));
        }
        if h.judgment.is_flagged() {
            if !h.judgment.documentation_gap.is_empty() {
                let _ = writeln!(s, "      gap: {}", h.judgment.documentation_gap);
            }
            if !h.judgment.citation.is_empty() {
                let _ = writeln!(s, "      citation: {}", h.judgment.citation);
            }
        }
        if !h.judgment.rationale.is_empty() {
            let _ = writeln!(s, "      rationale: {}", h.judgment.rationale);
        }
    }
    s
}
