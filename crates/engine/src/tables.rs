//! Loading and representation of the CMS V28 reference tables.
//!
//! Tables are parsed verbatim from the CMS-published CSVs (or a curated subset in
//! the same format). Nothing here is hand-authored model data — coefficients,
//! mappings, hierarchies, and interactions all come from the official files.

use crate::csv;
use crate::version::ModelVersion;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;

/// One ICD-10 → CC mapping entry, with optional MCE / age / sex edit conditions.
#[derive(Debug, Clone)]
pub struct MappingRow {
    pub cc: u32,
    pub mce_age: Option<String>,
    pub age_edit: Option<String>,
    pub sex_edit: Option<u8>,
}

/// Parsed V28 risk-adjustment tables, pinned to a model version `V`.
#[derive(Debug)]
pub struct RiskTables<V: ModelVersion> {
    /// ICD-10 (normalized) → one or more CC mappings.
    mappings: HashMap<String, Vec<MappingRow>>,
    /// HCC → the (less-severe) HCCs it trumps in the hierarchy.
    hierarchies: Vec<(u32, Vec<u32>)>,
    /// Disease category → member HCCs (drives interaction flags).
    categories: Vec<(String, Vec<u32>)>,
    /// Interaction variable name → (component var 1, component var 2).
    interactions: Vec<(String, String, String)>,
    /// Coefficient lookup: variable → segment column → coefficient.
    coef: HashMap<String, HashMap<String, f64>>,
    _v: PhantomData<V>,
}

fn parse_hcc(s: &str) -> Option<u32> {
    let t = s.trim();
    let t = t.strip_prefix("HCC").or_else(|| t.strip_prefix("CC")).unwrap_or(t);
    t.trim().parse::<u32>().ok()
}

fn parse_cc(s: &str) -> Option<u32> {
    s.trim().parse::<f64>().ok().map(|f| f as u32)
}

impl<V: ModelVersion> RiskTables<V> {
    /// Load all tables from a directory using the canonical CMS file names for `V`.
    pub fn load_from_dir(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let dir = dir.as_ref();
        let name = V::NAME; // e.g. "V28"
        let lower = name.to_lowercase();
        let year = V::PAYMENT_YEAR;
        let mappings = dir.join(format!("ICD10_CC_mappings_CMS_HCC_{year}_{lower}.csv"));
        let hierarchies = dir.join(format!("{name}_HCC_Hierarchies.csv"));
        let categories = dir.join(format!("{name}_Diagnosis_Categories.csv"));
        let interactions = dir.join(format!("{name}_Interactions.csv"));
        let ce = dir.join(format!("{name}_CE_Relative_Factors.csv"));
        Self::load(&mappings, &hierarchies, &categories, &interactions, &ce)
    }

    /// Load all tables from explicit file paths.
    pub fn load(
        mappings_path: &Path,
        hierarchies_path: &Path,
        categories_path: &Path,
        interactions_path: &Path,
        ce_factors_path: &Path,
    ) -> std::io::Result<Self> {
        // --- ICD-10 → CC mappings ---
        let m = csv::read(mappings_path)?;
        let (ic, cc, mce, ae, se) = (
            m.col("ICD10").unwrap(),
            m.col("CC").unwrap(),
            m.col("MCE_AGE_CONDITION").unwrap(),
            m.col("AGE_EDIT_CONDITION").unwrap(),
            m.col("SEX_EDIT_CONDITION").unwrap(),
        );
        let mut mappings: HashMap<String, Vec<MappingRow>> = HashMap::new();
        for row in &m.rows {
            let icd10 = crate::types::normalize_icd10(&row[ic]);
            let Some(cc) = parse_cc(&row[cc]) else { continue };
            let opt = |i: usize| {
                row.get(i).map(|s| s.trim()).filter(|s| !s.is_empty()).map(String::from)
            };
            let sex_edit = opt(se).and_then(|s| s.parse::<f64>().ok()).map(|f| f as u8);
            mappings.entry(icd10).or_default().push(MappingRow {
                cc,
                mce_age: opt(mce),
                age_edit: opt(ae),
                sex_edit,
            });
        }

        // --- HCC hierarchies ---
        let h = csv::read(hierarchies_path)?;
        let mut hierarchies = Vec::new();
        for row in &h.rows {
            let Some(hcc) = row.first().and_then(|s| parse_hcc(s)) else { continue };
            let trumped: Vec<u32> = row[1..].iter().filter_map(|s| parse_hcc(s)).collect();
            hierarchies.push((hcc, trumped));
        }

        // --- Diagnosis categories ---
        let c = csv::read(categories_path)?;
        let mut categories = Vec::new();
        for row in &c.rows {
            let Some(name) = row.first().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
            else {
                continue;
            };
            let hccs: Vec<u32> = row[1..].iter().filter_map(|s| parse_hcc(s)).collect();
            categories.push((name, hccs));
        }

        // --- Interactions ---
        let i = csv::read(interactions_path)?;
        let (inm, v1, v2) = (
            i.col("interaction").unwrap(),
            i.col("var_1").unwrap(),
            i.col("var_2").unwrap(),
        );
        let mut interactions = Vec::new();
        for row in &i.rows {
            let name = row[inm].trim().to_string();
            if name.is_empty() {
                continue;
            }
            interactions.push((name, row[v1].trim().to_string(), row[v2].trim().to_string()));
        }

        // --- CE relative factors (coefficients) ---
        let f = csv::read(ce_factors_path)?;
        let var_col = f.col("Variable").unwrap();
        // Segment columns are every header after Variable/Label.
        let seg_cols: Vec<(usize, String)> = f
            .header
            .iter()
            .enumerate()
            .filter(|(_, h)| *h != "Variable" && *h != "Label")
            .map(|(i, h)| (i, h.clone()))
            .collect();
        let mut coef: HashMap<String, HashMap<String, f64>> = HashMap::new();
        for row in &f.rows {
            let Some(var) = row.get(var_col).map(|s| s.trim()).filter(|s| !s.is_empty()) else {
                continue;
            };
            let entry = coef.entry(var.to_string()).or_default();
            for (idx, col) in &seg_cols {
                if let Some(v) = row.get(*idx).and_then(|s| s.trim().parse::<f64>().ok()) {
                    entry.insert(col.clone(), v);
                }
            }
        }

        Ok(RiskTables {
            mappings,
            hierarchies,
            categories,
            interactions,
            coef,
            _v: PhantomData,
        })
    }

