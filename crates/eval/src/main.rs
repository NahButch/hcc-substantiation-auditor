//! `hcceval` — compute the evaluation metrics from an agent audit + gold labels.
//!
//! Usage:
//!   hcceval --audit A.jsonl --gold G.jsonl
//!           [--candidates C.jsonl | --fhir DIR] [--tables DIR] [--crosswalk CSV]
//!           [--fhir DIR] [--exclude-holdout] [--json OUT] [--md OUT]
//!
//! Candidates (extraction metrics) come from --candidates, or are derived from the
//! engine over --fhir when --tables + --crosswalk are given. Notes (span metrics)
//! come from --fhir.

use agent::crosswalk::Crosswalk;
use engine::Engine;
use eval::{candidates, metrics, notes, report, schema};
use std::collections::HashMap;

fn arg(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).cloned()
}
fn flag(args: &[String], key: &str) -> bool {
    args.iter().any(|a| a == key)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(audit_p), Some(gold_p)) = (arg(&args, "--audit"), arg(&args, "--gold")) else {
        eprintln!("usage: hcceval --audit A.jsonl --gold G.jsonl [--candidates C | --fhir DIR] [--tables DIR] [--crosswalk CSV] [--exclude-holdout] [--json OUT] [--md OUT]");
        std::process::exit(2);
    };
    let include_holdout = !flag(&args, "--exclude-holdout");

    let audits = schema::load_audits(&audit_p).expect("load audits");
    let gold = schema::load_gold(&gold_p).expect("load gold");

    // Crosswalk + engine enable extraction metrics and engine-derived candidates.
    let xwalk_p = arg(&args, "--crosswalk").unwrap_or_else(|| "crosswalks/snomed_to_icd10_v28.csv".into());
    let xwalk = Crosswalk::load(&xwalk_p).ok();
    let engine = arg(&args, "--tables").and_then(|t| Engine::load_v28(t).ok());

    // Candidates: explicit file, else derive from --fhir via the engine.
    let candidates: Vec<schema::Candidate> = if let Some(c) = arg(&args, "--candidates") {
        schema::load_candidates(c).expect("load candidates")
    } else if let (Some(eng), Some(xw), Some(dir)) = (&engine, &xwalk, arg(&args, "--fhir")) {
        candidates::derive(eng, xw, dir).expect("derive candidates")
    } else {
        Vec::new()
    };

    let note_map: HashMap<String, String> =
        arg(&args, "--fhir").map(|d| notes::load_notes(d).unwrap_or_default()).unwrap_or_default();

    // Fall back to a no-op engine/crosswalk for compute_all's signature when absent.
    let xwalk = xwalk.unwrap_or_default();
    let m = match &engine {
        Some(eng) => metrics::compute_all(&audits, &gold, &candidates, &note_map, &xwalk, eng, include_holdout),
        None => {
            // No engine → skip extraction (needs collapse); build with an empty engine-less path.
            let mut m = metrics::Metrics {
                counts: metrics::Counts {
                    patients_audited: audits.len(),
                    audited_hccs: audits.iter().map(|a| a.hccs.len()).sum(),
                    gold_labels: gold.len(),
                    gold_holdout: gold.iter().filter(|g| g.holdout).count(),
                    gold_injected: gold.iter().filter(|g| g.source == "injected").count(),
                    candidates: candidates.len(),
                    notes_available: note_map.len(),
                },
                substantiation: metrics::substantiation_metrics(&audits, &gold, include_holdout),
                system: metrics::system_metrics(&audits, &gold, include_holdout),
                extraction: None,
                spans: (!note_map.is_empty())
                    .then(|| metrics::span_and_hallucination_metrics(&audits, &note_map)),
                calibration: metrics::calibration_metrics(&audits, &gold, 10, include_holdout),
            };
            if engine.is_none() && !candidates.is_empty() {
                eprintln!("note: --tables not given; extraction metrics skipped (need engine for hierarchy collapse)");
            }
            m.extraction = None;
            m
        }
    };

    println!("{}", report::render_text(&m));
    if let Some(p) = arg(&args, "--json") {
        std::fs::write(&p, report::render_json(&m)).expect("write json");
        eprintln!("wrote {p}");
    }
    if let Some(p) = arg(&args, "--md") {
        std::fs::write(&p, report::render_json(&m)).expect("write md"); // md summary == json for now
        eprintln!("wrote {p}");
    }
}
