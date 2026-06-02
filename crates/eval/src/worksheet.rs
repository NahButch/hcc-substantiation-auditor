//! Human labeling worksheet (ports `hcceval/worksheet.py`): a CSV a reviewer fills
//! to produce gold labels, one row per (patient, engine-eligible HCC), anchored to
//! CMS V28 M.E.A.T. expectations. The filled sheet converts back to gold JSONL.

use crate::schema::{Candidate, GoldLabel};
use agent::crosswalk::Crosswalk;
use std::path::Path;

pub const COLUMNS: &[&str] = &[
    "patient_id", "hcc", "triggering_icd10", "triggering_snomed", "hcc_description",
    "meat_monitor", "meat_evaluate", "meat_assess", "meat_treat", "specificity_ok",
    "gold_status", "holdout", "rationale",
];

const INSTRUCTIONS: &str = "Fill one row per coded HCC. Read the patient note. Mark each M.E.A.T. anchor Y/N (Monitor, Evaluate, Assess, Treat) and whether documentation supports the coded specificity. Set gold_status to 'supported' only if the note documents the condition to CMS V28 standards (>=1 M.E.A.T. element clearly tied to the coded condition in the service period); otherwise 'unsupported'. Set holdout=TRUE for the reserved slice. Give a rationale for every 'unsupported' row.";

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Write a blank worksheet (one row per candidate). Returns the row count.
pub fn build_worksheet(
    candidates: &[Candidate],
    xwalk: &Crosswalk,
    path: impl AsRef<Path>,
) -> std::io::Result<usize> {
    let mut rows: Vec<&Candidate> = candidates.iter().collect();
    rows.sort_by(|a, b| a.patient_id.cmp(&b.patient_id).then(a.hcc.cmp(&b.hcc)));
    let mut out = String::new();
    out.push_str(&format!("# {INSTRUCTIONS}\n"));
    out.push_str(&COLUMNS.join(","));
    out.push('\n');
    for c in &rows {
        let desc = c
            .triggering_icd10
            .iter()
            .find_map(|icd| xwalk.describe_icd10(&engine::types::normalize_icd10(icd)))
            .unwrap_or("");
        let line = [
            c.patient_id.clone(),
            c.hcc.to_string(),
            c.triggering_icd10.join(";"),
            c.triggering_snomed.join(";"),
            desc.to_string(),
            String::new(), String::new(), String::new(), String::new(), String::new(),
            String::new(), String::new(), String::new(),
        ];
        out.push_str(&line.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(","));
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(rows.len())
}

fn truthy(v: &str) -> bool {
    matches!(v.trim().to_lowercase().as_str(), "y" | "yes" | "true" | "1" | "t")
}

/// Parse a filled worksheet back into gold labels (rows with a gold_status set).
pub fn worksheet_to_gold(path: impl AsRef<Path>) -> std::io::Result<Vec<GoldLabel>> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines().filter(|l| !l.trim_start().starts_with('#'));
    let header: Vec<String> = lines.next().map(parse_csv_line).unwrap_or_default();
    let col = |name: &str| header.iter().position(|h| h == name);
    let (Some(pid_i), Some(hcc_i), Some(gs_i)) =
        (col("patient_id"), col("hcc"), col("gold_status"))
    else {
        return Ok(Vec::new());
    };
    let (rat_i, hold_i) = (col("rationale"), col("holdout"));
    let mut out = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let f = parse_csv_line(line);
        let status = f.get(gs_i).map(|s| s.trim().to_lowercase()).unwrap_or_default();
        if status.is_empty() {
            continue;
        }
        out.push(GoldLabel {
            patient_id: f.get(pid_i).map(|s| s.trim().to_string()).unwrap_or_default(),
            hcc: f.get(hcc_i).and_then(|s| s.trim().parse().ok()).unwrap_or(0),
            gold_status: status,
            source: "manual".to_string(),
            rationale: rat_i.and_then(|i| f.get(i)).map(|s| s.trim().to_string()).unwrap_or_default(),
            holdout: hold_i.and_then(|i| f.get(i)).map(|s| truthy(s)).unwrap_or(false),
        });
    }
    Ok(out)
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut q = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if q && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => q = !q,
            ',' if !q => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}
