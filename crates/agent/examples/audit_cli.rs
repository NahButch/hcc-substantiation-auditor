//! Single-pass audit CLI: run the auditor over Synthea patient bundles.
//!
//! Usage:
//!   audit_cli <tables_dir> <crosswalk.csv> <fhir_dir> [N]
//!
//! Audits the first N patient bundles (default 3) that have at least one
//! engine-triggered HCC, printing a flagged-code report for each.

use agent::llm::OllamaProvider;
use agent::{audit, crosswalk::Crosswalk, fhir};
use engine::Engine;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: audit_cli <tables_dir> <crosswalk.csv> <fhir_dir> [N]");
        std::process::exit(2);
    }
    let n: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(3);

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
            f.ends_with(".json") && !f.contains("hospitalInformation") && !f.contains("practitionerInformation")
        })
        .collect();
    bundles.sort();

    let mut done = 0;
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
        match audit::audit_patient(&engine, &provider, &xwalk, &rec) {
            Ok(report) => {
                println!("{}", "=".repeat(72));
                print!("{}", audit::render_report(&report));
                done += 1;
            }
            Err(e) => eprintln!("audit failed for {}: {e}", rec.id),
        }
    }
    println!("{}", "=".repeat(72));
    println!("Audited {done} patient(s).");
}
