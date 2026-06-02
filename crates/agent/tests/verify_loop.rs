//! Deterministic verify → self-correct loop tests using the mock LLM provider —
//! no live model, fully reproducible. Scripts disagreements in BOTH directions and
//! asserts the loop detects them, self-corrects, and logs initial ≠ final + reason.

use agent::audit::audit_patient_verified;
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

fn patient() -> PatientRecord {
    // T2DM (44054006 → E119 → HCC38) and chronic CHF (88805009 → I509 → HCC226).
    // Engine yields HCCs {38, 226} (sorted). The note documents ONLY diabetes;
    // heart failure is never mentioned.
    PatientRecord {
        id: "T1".into(),
        name: "Test Patient".into(),
        date_of_birth: Date::new(1950, 1, 1),
        sex: Sex::Female,
        conditions: vec![
            Condition { snomed: "44054006".into(), description: "Diabetes mellitus type 2".into() },
            Condition { snomed: "88805009".into(), description: "Chronic congestive heart failure".into() },
        ],
        note: "Assessment: Type 2 diabetes, on metformin, A1c reviewed.".into(),
    }
}

#[test]
fn detects_and_self_corrects_both_directions() {
    let (tables, xwalk_path) = paths();
    let engine = Engine::load_v28(tables).expect("engine");
    let xwalk = Crosswalk::load(xwalk_path).expect("crosswalk");
    let rec = patient();

    // Scripted, in call order:
    //   1. extraction — diabetes documented with a verbatim in-note span (icd10 E119);
    //      no heart-failure condition surfaced.
    //   2. judge HCC 38 (diabetes) — WRONGLY "unsupported" (extraction contradicts it).
    //   3. judge HCC 226 (heart failure) — WRONGLY "supported" (no note evidence).
    //   4. recheck HCC 38 — flagged_with_evidence → corrected to "supported".
    //   5. recheck HCC 226 — supported_no_evidence → corrected to "unsupported".
    let provider = MockProvider::new(vec![
        r#"{"conditions":[{"name":"Type 2 diabetes mellitus","evidence":"Type 2 diabetes, on metformin","section":"Assessment","icd10":"E119"}]}"#.into(),
        r#"{"status":"unsupported","meat_present":[],"specificity_supported":false,"citation":"CMS Medicare Managed Care Manual Ch.7","documentation_gap":"weak","rationale":"Erroneous initial flag."}"#.into(),
        r#"{"status":"supported","meat_present":["Assessment"],"specificity_supported":true,"citation":"ICD-10-CM Official Guidelines, Section IV","documentation_gap":"","rationale":"Erroneously called supported."}"#.into(),
        // recheck for HCC 38 → flip to supported
        r#"{"status":"supported","meat_present":["Assessment","Treatment"],"specificity_supported":true,"citation":"CMS Medicare Managed Care Manual Ch.7","documentation_gap":"","rationale":"Diabetes documented with treatment (metformin).","correction_reason":"Note span substantiates the code; initial flag was wrong.","confidence":0.9}"#.into(),
        // recheck for HCC 226 → flip to unsupported
        r#"{"status":"unsupported","meat_present":[],"specificity_supported":false,"citation":"CMS-HCC RADV medical record review documentation standards","documentation_gap":"No heart failure documented in note","rationale":"The note never mentions heart failure.","correction_reason":"No note evidence supports the coded HF; initial 'supported' had no basis.","confidence":0.95}"#.into(),
    ]);

    let a = audit_patient_verified(&engine, &provider, &xwalk, &rec).expect("audit");

    // Engine remains authoritative: both coded HCCs present, score unchanged.
    let gt = agent::groundtruth::ground_truth(&engine, &rec, &xwalk);
    assert_eq!(a.hccs.len(), 2);
    assert_eq!(a.raw_score, gt.raw_score);

    // HCC 38: flagged-with-evidence → corrected unsupported → supported.
    let h38 = a.hccs.iter().find(|h| h.hcc == 38).expect("hcc 38");
    assert_eq!(h38.initial.status, "unsupported");
    assert_eq!(h38.final_judgment.status, "supported");
    assert!(h38.corrected);
    assert!(h38.initial.status != h38.final_judgment.status);
    assert!(h38.correction_reason.contains("flagged_with_evidence"));
    assert!(!h38.correction_reason.is_empty());
    assert_eq!(h38.final_judgment.confidence, Some(0.9));

    // HCC 226: supported-without-evidence → corrected supported → unsupported.
    let h226 = a.hccs.iter().find(|h| h.hcc == 226).expect("hcc 226");
    assert_eq!(h226.initial.status, "supported");
    assert_eq!(h226.final_judgment.status, "unsupported");
    assert!(h226.corrected);
    assert!(h226.correction_reason.contains("supported_no_evidence"));

    // Both HCCs were re-examined and changed.
    assert_eq!(a.corrected().count(), 2);
}

#[test]
fn no_disagreement_means_no_correction() {
    let (tables, xwalk_path) = paths();
    let engine = Engine::load_v28(tables).expect("engine");
    let xwalk = Crosswalk::load(xwalk_path).expect("crosswalk");
    let rec = patient();

    // Consistent judgments: diabetes documented → supported; heart failure absent →
    // unsupported. Verification finds NO contradiction, so no recheck is requested.
    // Exactly three responses are scripted (extraction + 2 judgments); a spurious
    // recheck would exhaust the mock and error.
    let provider = MockProvider::new(vec![
        r#"{"conditions":[{"name":"Type 2 diabetes mellitus","evidence":"Type 2 diabetes, on metformin","section":"Assessment","icd10":"E119"}]}"#.into(),
        r#"{"status":"supported","meat_present":["Assessment","Treatment"],"specificity_supported":true,"citation":"ICD-10-CM Official Guidelines, Section IV","documentation_gap":"","rationale":"Diabetes assessed and treated."}"#.into(),
        r#"{"status":"unsupported","meat_present":[],"specificity_supported":false,"citation":"CMS-HCC RADV medical record review documentation standards","documentation_gap":"No heart failure documented","rationale":"Note does not mention heart failure."}"#.into(),
    ]);

    let a = audit_patient_verified(&engine, &provider, &xwalk, &rec).expect("audit");

    assert_eq!(a.corrected().count(), 0);
    for h in &a.hccs {
        assert!(!h.corrected);
        assert_eq!(h.initial.status, h.final_judgment.status);
        assert!(h.correction_reason.is_empty());
    }
    // Final statuses match the consistent single-pass judgments.
    assert!(a.hccs.iter().find(|h| h.hcc == 38).unwrap().final_judgment.is_supported());
    assert!(a.hccs.iter().find(|h| h.hcc == 226).unwrap().final_judgment.is_flagged());
}
