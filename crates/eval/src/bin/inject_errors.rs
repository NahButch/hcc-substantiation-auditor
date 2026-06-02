//! Generate the controlled known-truth injection set: FHIR bundles + candidates + gold.
//!
//! Usage: inject_errors --fhir-out DIR --candidates-out C.jsonl --gold-out G.jsonl [--holdout-every N]

use eval::{inject, schema};

fn arg(a: &[String], k: &str) -> Option<String> {
    a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).cloned()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (Some(fhir_out), Some(cand_out), Some(gold_out)) =
        (arg(&a, "--fhir-out"), arg(&a, "--candidates-out"), arg(&a, "--gold-out"))
    else {
        eprintln!("usage: inject_errors --fhir-out DIR --candidates-out C.jsonl --gold-out G.jsonl [--holdout-every N]");
        std::process::exit(2);
    };
    let holdout_every: usize = arg(&a, "--holdout-every").and_then(|s| s.parse().ok()).unwrap_or(2);

    let set = inject::build_injection_set(inject::DEFAULT_SPECS, holdout_every);
    let nb = inject::write_bundles(&set, &fhir_out).expect("write bundles");
    let nc = schema::write_jsonl(&cand_out, &set.candidates).expect("write candidates");
    let ng = schema::write_jsonl(&gold_out, &set.gold).expect("write gold");
    let holdout = set.gold.iter().filter(|g| g.holdout).count();
    let sup = set.gold.iter().filter(|g| g.supported()).count();
    println!("wrote {nb} FHIR bundles -> {fhir_out}");
    println!("wrote {nc} candidates -> {cand_out}");
    println!("wrote {ng} gold labels -> {gold_out} (supported {sup}, unsupported {}, holdout {holdout})", ng - sup);
}
