//! Score beneficiaries from CMS-format CSVs and print results — used by the
//! Python cross-check harness to compare the engine against the CMS reference.
//!
//! Usage: score_csv <tables_dir> <beneficiaries.csv> <diagnoses.csv>
//! Output (stdout, CSV): ID,SEGMENT_COLUMN,SCORE
//!
//! Beneficiaries CSV columns: ID,DOB,SEX,OREC,LTIMCAID,NEMCAID  (DOB = M/D/YYYY)
//! Diagnoses CSV columns: ID,ICD10

use engine::demographics::Date;
use engine::{Diagnosis, DualStatus, Engine, PersonInput, Sex};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: score_csv <tables_dir> <beneficiaries.csv> <diagnoses.csv>");
        std::process::exit(2);
    }
    let engine = Engine::load_v28(&args[1]).expect("load tables");

    // Group diagnoses by beneficiary ID.
    let diag_csv = engine::csv::read(&args[3]).expect("read diagnoses");
    let (did, dicd) = (diag_csv.col("ID").unwrap(), diag_csv.col("ICD10").unwrap());
    let mut diags: BTreeMap<String, Vec<Diagnosis>> = BTreeMap::new();
    for row in &diag_csv.rows {
        if let (Some(id), Some(icd)) = (row.get(did), row.get(dicd)) {
            if !icd.trim().is_empty() {
                diags.entry(id.trim().to_string()).or_default().push(Diagnosis::new(icd.trim()));
            }
        }
    }

    let ben_csv = engine::csv::read(&args[2]).expect("read beneficiaries");
    let col = |n: &str| ben_csv.col(n).unwrap_or_else(|| panic!("missing column {n}"));
    let (cid, cdob, csex, corec, cltimcaid) =
        (col("ID"), col("DOB"), col("SEX"), col("OREC"), col("LTIMCAID"));

    println!("ID,SEGMENT,SCORE");
    for row in &ben_csv.rows {
        let id = row[cid].trim().to_string();
        let person = PersonInput {
            person_id: id.clone(),
            date_of_birth: parse_dob(row[cdob].trim()),
            sex: if row[csex].trim() == "1" { Sex::Male } else { Sex::Female },
            orec: row[corec].trim().parse().unwrap_or(0),
            long_term_medicaid: row[cltimcaid].trim() == "1",
            dual_status: DualStatus::NonDual,
            diagnoses: diags.get(&id).cloned().unwrap_or_default(),
        };
        let r = engine.score(&person);
        println!("{},{},{:.3}", id, r.segment.column(), r.raw_score);
    }
}

/// Parse a `M/D/YYYY` date (CMS `%m/%d/%Y`, non-padded accepted).
fn parse_dob(s: &str) -> Date {
    let p: Vec<&str> = s.split('/').collect();
    Date::new(
        p[2].parse().expect("year"),
        p[0].parse().expect("month"),
        p[1].parse().expect("day"),
    )
}
