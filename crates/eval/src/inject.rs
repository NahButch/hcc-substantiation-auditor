//! Controlled error injection (ports `hcceval/inject.py`): known-truth
//! substantiation cases. For each target family, two matched patients share the
//! same coded HCC — one whose note carries full M.E.A.T. (gold "supported") and
//! one where the code is present but the note documents an unrelated visit (gold
//! "unsupported", a textbook over-code). Each is a minimal Synthea-style FHIR
//! bundle the real agent can audit, plus a candidate and a gold label whose truth
//! is known by construction.

use crate::schema::{Candidate, GoldLabel};
use base64::Engine as _;
use std::path::Path;

pub struct InjectionSpec {
    pub family: &'static str,
    pub hcc: u32,
    pub icd10: &'static str,
    pub snomed: &'static str,
    pub display: &'static str,
    pub supported_note: &'static str,
    pub unsupported_note: &'static str,
}

pub const DEFAULT_SPECS: &[InjectionSpec] = &[
    InjectionSpec {
        family: "diabetes", hcc: 37, icd10: "E1122", snomed: "127013003",
        display: "Type 2 diabetes mellitus with diabetic chronic kidney disease",
        supported_note: "# Assessment and Plan\n1. Type 2 diabetes mellitus with diabetic chronic kidney disease.\n   - Monitor: HbA1c 8.1% today, up from 7.6%. Reviewing home glucose logs.\n   - Evaluate: microalbuminuria stable; eGFR 52.\n   - Assess: suboptimal glycemic control with established diabetic CKD.\n   - Treat: continue metformin, increase insulin glargine to 24 units nightly; referral to nephrology.\n",
        unsupported_note: "# Chief Complaint\n- Ankle sprain.\n\n# History of Present Illness\nPatient twisted the right ankle stepping off a curb.\n\n# Assessment and Plan\n1. Right ankle sprain. RICE, ibuprofen PRN, follow up if no improvement in two weeks.\n",
    },
    InjectionSpec {
        family: "ckd", hcc: 327, icd10: "N184", snomed: "431857002",
        display: "Chronic kidney disease stage 4 (severe)",
        supported_note: "# Assessment and Plan\n1. Chronic kidney disease, stage 4.\n   - Monitor: eGFR 22, creatinine 2.9 (prior 2.6). Potassium 5.1.\n   - Evaluate: renal ultrasound reviewed; no obstruction.\n   - Assess: progressive stage 4 CKD, nearing transplant evaluation.\n   - Treat: started on sodium bicarbonate; dietary protein restriction counseled; nephrology follow-up in 4 weeks.\n",
        unsupported_note: "# Chief Complaint\n- Annual flu vaccination.\n\n# Assessment and Plan\n1. Influenza immunization administered. No acute concerns today. Patient feels well.\n",
    },
    InjectionSpec {
        family: "heart_failure", hcc: 226, icd10: "I509", snomed: "88805009",
        display: "Chronic congestive heart failure",
        supported_note: "# Assessment and Plan\n1. Chronic congestive heart failure (HFrEF, EF 35%).\n   - Monitor: daily weights stable, no orthopnea; BNP 540.\n   - Evaluate: echocardiogram reviewed, EF unchanged.\n   - Assess: compensated chronic systolic heart failure.\n   - Treat: continue carvedilol and furosemide; up-titrate lisinopril; low-sodium diet reinforced.\n",
        unsupported_note: "# Chief Complaint\n- Seasonal allergies.\n\n# Assessment and Plan\n1. Allergic rhinitis. Start loratadine daily; nasal saline rinses. No cardiopulmonary complaints.\n",
    },
    InjectionSpec {
        family: "copd", hcc: 280, icd10: "J439", snomed: "87433001",
        display: "Pulmonary emphysema",
        supported_note: "# Assessment and Plan\n1. COPD / pulmonary emphysema.\n   - Monitor: dyspnea on exertion stable; SpO2 93% on room air.\n   - Evaluate: spirometry FEV1 48% predicted; reviewed today.\n   - Assess: moderate-to-severe COPD, emphysema-predominant.\n   - Treat: continue tiotropium and albuterol; pulmonary rehab referral; smoking-cessation counseling provided.\n",
        unsupported_note: "# Chief Complaint\n- Wrist laceration.\n\n# Assessment and Plan\n1. Superficial laceration, left wrist. Cleaned, two sutures placed, tetanus up to date. Return for suture removal in 10 days.\n",
    },
];

const DOB: &str = "1950-01-01";
const SEX: &str = "male";

fn fhir_bundle(pid: &str, snomed: &str, display: &str, note: &str) -> serde_json::Value {
    let data = base64::engine::general_purpose::STANDARD.encode(note.as_bytes());
    serde_json::json!({
        "resourceType": "Bundle", "type": "collection",
        "entry": [
            {"resource": {"resourceType": "Patient", "id": pid, "birthDate": DOB, "gender": SEX,
                "name": [{"given": ["Synthetic"], "family": pid}]}},
            {"resource": {"resourceType": "Condition", "code": {"coding": [
                {"system": "http://snomed.info/sct", "code": snomed, "display": display}]}}},
            {"resource": {"resourceType": "DocumentReference", "date": "2026-01-15T09:00:00-05:00",
                "content": [{"attachment": {"contentType": "text/plain; charset=utf-8", "data": data}}]}},
        ],
    })
}

pub struct InjectionSet {
    pub candidates: Vec<Candidate>,
    pub gold: Vec<GoldLabel>,
    pub bundles: Vec<(String, serde_json::Value)>,
}

/// Build matched supported/unsupported cases for every spec. `holdout_every`
/// flags every Nth gold label as held-out (default 2). Deterministic patient IDs.
pub fn build_injection_set(specs: &[InjectionSpec], holdout_every: usize) -> InjectionSet {
    let mut set = InjectionSet { candidates: Vec::new(), gold: Vec::new(), bundles: Vec::new() };
    let mut idx = 0usize;
    for spec in specs {
        for status in ["supported", "unsupported"] {
            let pid = format!("inj-{}-{}", spec.family, status);
            let note = if status == "supported" { spec.supported_note } else { spec.unsupported_note };
            set.bundles.push((pid.clone(), fhir_bundle(&pid, spec.snomed, spec.display, note)));
            set.candidates.push(Candidate {
                patient_id: pid.clone(),
                hcc: spec.hcc,
                triggering_icd10: vec![spec.icd10.to_string()],
                triggering_snomed: vec![spec.snomed.to_string()],
            });
            let rationale = if status == "supported" {
                format!("Constructed supported case for {} HCC {}: note documents full M.E.A.T.", spec.family, spec.hcc)
            } else {
                format!("Constructed unsupported case for {} HCC {}: code present but note documents an unrelated visit (over-code).", spec.family, spec.hcc)
            };
            set.gold.push(GoldLabel {
                patient_id: pid,
                hcc: spec.hcc,
                gold_status: status.to_string(),
                source: "injected".to_string(),
                rationale,
                holdout: holdout_every > 0 && idx % holdout_every == 1,
            });
            idx += 1;
        }
    }
    set
}

/// Write each synthetic bundle to `fhir_dir` as `<patient_id>.json`.
pub fn write_bundles(set: &InjectionSet, fhir_dir: impl AsRef<Path>) -> std::io::Result<usize> {
    let dir = fhir_dir.as_ref();
    std::fs::create_dir_all(dir)?;
    for (pid, bundle) in &set.bundles {
        std::fs::write(dir.join(format!("{pid}.json")), serde_json::to_string_pretty(bundle).unwrap())?;
    }
    Ok(set.bundles.len())
}
