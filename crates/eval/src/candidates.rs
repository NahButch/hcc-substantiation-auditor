//! Mechanical truth: the engine-eligible candidate HCCs per patient.
//!
//! Unlike the Python harness — which *replicated* the engine's diagnosis→HCC +
//! hierarchy logic — this calls the real [`engine`] directly via the agent's
//! ground-truth path. The engine is the single source of truth, so no
//! reconciliation step is needed.

use crate::schema::Candidate;
use agent::crosswalk::Crosswalk;
use agent::{fhir, groundtruth};
use engine::{Engine, ModelVersion};
use std::collections::BTreeMap;
use std::path::Path;

/// Derive the candidate set from a directory of Synthea FHIR bundles.
pub fn derive<V: ModelVersion>(
    engine: &Engine<V>,
    xwalk: &Crosswalk,
    fhir_dir: impl AsRef<Path>,
) -> std::io::Result<Vec<Candidate>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(fhir_dir)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !name.ends_with(".json")
            || name.contains("hospitalInformation")
            || name.contains("practitionerInformation")
        {
            continue;
        }
        let Some(rec) = fhir::parse_bundle(&path) else { continue };

        // icd10 (normalized) → contributing SNOMED codes, for informational output.
        let mut icd_to_snomed: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for cond in &rec.conditions {
            if let Some(e) = xwalk.icd10(&cond.snomed) {
                icd_to_snomed
                    .entry(engine::types::normalize_icd10(&e.icd10))
                    .or_default()
                    .push(cond.snomed.clone());
            }
        }

        let score = groundtruth::ground_truth(engine, &rec, xwalk);
        for h in &score.hccs {
            let mut snomed: Vec<String> = h
                .triggering_diagnoses
                .iter()
                .filter_map(|icd| icd_to_snomed.get(icd))
                .flatten()
                .cloned()
                .collect();
            snomed.sort();
            snomed.dedup();
            out.push(Candidate {
                patient_id: rec.id.clone(),
                hcc: h.hcc,
                triggering_icd10: h.triggering_diagnoses.clone(),
                triggering_snomed: snomed,
            });
        }
    }
    out.sort_by(|a, b| a.patient_id.cmp(&b.patient_id).then(a.hcc.cmp(&b.hcc)));
    Ok(out)
}
