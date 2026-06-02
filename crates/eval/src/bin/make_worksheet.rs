//! Scaffold a labeling worksheet from candidates, or convert a filled one to gold.
//!
//! Usage:
//!   make_worksheet --candidates C.jsonl --out worksheet.csv [--crosswalk CSV]
//!   make_worksheet --to-gold worksheet.csv --out gold.jsonl

use agent::crosswalk::Crosswalk;
use eval::{schema, worksheet};

fn arg(a: &[String], k: &str) -> Option<String> {
    a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).cloned()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = arg(&a, "--out").unwrap_or_else(|| {
        eprintln!("usage: make_worksheet --candidates C.jsonl --out worksheet.csv | --to-gold worksheet.csv --out gold.jsonl");
        std::process::exit(2);
    });

    if let Some(sheet) = arg(&a, "--to-gold") {
        let gold = worksheet::worksheet_to_gold(&sheet).expect("parse worksheet");
        let n = schema::write_jsonl(&out, &gold).expect("write gold");
        println!("converted {n} labeled rows -> {out}");
    } else if let Some(cand_p) = arg(&a, "--candidates") {
        let candidates = schema::load_candidates(&cand_p).expect("load candidates");
        let xwalk_p = arg(&a, "--crosswalk").unwrap_or_else(|| "crosswalks/snomed_to_icd10_v28.csv".into());
        let xwalk = Crosswalk::load(&xwalk_p).expect("load crosswalk");
        let n = worksheet::build_worksheet(&candidates, &xwalk, &out).expect("write worksheet");
        println!("wrote worksheet with {n} rows -> {out}");
    } else {
        eprintln!("need --candidates (build) or --to-gold (convert)");
        std::process::exit(2);
    }
}
