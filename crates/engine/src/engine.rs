//! The deterministic scoring engine: orchestrates mapping → hierarchy →
//! interactions → score, mirroring the CMS V28 reference pipeline.

use crate::demographics::{
    age_sex_variable, calculate_age, is_disabled, is_originally_disabled,
};
use crate::tables::{evaluate_age_rule, RiskTables};
use crate::types::*;
use crate::version::{ModelVersion, V28};
use std::collections::{BTreeMap, BTreeSet};

/// The scoring engine, pinned to model version `V`.
#[derive(Debug)]
pub struct Engine<V: ModelVersion> {
    tables: RiskTables<V>,
    /// Apply MCE (Medicare Code Editor) age conditions during ICD→CC mapping.
    /// CMS reference default is `true`.
    switch_edits: bool,
}

impl Engine<V28> {
    /// Construct a V28 engine from tables in a directory (canonical CMS file names).
    pub fn load_v28(dir: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        Ok(Engine { tables: RiskTables::<V28>::load_from_dir(dir)?, switch_edits: true })
    }
}

impl<V: ModelVersion> Engine<V> {
    pub fn new(tables: RiskTables<V>) -> Self {
        Engine { tables, switch_edits: true }
    }

    pub fn with_switch_edits(mut self, on: bool) -> Self {
        self.switch_edits = on;
        self
    }

    pub fn tables(&self) -> &RiskTables<V> {
        &self.tables
    }

