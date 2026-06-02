//! Render computed metrics (ports `hcceval/report.py`). Undefined ratios print as
//! `n/a`, never a fabricated 0.

use crate::metrics::Metrics;

fn pct(x: Option<f64>) -> String {
    x.map_or("n/a".into(), |v| format!("{:.1}%", v * 100.0))
}
fn num(x: Option<f64>) -> String {
    x.map_or("n/a".into(), |v| format!("{v:.3}"))
}

pub fn render_text(m: &Metrics) -> String {
    let mut l: Vec<String> = Vec::new();
    let eq = "=".repeat(64);
    l.push(eq.clone());
    l.push("HCC Substantiation Auditor — Evaluation Metrics (Rust)".into());
    l.push(eq.clone());
    let c = &m.counts;
    l.push(format!("patients audited: {}   audited HCCs: {}", c.patients_audited, c.audited_hccs));
    l.push(format!(
        "gold labels: {} (injected {}, holdout {})   candidates: {}   notes: {}",
        c.gold_labels, c.gold_injected, c.gold_holdout, c.candidates, c.notes_available
    ));

    let s = &m.substantiation;
    let cf = &s.confusion;
    l.push(String::new());
    l.push(format!("-- Substantiation (positive class = unsupported/flagged) {}", "-".repeat(7)));
    l.push(format!("  scored pairs:            {}{}", s.scored_pairs,
        if s.gold_pairs_unreviewed > 0 { format!("   (unreviewed gold: {})", s.gold_pairs_unreviewed) } else { String::new() }));
    l.push(format!("  confusion:               TP={} FP={} FN={} TN={}", cf.tp, cf.fp, cf.fn_, cf.tn));
    l.push(format!("  accuracy (agreement):    {}", pct(s.accuracy)));
    l.push(format!("  flag precision:          {}", pct(s.flag_precision)));
    l.push(format!("  flag recall:             {}", pct(s.flag_recall)));
    l.push(format!("  flag F1:                 {}", num(s.flag_f1)));
    l.push(format!("  OVER-CODING rate:        {}   (FN/all — expensive RADV error)", pct(s.over_coding_rate)));
    l.push(format!("  under-flagging rate:     {}   (FN/gold-unsupported = 1-recall)", pct(s.under_flagging_rate)));
    l.push(format!("  over-flagging rate:      {}   (FP/gold-supported)", pct(s.over_flagging_rate)));

    let sy = &m.system;
    l.push(String::new());
    l.push(format!("-- System {}", "-".repeat(53)));
    l.push(format!("  agreement with gold:     {}", pct(sy.agreement_with_gold)));
    l.push(format!("  changed by oracle loop:  {}", sy.changed_pairs));
    l.push(format!("  disagreement-resolution: {}   (improved {}, regressed {})",
        pct(sy.disagreement_resolution_rate), sy.oracle_improved, sy.oracle_regressed));

    if let Some(e) = &m.extraction {
        l.push(String::new());
        l.push(format!("-- Extraction (HCC-level vs. candidate set) {}", "-".repeat(20)));
        l.push(format!("  precision:               {}", pct(e.precision)));
        l.push(format!("  recall:                  {}", pct(e.recall)));
        l.push(format!("  F1:                      {}", num(e.f1)));
        l.push(format!("  TP/FP/FN:                {}/{}/{}   (unmapped extractions: {})", e.tp, e.fp, e.fn_, e.unmapped_extractions));
    }
    if let Some(sp) = &m.spans {
        l.push(String::new());
        l.push(format!("-- Spans, citations & hallucination {}", "-".repeat(28)));
        l.push(format!("  span accuracy:           {}   ({} spans)", pct(sp.span_accuracy), sp.spans_checked));
        l.push(format!("  citation validity:       {}   ({} citations, vs authority list)", pct(sp.citation_validity), sp.citations_checked));
        l.push(format!("  HALLUCINATION rate:      {}   ({} evidence quotes; target 0)", pct(sp.hallucination_rate), sp.evidence_checked));
        if sp.patients_without_notes > 0 {
            l.push(format!("  patients without notes:  {} (skipped)", sp.patients_without_notes));
        }
    }
    if let Some(cal) = &m.calibration {
        l.push(String::new());
        l.push(format!("-- Calibration (n={}, ECE={}) {}", cal.n, num(Some(cal.ece)), "-".repeat(28)));
        for b in &cal.curve {
            if b.n == 0 {
                continue;
            }
            l.push(format!("  [{:.1},{:.1}]   n={:<4} mean_conf={}  acc={}",
                b.bin[0], b.bin[1], b.n, num(b.mean_confidence), num(b.accuracy)));
        }
    }
    l.push(eq);
    l.join("\n")
}

pub fn render_json(m: &Metrics) -> String {
    serde_json::to_string_pretty(m).unwrap_or_default()
}
