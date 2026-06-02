//! Parity gate: the Rust eval reproduces the Python `hcceval` metrics on the
//! shared committed fixtures (bit-for-bit on every metric except the intentionally
//! fixed citation/hallucination semantics). Self-contained: uses the committed
//! engine fixture tables + crosswalk, no gitignored data.

use agent::crosswalk::Crosswalk;
use engine::Engine;
use eval::{metrics, notes, schema};

fn p(rel: &str) -> String {
    format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel)
}

fn approx(a: Option<f64>, b: f64) {
    assert!(a.is_some() && (a.unwrap() - b).abs() < 1e-9, "expected {b}, got {a:?}");
}

#[test]
fn matches_python_reference_on_fixtures() {
    let fx = "../../harness/fixtures";
    let audits = schema::load_audits(p(&format!("{fx}/audit_results.jsonl"))).unwrap();
    let gold = schema::load_gold(p(&format!("{fx}/gold_labels.jsonl"))).unwrap();
    let candidates = schema::load_candidates(p(&format!("{fx}/candidates.jsonl"))).unwrap();
    let note_map = notes::load_notes(p(&format!("{fx}/notes"))).unwrap();
    let xwalk = Crosswalk::load(p("../../crosswalks/snomed_to_icd10_v28.csv")).unwrap();
    let engine = Engine::load_v28(p("../engine/tests/fixtures/v28")).unwrap();

    let m = metrics::compute_all(&audits, &gold, &candidates, &note_map, &xwalk, &engine, true);

    // Substantiation — matches Python expected_metrics.json exactly.
    let s = &m.substantiation;
    assert_eq!((s.confusion.tp, s.confusion.fp, s.confusion.fn_, s.confusion.tn), (2, 1, 1, 2));
    approx(s.accuracy, 2.0 / 3.0);
    approx(s.flag_precision, 2.0 / 3.0);
    approx(s.flag_recall, 2.0 / 3.0);
    approx(s.over_coding_rate, 1.0 / 6.0);

    // System — oracle loop accounting.
    assert_eq!(m.system.changed_pairs, 3);
    assert_eq!((m.system.oracle_improved, m.system.oracle_regressed), (2, 1));
    approx(m.system.disagreement_resolution_rate, 2.0 / 3.0);

    // Extraction — HCC-level vs candidate set (engine hierarchy collapse).
    let e = m.extraction.as_ref().unwrap();
    assert_eq!((e.tp, e.fp, e.fn_, e.unmapped_extractions), (3, 1, 3, 1));
    approx(e.precision, 0.75);
    approx(e.recall, 0.5);

    // Spans — span accuracy matches Python; hallucination is evidence-only (the fix).
    let sp = m.spans.as_ref().unwrap();
    approx(sp.span_accuracy, 0.8);
    assert_eq!(sp.spans_checked, 5);
    approx(sp.hallucination_rate, 0.2); // Python (incl. citations) was 0.25

    // Calibration — matches Python (n=6, ECE=0.3).
    let cal = m.calibration.as_ref().unwrap();
    assert_eq!(cal.n, 6);
    assert!((cal.ece - 0.3).abs() < 1e-9, "ECE {} != 0.3", cal.ece);
}

#[test]
fn injection_set_is_balanced_known_truth() {
    let set = eval::inject::build_injection_set(eval::inject::DEFAULT_SPECS, 2);
    assert_eq!(set.bundles.len(), 8);
    assert_eq!(set.gold.iter().filter(|g| g.supported()).count(), 4);
    assert_eq!(set.gold.iter().filter(|g| !g.supported()).count(), 4);
    assert_eq!(set.gold.iter().filter(|g| g.holdout).count(), 4);
}