    /// Score one beneficiary, returning a fully decomposed result with provenance.
    pub fn score(&self, person: &PersonInput) -> ScoreResult {
        let age = calculate_age(person.date_of_birth, V::cutoff_date());
        let segment = Segment::derive(age, person.dual_status);
        let seg = segment.column();

        // The set of valid (payment) HCCs is exactly the hierarchy's primary HCCs.
        let valid: BTreeSet<u32> = self.tables.hierarchies().iter().map(|(h, _)| *h).collect();

        // --- 1. ICD-10 → CC mapping with MCE / age / sex edits ---
        // cc -> diagnoses that produced it (only CCs that are valid payment HCCs).
        let mut cc_diags: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
        for dx in &person.diagnoses {
            for row in self.tables.mappings_for(&dx.icd10) {
                if !self.mapping_passes(row, age, person.sex) {
                    continue;
                }
                if valid.contains(&row.cc) {
                    cc_diags.entry(row.cc).or_default().insert(dx.icd10.clone());
                }
            }
        }

        // --- 2. CC223 recode (CMS V28 reference rule) ---
        // CC223 (acute-on-chronic HF) only counts alongside another HF CC.
        if cc_diags.contains_key(&223)
            && ![221u32, 222, 224, 225, 226].iter().any(|c| cc_diags.contains_key(c))
        {
            cc_diags.remove(&223);
        }

        // CC == HCC numbering in V28. Present (pre-hierarchy) HCC set.
        let present: BTreeSet<u32> = cc_diags.keys().copied().collect();

        // --- 3. Hierarchy: more-severe HCC trumps related less-severe HCCs ---
        // trumped HCC -> the HCC that suppressed it (read from the pre-hierarchy snapshot).
        let mut trumped_by: BTreeMap<u32, u32> = BTreeMap::new();
        for (hcc, secondaries) in self.tables.hierarchies() {
            if present.contains(hcc) {
                for t in secondaries {
                    if present.contains(t) {
                        trumped_by.entry(*t).or_insert(*hcc);
                    }
                }
            }
        }
        let final_hccs: BTreeSet<u32> =
            present.iter().copied().filter(|h| !trumped_by.contains_key(h)).collect();

        // --- 4. Disease-category flags (from surviving HCCs) ---
        let category_flag = |name: &str| -> bool {
            self.tables
                .categories()
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, hccs)| hccs.iter().any(|h| final_hccs.contains(h)))
                .unwrap_or(false)
        };

        // Resolve an interaction component variable to 0/1.
        let disabled = is_disabled(age, person.orec);
        let flag = |var: &str| -> bool {
            if let Some(num) = var.strip_prefix("HCC") {
                num.parse::<u32>().map(|h| final_hccs.contains(&h)).unwrap_or(false)
            } else if var == "DISABL" {
                disabled
            } else {
                category_flag(var)
            }
        };

        // --- 5. Build the variable → value map for scoring ---
        let mut values: BTreeMap<String, f64> = BTreeMap::new();

        // Age/sex demographic cell.
        let age_sex = age_sex_variable(age, person.sex);
        values.insert(age_sex.clone(), 1.0);

        // Other demographic flags.
        let origdis = is_originally_disabled(age, person.orec);
        let mut extra_demographic = Vec::new();
        let add_demo = |values: &mut BTreeMap<String, f64>,
                            extra: &mut Vec<Factor>,
                            var: &str,
                            on: bool| {
            if on {
                values.insert(var.to_string(), 1.0);
                extra.push(Factor { variable: var.to_string(), coefficient: self.tables.coef(var, seg) });
            }
        };
        add_demo(&mut values, &mut extra_demographic, "ORIGDIS", origdis);
        add_demo(
            &mut values,
            &mut extra_demographic,
            "OriginallyDisabled_Female",
            origdis && person.sex == Sex::Female,
        );
        add_demo(
            &mut values,
            &mut extra_demographic,
            "OriginallyDisabled_Male",
            origdis && person.sex == Sex::Male,
        );
        add_demo(&mut values, &mut extra_demographic, "LTIMCAID", person.long_term_medicaid);

        // HCC flags.
        for h in &final_hccs {
            values.insert(format!("HCC{h}"), 1.0);
        }

        // HCC count bucket (D1..D9, D10P).
        let count = final_hccs.len();
        let count_var = match count {
            0 => "D0".to_string(),
            n if n >= 10 => "D10P".to_string(),
            n => format!("D{n}"),
        };
        if count > 0 {
            values.insert(count_var.clone(), 1.0);
        }

        // Interaction terms.
        let mut interactions = Vec::new();
        for (name, v1, v2) in self.tables.interactions() {
            if flag(v1) && flag(v2) {
                values.insert(name.clone(), 1.0);
                interactions.push(Factor { variable: name.clone(), coefficient: self.tables.coef(name, seg) });
            }
        }

        // --- 6. Score = Σ value × coefficient over the segment column ---
        let mut raw = 0.0f64;
        for (var, val) in &values {
            raw += val * self.tables.coef(var, seg);
        }
        let raw_score = round3(raw);

        // --- 7. Assemble HCC assignments and provenance ---
        let hccs: Vec<HccAssignment> = final_hccs
            .iter()
            .map(|h| HccAssignment {
                hcc: *h,
                coefficient: self.tables.coef(&format!("HCC{h}"), seg),
                triggering_diagnoses: cc_diags
                    .get(h)
                    .map(|d| d.iter().cloned().collect())
                    .unwrap_or_default(),
                trumped: self
                    .tables
                    .hierarchies()
                    .iter()
                    .find(|(hcc, _)| hcc == h)
                    .map(|(_, sec)| sec.iter().copied().filter(|t| trumped_by.get(t) == Some(h)).collect())
                    .unwrap_or_default(),
            })
            .collect();

        let mut provenance: Vec<Provenance> = Vec::new();
        for (cc, diags) in &cc_diags {
            for icd10 in diags {
                provenance.push(Provenance {
                    icd10: icd10.clone(),
                    cc: *cc,
                    hcc: *cc,
                    kept: final_hccs.contains(cc),
                    trumped_by: trumped_by.get(cc).copied(),
                });
            }
        }
        provenance.sort_by(|a, b| a.icd10.cmp(&b.icd10).then(a.hcc.cmp(&b.hcc)));

        ScoreResult {
            person_id: person.person_id.clone(),
            model: V::NAME,
            segment,
            age,
            demographic: Factor { variable: age_sex.clone(), coefficient: self.tables.coef(&age_sex, seg) },
            extra_demographic,
            hccs,
            hcc_count: Factor { variable: count_var.clone(), coefficient: self.tables.coef(&count_var, seg) },
            interactions,
            raw_score,
            provenance,
        }
    }

    /// Whether a mapping row's edit conditions pass for this beneficiary.
    fn mapping_passes(&self, row: &crate::tables::MappingRow, age: i32, sex: Sex) -> bool {
        let sex_code = match sex {
            Sex::Male => 1u8,
            Sex::Female => 2u8,
        };
        let age_ok = |cond: &Option<String>| match cond {
            Some(c) => evaluate_age_rule(c, age),
            None => true,
        };
        let sex_ok = match row.sex_edit {
            Some(s) => s == sex_code,
            None => true,
        };
        if self.switch_edits {
            age_ok(&row.mce_age) && age_ok(&row.age_edit) && sex_ok
        } else {
            age_ok(&row.age_edit) && sex_ok
        }
    }
}

/// Round to 3 decimals (CMS convention). Inputs are sums of 3-decimal coefficients,
/// so this only cleans floating-point error.
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
