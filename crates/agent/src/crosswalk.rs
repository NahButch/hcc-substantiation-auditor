//! SNOMED CT → ICD-10-CM crosswalk for the target families (see
//! `crosswalks/snomed_to_icd10_v28.csv`). Used to turn Synthea's SNOMED-coded
//! conditions into the ICD-10 the engine consumes.

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Entry {
    pub icd10: String,
    pub icd10_description: String,
}

#[derive(Debug, Default)]
pub struct Crosswalk {
    by_snomed: HashMap<String, Entry>,
}

impl Crosswalk {
    /// Load from the committed crosswalk CSV.
    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let mut by_snomed = HashMap::new();
        let mut lines = text.lines();
        lines.next(); // header
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let f = parse_csv_line(line);
            // snomed_code,snomed_description,icd10,icd10_description,expected_v28_hcc,note
            if f.len() < 4 {
                continue;
            }
            let snomed = f[0].trim().to_string();
            let icd10 = f[2].trim().to_string();
            if snomed.is_empty() || icd10.is_empty() {
                continue;
            }
            by_snomed.insert(
                snomed,
                Entry { icd10, icd10_description: f[3].trim().to_string() },
            );
        }
        Ok(Crosswalk { by_snomed })
    }

    /// ICD-10 for a SNOMED code, if mapped.
    pub fn icd10(&self, snomed: &str) -> Option<&Entry> {
        self.by_snomed.get(snomed)
    }

    /// Description for an ICD-10 code (normalized, dotless), if known.
    pub fn describe_icd10(&self, icd10_dotless: &str) -> Option<&str> {
        self.by_snomed
            .values()
            .find(|e| e.icd10.replace('.', "").eq_ignore_ascii_case(icd10_dotless))
            .map(|e| e.icd10_description.as_str())
    }
}

/// Minimal CSV line parser honoring double-quoted fields (the description column
/// may contain commas, e.g. "Heart failure, unspecified").
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
