//! Hand-computed agreement tests for the V28 engine.
//!
//! Each scenario is scored by hand from the exact CMS V28 coefficients in
//! `tests/fixtures/v28/` (a public-domain subset; see `fixtures/NOTICE.md`) and
//! asserted against the engine's output. These cover the hard parts: hierarchy
//! trumping, V28 constraining (shared coefficients), disease interactions, the
//! MCE age edit, the CC223 recode, provenance, and determinism.

use engine::demographics::Date;
use engine::tables::evaluate_age_rule;
use engine::{Diagnosis, DualStatus, Engine, PersonInput, Segment, Sex};

fn fixtures() -> String {
    format!("{}/tests/fixtures/v28", env!("CARGO_MANIFEST_DIR"))
}

fn engine() -> Engine<engine::V28> {
    Engine::load_v28(fixtures()).expect("load V28 fixtures")
}

/// Build a non-dual beneficiary with the given DOB/sex/diagnoses.
fn bene(id: &str, dob: Date, sex: Sex, codes: &[&str]) -> PersonInput {
    PersonInput {
        person_id: id.into(),
        date_of_birth: dob,
        sex,
        orec: 0,
        long_term_medicaid: false,
        dual_status: DualStatus::NonDual,
        diagnoses: codes.iter().map(|c| Diagnosis::new(*c)).collect(),
    }
}

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "expected {b}, got {a}");
}

fn hcc_numbers(r: &engine::ScoreResult) -> Vec<u32> {
    r.hccs.iter().map(|h| h.hcc).collect()
}

#[test]
fn diabetes_hierarchy_trumps_less_severe() {
    // Female, age 71 as of 2026-02-01 (band F70_74 = 0.395).
    // E11.22 → CC37 (chronic complications), E11.65 → CC38 (uncomplicated).
    // HCC37 trumps HCC38 → final {37}. Score = 0.395 + HCC37(0.166) = 0.561.
    let r = engine().score(&bene("A", Date::new(1954, 3, 10), Sex::Female, &["E11.22", "E11.65"]));

    assert_eq!(r.age, 71);
    assert_eq!(r.segment, Segment::CommunityNonDualAged);
    assert_eq!(r.demographic.variable, "F70_74");
    approx(r.demographic.coefficient, 0.395);
    assert_eq!(hcc_numbers(&r), vec![37]);
    approx(r.hccs[0].coefficient, 0.166);
    assert_eq!(r.hccs[0].triggering_diagnoses, vec!["E1122".to_string()]);
    assert_eq!(r.hccs[0].trumped, vec![38]);
    approx(r.raw_score, 0.561);

    // Provenance records both, with HCC38 trumped by HCC37.
    let p38 = r.provenance.iter().find(|p| p.hcc == 38).unwrap();
    assert!(!p38.kept);
    assert_eq!(p38.trumped_by, Some(37));
    let p37 = r.provenance.iter().find(|p| p.hcc == 37).unwrap();
    assert!(p37.kept);
}

#[test]
fn multi_family_with_interactions() {
    // Male, age 68 (band M65_69 = 0.332).
    // E11.65 → HCC38 (diabetes), I50.1 → HCC226 (HF), N18.4 → HCC327 (CKD st.4).
    // No trumping. Interactions: DIABETES_HF (0.112) + HF_KIDNEY (0.176).
    // Score = 0.332 + 0.166 + 0.36 + 0.514 + 0.112 + 0.176 = 1.66.
    let r = engine().score(&bene("B", Date::new(1957, 6, 1), Sex::Male, &["E11.65", "I50.1", "N18.4"]));

    assert_eq!(r.age, 68);
    assert_eq!(r.demographic.variable, "M65_69");
    assert_eq!(hcc_numbers(&r), vec![38, 226, 327]);
    let mut interactions: Vec<_> = r.interactions.iter().map(|f| f.variable.as_str()).collect();
    interactions.sort();
    assert_eq!(interactions, vec!["DIABETES_HF_V28", "HF_KIDNEY_V28"]);
    approx(r.raw_score, 1.66);
}

#[test]
fn ckd_hierarchy_and_copd_age_edit_passes() {
    // Female, age 80 (band F80_84 = 0.524).
    // J44.9 → HCC280 (COPD; age>=18 ok), N18.30 → CC329, N18.5 → HCC326 (st.5).
    // HCC326 trumps 329 → final {280, 326}. Score = 0.524 + 0.319 + 0.815 = 1.658.
    let r = engine().score(&bene("C", Date::new(1946, 1, 1), Sex::Female, &["J44.9", "N18.30", "N18.5"]));

    assert_eq!(r.age, 80);
    assert_eq!(hcc_numbers(&r), vec![280, 326]);
    approx(r.raw_score, 1.658);
    let p329 = r.provenance.iter().find(|p| p.hcc == 329).unwrap();
    assert!(!p329.kept);
    assert_eq!(p329.trumped_by, Some(326));
}

#[test]
fn copd_age_edit_drops_code_for_minor() {
    // Age 10: J44.9 requires age >= 18, so CC280 is not assigned at all.
    let r = engine().score(&bene("D", Date::new(2015, 1, 1), Sex::Male, &["J44.9"]));
    assert_eq!(r.age, 11);
    assert!(r.hccs.is_empty(), "no HCC should be assigned");
    assert!(r.provenance.is_empty(), "no provenance when the only code is edited out");
}

#[test]
fn cc223_recode_removes_lone_device_code() {
    // Z95811 (heart-assist device) → CC223. With no other HF CC present, the V28
    // CC223 recode removes it → no HCC.
    let lone = engine().score(&bene("E1", Date::new(1950, 1, 1), Sex::Female, &["Z95811"]));
    assert!(lone.hccs.is_empty(), "lone CC223 should be recoded away");

    // With a companion HF code (I50.1 → HCC226), CC223 is retained and, being more
    // severe, trumps HCC226 → final {223}.
    let paired = engine().score(&bene("E2", Date::new(1950, 1, 1), Sex::Female, &["Z95811", "I50.1"]));
    assert_eq!(hcc_numbers(&paired), vec![223]);
    let p226 = paired.provenance.iter().find(|p| p.hcc == 226).unwrap();
    assert_eq!(p226.trumped_by, Some(223));
}

#[test]
fn deterministic_repeated_scoring() {
    let e = engine();
    let p = bene("F", Date::new(1957, 6, 1), Sex::Male, &["E11.65", "I50.1", "N18.4"]);
    let a = e.score(&p);
    let b = e.score(&p);
    assert_eq!(hcc_numbers(&a), hcc_numbers(&b));
    approx(a.raw_score, b.raw_score);
    // Diagnosis order must not matter.
    let p2 = bene("F", Date::new(1957, 6, 1), Sex::Male, &["N18.4", "E11.65", "I50.1"]);
    approx(e.score(&p2).raw_score, a.raw_score);
}

#[test]
fn age_rule_evaluator_forms() {
    assert!(evaluate_age_rule("age >= 18", 18));
    assert!(!evaluate_age_rule("age >= 18", 17));
    assert!(evaluate_age_rule("age < 50", 49));
    assert!(!evaluate_age_rule("age < 50", 50));
    assert!(evaluate_age_rule("age = 0", 0));
    assert!(!evaluate_age_rule("age = 0", 1));
    // Chained range form found in MCE conditions.
    assert!(evaluate_age_rule("0 <= age <= 17", 10));
    assert!(!evaluate_age_rule("0 <= age <= 17", 18));
    assert!(evaluate_age_rule("9 <= age <= 64", 9));
    assert!(!evaluate_age_rule("9 <= age <= 64", 8));
}
