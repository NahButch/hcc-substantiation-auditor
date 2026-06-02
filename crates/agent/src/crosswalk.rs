//! SNOMED CT → ICD-10-CM crosswalk for the target families (see
//! `crosswalks/snomed_to_icd10_v28.csv`). Used to turn Synthea's SNOMED-coded
//! conditions into the ICD-10 the engine consumes.

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Entry {
    pub icd10: String,
    pub icd10_description: String,
    /// The V28 HCC this code is expected to map to, if any (the crosswalk's
    /// `expected_v28_hcc` column; blank for near-miss / non-eligible codes).
    pub expected_hcc: Option<u32>,
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
            let expected_hcc = f.get(4).and_then(|s| s.trim().parse::<u32>().ok());
            by_snomed.insert(
                snomed,
                Entry { icd10, icd10_description: f[3].trim().to_string(), expected_hcc },
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

    /// Expected V28 HCC for an ICD-10 code (normalized, dotless), if the crosswalk
    /// maps it to one. Used to test whether an extracted condition *implies* a given
    /// HCC during verification.
    pub fn hcc_for_icd10(&self, icd10_dotless: &str) -> Option<u32> {
        let norm = icd10_dotless.trim().to_uppercase().replace('.', "");
        self.by_snomed
            .values()
            .find(|e| e.icd10.replace('.', "").eq_ignore_ascii_case(&norm))
            .and_then(|e| e.expected_hcc)
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
