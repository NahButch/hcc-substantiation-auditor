//! Phase 3a audit CLI → shared `audit_results.jsonl` contract (one JSON object per
//! patient) consumed by the harness.
//!
//! Usage:
//!   audit_jsonl <tables_dir> <crosswalk.csv> <fhir_dir> <out.jsonl> [N]
//!
//! Runs the verify → self-correct loop over the first N patient bundles (default 5)
//! that trigger at least one engine HCC, writing one contract line per patient and
//! printing a short per-patient report (with any self-corrections) to stderr.

use agent::audit::{self, VerifiedPatientAudit};
use agent::llm::OllamaProvider;
use agent::{crosswalk::Crosswalk, fhir};
use engine::Engine;
use std::io::Write as _;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("usage: audit_jsonl <tables_dir> <crosswalk.csv> <fhir_dir> <out.jsonl> [N]");
        std::process::exit(2);
    }
    let n: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(5);

    let engine = Engine::load_v28(&args[1]).expect("load V28 tables");
    let xwalk = Crosswalk::load(&args[2]).expect("load crosswalk");
    let model = std::env::var("HCC_MODEL").unwrap_or_else(|_| "qwen2.5:7b-instruct".to_string());
    let provider = OllamaProvider::new(model);

    let mut bundles: Vec<_> = std::fs::read_dir(&args[3])
        .expect("read fhir dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let f = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            f.ends_with(".json")
                && !f.contains("hospitalInformation")
                && !f.contains("practitionerInformation")
        })
        .collect();
    bundles.sort();

    let out_path = &args[4];
    if let Some(parent) = std::path::Path::new(out_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut out = std::fs::File::create(out_path).expect("create out.jsonl");

    let mut done = 0;
    let mut total_corrections = 0;
    for path in &bundles {
        if done >= n {
            break;
        }
        let Some(rec) = fhir::parse_bundle(path) else { continue };
        // Only audit patients who actually trigger an HCC (otherwise nothing to review).
        let gt = agent::groundtruth::ground_truth(&engine, &rec, &xwalk);
        if gt.hccs.is_empty() {
            continue;
        }
        match audit::audit_patient_verified(&engine, &provider, &xwalk, &rec) {
            Ok(a) => {
                let line = to_contract_json(&a);
                writeln!(out, "{}", serde_json::to_string(&line).unwrap()).expect("write line");
                total_corrections += a.corrected().count();
                eprintln!("{}", "=".repeat(72));
                eprint!("{}", audit::render_verified_report(&a));
                done += 1;
            }
            Err(e) => eprintln!("audit failed for {}: {e}", rec.id),
        }
    }
    eprintln!("{}", "=".repeat(72));
    eprintln!(
        "Wrote {done} patient(s) to {out_path} — {total_corrections} self-correction(s) total."
    );
}

/// Map a verified audit to the exact shared output contract.
fn to_contract_json(a: &VerifiedPatientAudit) -> serde_json::Value {
    let extracted: Vec<_> = a
        .extracted
        .iter()
        .map(|c| {
            serde_json::json!({
                "name": c.name,
                "evidence": c.evidence,
                "section": c.section,
                "icd10": c.icd10,
            })
        })
        .collect();

    let hccs: Vec<_> = a
        .hccs
        .iter()
        .map(|h| {
            let f = &h.final_judgment;
            serde_json::json!({
                "hcc": h.hcc,
                "coefficient": h.coefficient,
                "triggering_diagnoses": h.diagnoses.iter().map(|(c, _)| c.clone()).collect::<Vec<_>>(),
                "initial_status": h.initial.status,
                "final_status": f.status,
                "corrected": h.corrected,
                "correction_reason": h.correction_reason,
                "meat_present": f.meat_present,
                "specificity_supported": f.specificity_supported,
                "citation": f.citation,
                "documentation_gap": f.documentation_gap,
                "confidence": f.confidence,
            })
        })
        .collect();

    serde_json::json!({
        "patient_id": a.patient_id,
        "age": a.age,
        "segment": a.segment,
        "raw_score": a.raw_score,
        "extracted": extracted,
        "hccs": hccs,
    })
}