    pub fn mappings_for(&self, icd10: &str) -> &[MappingRow] {
        self.mappings.get(icd10).map(|v| v.as_slice()).unwrap_or(&[])
    }
    pub fn hierarchies(&self) -> &[(u32, Vec<u32>)] {
        &self.hierarchies
    }
    pub fn categories(&self) -> &[(String, Vec<u32>)] {
        &self.categories
    }
    pub fn interactions(&self) -> &[(String, String, String)] {
        &self.interactions
    }
    /// Coefficient for `variable` in `segment_col`; `0.0` if blank/absent.
    pub fn coef(&self, variable: &str, segment_col: &str) -> f64 {
        self.coef
            .get(variable)
            .and_then(|m| m.get(segment_col))
            .copied()
            .unwrap_or(0.0)
    }
}

/// Evaluate a CMS age-edit condition string against an age.
///
/// Handles the forms found in the V28 mapping file:
/// `age >= N`, `age < N`, `age = N`, `N <= age <= M` (chained), and `age N+`.
pub fn evaluate_age_rule(expr: &str, age: i32) -> bool {
    let e = expr.trim().to_lowercase();
    let Some(pos) = e.find("age") else { return false };
    let left = e[..pos].trim();
    let right = e[pos + 3..].trim();

    let mut ok = true;
    // Right side: "OP N" (e.g. ">= 17"), or "N+" (e.g. "50+").
    if !right.is_empty() {
        ok &= eval_right(right, age);
    }
    // Left side of a chained comparison: "N OP" (e.g. "0 <="), meaning N OP age.
    if !left.is_empty() {
        ok &= eval_left(left, age);
    }
    ok
}

fn split_op(s: &str) -> Option<(&'static str, &str)> {
    for op in [">=", "<=", "==", ">", "<", "="] {
        if let Some(rest) = s.strip_prefix(op) {
            return Some((op, rest));
        }
    }
    None
}

fn cmp(op: &str, a: i32, b: i32) -> bool {
    match op {
        ">" => a > b,
        ">=" => a >= b,
        "<" => a < b,
        "<=" => a <= b,
        "=" | "==" => a == b,
        _ => false,
    }
}

/// Right side: `"OP N"` → `age OP N`, or `"N+"` → `age >= N`.
fn eval_right(s: &str, age: i32) -> bool {
    let s = s.trim();
    if let Some(num) = s.strip_suffix('+') {
        if let Ok(n) = num.trim().parse::<i32>() {
            return age >= n;
        }
    }
    if let Some((op, rest)) = split_op(s) {
        if let Ok(n) = rest.trim().parse::<i32>() {
            return cmp(op, age, n);
        }
    }
    false
}

/// Left side: `"N OP"` → `N OP age`.
fn eval_left(s: &str, age: i32) -> bool {
    let s = s.trim();
    // Find where the operator starts.
    if let Some(op_pos) = s.find(['<', '>', '=']) {
        let (num_part, op_part) = s.split_at(op_pos);
        if let (Ok(n), Some((op, _))) = (num_part.trim().parse::<i32>(), split_op(op_part.trim())) {
            return cmp(op, n, age);
        }
    }
    false
}
