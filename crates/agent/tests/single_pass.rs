//! Deterministic single-pass audit test using the mock LLM provider — no live
//! model, fully reproducible. Validates the orchestration: engine ground truth +
//! extraction + per-HCC substantiation → flagged-code report.

use agent::audit::audit_patient;
use agent::crosswalk::Crosswalk;
use agent::fhir::{Condition, PatientRecord};
use agent::llm::MockProvider;
use engine::demographics::Date;
use engine::{Engine, Sex};

fn paths() -> (String, String) {
    let m = env!("CARGO_MANIFEST_DIR");
    (
        format!("{m}/../engine/tests/fixtures/v28"),
        format!("{m}/../../crosswalks/snomed_to_icd10_v28.csv"),
    )
}

#[test]
fn single_pass_flags_unsupported_hcc() {
    let (tables, xwalk_path) = paths();
    let engine = Engine::load_v28(tables).expect("engine");
    let xwalk = Crosswalk::load(xwalk_path).expect("crosswalk");

    // Patient with T2DM (44054006 → E119 → HCC38) and chronic CHF
    // (88805009 → I509 → HCC226). Engine yields HCCs {38, 226} (sorted).
    let rec = PatientRecord {
        id: "T1".into(),
        name: "Test Patient".into(),
        date_of_birth: Date::new(1950, 1, 1),
        sex: Sex::Female,
        conditions: vec![
            Condition { snomed: "44054006".into(), description: "Diabetes mellitus type 2".into() },
            Condition { snomed: "88805009".into(), description: "Chronic congestive heart failure".into() },
        ],
        note: "Assessment: Type 2 diabetes, on metformin, A1c reviewed.".into(),
    };

    // Scripted responses: extraction, then one judgment per HCC in order (38, 226).
    let provider = MockProvider::new(vec![
        r#"{"conditions":[{"name":"Type 2 diabetes mellitus","evidence":"Type 2 diabetes, on metformin","section":"Assessment"}]}"#.into(),
        // HCC 38 — diabetes: supported.
        r#"{"status":"supported","meat_present":["Assessment","Treatment","Evaluation"],"specificity_supported":true,"citation":"ICD-10-CM Official Guidelines, Section IV","documentation_gap":"","rationale":"Diabetes assessed with treatment and A1c."}"#.into(),
        // HCC 226 — heart failure: unsupported (note never mentions HF).
        r#"{"status":"unsupported","meat_present":[],"specificity_supported":false,"citation":"CMS Medicare Managed Care Manual Ch.7","documentation_gap":"No heart failure documented in note","rationale":"The note does not mention heart failure."}"#.into(),
    ]);

    let report = audit_patient(&engine, &provider, &xwalk, &rec).expect("audit");

    assert_eq!(report.hccs.len(), 2);
    assert_eq!(report.hccs[0].hcc, 38);
    assert!(report.hccs[0].judgment.is_supported());
    assert_eq!(report.hccs[1].hcc, 226);
    assert!(report.hccs[1].judgment.is_flagged());

    let flagged: Vec<_> = report.flagged().collect();
    assert_eq!(flagged.len(), 1);
    assert_eq!(flagged[0].hcc, 226);
    assert_eq!(flagged[0].judgment.documentation_gap, "No heart failure documented in note");

    // Extraction surfaced the documented diabetes.
    assert_eq!(report.extracted.len(), 1);
    assert!(report.extracted[0].name.to_lowercase().contains("diabetes"));
}
