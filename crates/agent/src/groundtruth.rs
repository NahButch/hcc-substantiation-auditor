//! Build the deterministic "coded" ground truth: translate a patient's
//! SNOMED-coded conditions to ICD-10 via the crosswalk, run them through the
//! engine oracle, and return the HCCs that would be billed.

use crate::crosswalk::Crosswalk;
use crate::fhir::PatientRecord;
use engine::{Diagnosis, DualStatus, Engine, ModelVersion, PersonInput, ScoreResult};

/// Convert a patient record into the engine's PERSON+DIAG input using the
/// crosswalk. Conditions with no crosswalk entry are dropped (out of scope).
pub fn to_person_input(rec: &PatientRecord, xwalk: &Crosswalk) -> PersonInput {
    let mut diagnoses = Vec::new();
    for c in &rec.conditions {
        if let Some(entry) = xwalk.icd10(&c.snomed) {
            diagnoses.push(Diagnosis {
                icd10: engine::types::normalize_icd10(&entry.icd10),
                source_id: Some(c.snomed.clone()),
            });
        }
    }
    PersonInput {
        person_id: rec.id.clone(),
        date_of_birth: rec.date_of_birth,
        sex: rec.sex,
        // Synthea does not provide OREC / dual status; default to a community
        // non-dual continuing enrollee (the modeled v1 default segment).
        orec: 0,
        long_term_medicaid: false,
        dual_status: DualStatus::NonDual,
        diagnoses,
    }
}

/// Score the patient with the engine → the coded ground-truth HCC profile.
pub fn ground_truth<V: ModelVersion>(
    engine: &Engine<V>,
    rec: &PatientRecord,
    xwalk: &Crosswalk,
) -> ScoreResult {
    engine.score(&to_person_input(rec, xwalk))
}
