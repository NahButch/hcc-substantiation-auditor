//! Single-pass audit orchestration: coded ground truth (engine) + note extraction
//! + per-HCC substantiation judgment → a flagged-code report.

use crate::crosswalk::Crosswalk;
use crate::extraction::{self, ExtractedCondition};
use crate::fhir::PatientRecord;
use crate::groundtruth;
use crate::llm::{LlmError, LlmProvider};
use crate::substantiation::{self, Judgment, Target};
use crate::{self_correct, verify};
use engine::{Engine, ModelVersion, Segment};
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

/// One engine-triggered HCC after the verify → self-correct loop, retaining both
/// the initial single-pass judgment and the (possibly re-examined) final one.
#[derive(Debug, Clone)]
pub struct VerifiedHccAudit {
    pub hcc: u32,
    pub coefficient: f64,
    pub diagnoses: Vec<(String, String)>, // (icd10, description)
    /// The single-pass judgment, before verification.
    pub initial: Judgment,
    /// The judgment after self-correction (== `initial` when not re-examined).
    pub final_judgment: Judgment,
    /// Whether the final status differs from the initial status.
    pub corrected: bool,
    /// Why this HCC was re-examined and what changed (empty if not re-examined).
    pub correction_reason: String,
}

impl VerifiedHccAudit {
    pub fn final_is_supported(&self) -> bool {
        self.final_judgment.is_supported()
    }
}

/// The full verify → self-correct audit for one patient.
#[derive(Debug, Clone)]
pub struct VerifiedPatientAudit {
    pub patient_id: String,
    pub name: String,
    pub age: i32,
    pub segment: &'static str,
    pub raw_score: f64,
    pub model: &'static str,
    pub extracted: Vec<ExtractedCondition>,
    pub hccs: Vec<VerifiedHccAudit>,
    /// HCCs the extraction implied but the engine did NOT trigger (informational;
    /// the engine's coded set is authoritative and these are never added).
    pub extraction_implied_untriggered: Vec<u32>,
}

impl VerifiedPatientAudit {
    /// HCCs whose status was changed by the self-correct loop.
    pub fn corrected(&self) -> impl Iterator<Item = &VerifiedHccAudit> {
        self.hccs.iter().filter(|h| h.corrected)
    }
}

/// Stable string label for an engine segment (for the shared output contract).
pub fn segment_label(seg: Segment) -> &'static str {
    match seg {
        Segment::CommunityNonDualAged => "CommunityNonDualAged",
        Segment::CommunityPartialDualAged => "CommunityPartialDualAged",
        Segment::CommunityFullDualAged => "CommunityFullDualAged",
        Segment::CommunityNonDualDisabled => "CommunityNonDualDisabled",
        Segment::CommunityPartialDualDisabled => "CommunityPartialDualDisabled",
        Segment::CommunityFullDualDisabled => "CommunityFullDualDisabled",
    }
}

/// Run the full Phase 3a audit for one patient: single-pass judgments, then VERIFY
/// against the engine oracle + extraction, then SELF-CORRECT each disagreement.
///
/// The engine's coded HCC set and `raw_score` are authoritative and untouched: the
/// loop only re-examines per-HCC *substantiation status*, never membership or score.
pub fn audit_patient_verified<V: ModelVersion>(
    engine: &Engine<V>,
    provider: &dyn LlmProvider,
    xwalk: &Crosswalk,
    rec: &PatientRecord,
) -> Result<VerifiedPatientAudit, LlmError> {
    // 1. Coded ground truth from the engine oracle (authoritative).
    let score = groundtruth::ground_truth(engine, rec, xwalk);

    // 2. Extract documented conditions from the note (LLM job 1).
    let extracted = extraction::extract(provider, &rec.note)?;

    // 3. Single-pass substantiation of each engine-triggered HCC (LLM job 2).
    let mut initial = Vec::with_capacity(score.hccs.len());
    for h in &score.hccs {
        let diagnoses: Vec<(String, String)> = h
            .triggering_diagnoses
            .iter()
            .map(|c| (c.clone(), xwalk.describe_icd10(c).unwrap_or("").to_string()))
            .collect();
        let target = Target { hcc: h.hcc, diagnoses: &diagnoses };
        let judgment = substantiation::judge(provider, &rec.note, &target)?;
        initial.push((diagnoses, judgment));
    }

    // 4. VERIFY: cross-check judgments against the oracle + extraction.
    let judged: Vec<_> = score
        .hccs
        .iter()
        .zip(initial.iter())
        .map(|(h, (_, j))| (h, j))
        .collect();
    let disagreements = verify::detect(&judged, &extracted, &rec.note, xwalk);
    let by_hcc: std::collections::HashMap<u32, &verify::Disagreement> =
        disagreements.iter().map(|d| (d.hcc, d)).collect();

    // 5. SELF-CORRECT: re-examine each disagreement; adopt the re-judged status.
    let mut hccs = Vec::with_capacity(score.hccs.len());
    for (h, (diagnoses, init_judgment)) in score.hccs.iter().zip(initial.into_iter()) {
        let (final_judgment, corrected, correction_reason) = match by_hcc.get(&h.hcc) {
            Some(dis) => {
                let target = Target { hcc: h.hcc, diagnoses: &diagnoses };
                let (revised, reason) =
                    self_correct::recheck(provider, &rec.note, &target, dis)?;
                let changed = !revised.status.eq_ignore_ascii_case(&init_judgment.status);
                let reason = format!(
                    "verify[{}]: initial '{}' → final '{}'{}",
                    dis.kind.as_str(),
                    init_judgment.status,
                    revised.status,
                    if reason.trim().is_empty() { String::new() } else { format!(" — {}", reason.trim()) },
                );
                (revised, changed, reason)
            }
            None => (init_judgment.clone(), false, String::new()),
        };
        hccs.push(VerifiedHccAudit {
            hcc: h.hcc,
            coefficient: h.coefficient,
            diagnoses,
            initial: init_judgment,
            final_judgment,
            corrected,
            correction_reason,
        });
    }

    let extraction_implied_untriggered =
        verify::extraction_implied_untriggered(&extracted, &score.hccs, &rec.note, xwalk);

    Ok(VerifiedPatientAudit {
        patient_id: rec.id.clone(),
        name: rec.name.clone(),
        age: score.age,
        segment: segment_label(score.segment),
        raw_score: score.raw_score,
        model: score.model,
        extracted,
        hccs,
        extraction_implied_untriggered,
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

/// Render a human-readable report for a verified (verify → self-correct) audit,
/// highlighting which HCCs the loop re-examined and how their status changed.
pub fn render_verified_report(a: &VerifiedPatientAudit) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "Patient {} ({}), age {} — {} [{}] risk score {:.3}",
        a.patient_id, a.name, a.age, a.model, a.segment, a.raw_score
    );
    let corrected = a.corrected().count();
    let _ = writeln!(
        s,
        "Engine-triggered HCCs: {} | self-corrected: {}",
        a.hccs.len(),
        corrected
    );
    if !a.extraction_implied_untriggered.is_empty() {
        let _ = writeln!(
            s,
            "Note: extraction implied HCC(s) the engine did NOT trigger (engine is \
             authoritative; not added): {:?}",
            a.extraction_implied_untriggered
        );
    }
    let _ = writeln!(s);

    for h in &a.hccs {
        let mark = if h.final_is_supported() { "OK  " } else { "FLAG" };
        let dx = h
            .diagnoses
            .iter()
            .map(|(c, d)| if d.is_empty() { c.clone() } else { format!("{c} {d}") })
            .collect::<Vec<_>>()
            .join("; ");
        let _ = writeln!(s, "[{}] HCC {} (coef {:.3}) — {}", mark, h.hcc, h.coefficient, dx);
        if h.corrected {
            let _ = writeln!(
                s,
                "      status: {} (CORRECTED from {})",
                h.final_judgment.status, h.initial.status
            );
            let _ = writeln!(s, "      correction: {}", h.correction_reason);
        } else {
            let _ = writeln!(s, "      status: {}", h.final_judgment.status);
        }
        if !h.final_judgment.meat_present.is_empty() {
            let _ = writeln!(s, "      M.E.A.T.: {}", h.final_judgment.meat_present.join(", "));
        }
        if h.final_judgment.is_flagged() && !h.final_judgment.documentation_gap.is_empty() {
            let _ = writeln!(s, "      gap: {}", h.final_judgment.documentation_gap);
        }
        if !h.final_judgment.rationale.is_empty() {
            let _ = writeln!(s, "      rationale: {}", h.final_judgment.rationale);
        }
    }
    s
}
