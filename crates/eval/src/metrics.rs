//! Phase 3c metrics (ported from `hcceval/metrics.py`).
//!
//! Binary convention: the auditor's 3-way `final_status` collapses to a *flag*
//! (only "supported" clears a code). Positive class = "not substantiated".
//!   TP = gold unsupported & flagged    FP = gold supported   & flagged
//!   FN = gold unsupported & cleared     TN = gold supported   & cleared
//!
//! INTENTIONAL FIX vs. the Python original (see INTEGRATION_NOTES.md #1): evidence
//! spans and regulatory citations are scored separately. Evidence quotes must occur
//! in the note (span accuracy + hallucination); citations are validated against an
//! allowed regulatory-authority list, NOT required to appear in the note.

use crate::schema::{AuditRecord, Candidate, GoldLabel};
use agent::crosswalk::Crosswalk;
use engine::{Engine, ModelVersion};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

fn safe_div(num: f64, den: f64) -> Option<f64> {
    if den != 0.0 {
        Some(num / den)
    } else {
        None
    }
}
fn f1(p: Option<f64>, r: Option<f64>) -> Option<f64> {
    match (p, r) {
        (Some(p), Some(r)) if p > 0.0 && r > 0.0 => Some(2.0 * p * r / (p + r)),
        _ => None,
    }
}

/// Allowed regulatory authorities a substantiation citation may rely on.
const AUTHORITIES: &[&str] = &[
    "icd-10-cm official guidelines",
    "cms medicare managed care manual",
    "managed care manual",
    "radv",
    "m.e.a.t",
    "meat",
];
fn citation_valid(c: &str) -> bool {
    let lc = c.to_lowercase();
    AUTHORITIES.iter().any(|a| lc.contains(a))
}

#[derive(Debug, Default, Serialize)]
pub struct Confusion {
    pub tp: u32,
    pub fp: u32,
    #[serde(rename = "fn")]
    pub fn_: u32,
    pub tn: u32,
}

#[derive(Debug, Serialize)]
pub struct Substantiation {
    pub scored_pairs: u32,
    pub gold_pairs_unreviewed: u32,
    pub confusion: Confusion,
    pub accuracy: Option<f64>,
    pub flag_precision: Option<f64>,
    pub flag_recall: Option<f64>,
    pub flag_f1: Option<f64>,
    pub over_coding_rate: Option<f64>,
    pub under_flagging_rate: Option<f64>,
    pub over_flagging_rate: Option<f64>,
}

fn audit_index(audits: &[AuditRecord]) -> HashMap<(String, u32), &crate::schema::HccAudit> {
    let mut idx = HashMap::new();
    for a in audits {
        for h in &a.hccs {
            idx.insert((a.patient_id.clone(), h.hcc), h);
        }
    }
    idx
}

pub fn substantiation_metrics(
    audits: &[AuditRecord],
    gold: &[GoldLabel],
    include_holdout: bool,
) -> Substantiation {
    let idx = audit_index(audits);
    let mut c = Confusion::default();
    let mut unreviewed = 0u32;
    for g in gold.iter().filter(|g| include_holdout || !g.holdout) {
        match idx.get(&g.key()) {
            None => unreviewed += 1,
            Some(h) => {
                let flagged = !h.final_supported();
                match (g.supported(), flagged) {
                    (false, true) => c.tp += 1,
                    (true, true) => c.fp += 1,
                    (false, false) => c.fn_ += 1,
                    (true, false) => c.tn += 1,
                }
            }
        }
    }
    let n = (c.tp + c.fp + c.fn_ + c.tn) as f64;
    let gold_unsupported = (c.tp + c.fn_) as f64;
    let gold_supported = (c.fp + c.tn) as f64;
    let precision = safe_div(c.tp as f64, (c.tp + c.fp) as f64);
    let recall = safe_div(c.tp as f64, (c.tp + c.fn_) as f64);
    Substantiation {
        scored_pairs: (c.tp + c.fp + c.fn_ + c.tn),
        gold_pairs_unreviewed: unreviewed,
        accuracy: safe_div((c.tp + c.tn) as f64, n),
        flag_precision: precision,
        flag_recall: recall,
        flag_f1: f1(precision, recall),
        over_coding_rate: safe_div(c.fn_ as f64, n),
        under_flagging_rate: safe_div(c.fn_ as f64, gold_unsupported),
        over_flagging_rate: safe_div(c.fp as f64, gold_supported),
        confusion: c,
    }
}

#[derive(Debug, Serialize)]
pub struct System {
    pub agreement_with_gold: Option<f64>,
    pub scored_pairs: u32,
    pub changed_pairs: u32,
    pub disagreement_resolution_rate: Option<f64>,
    pub oracle_improved: u32,
    pub oracle_regressed: u32,
}

pub fn system_metrics(audits: &[AuditRecord], gold: &[GoldLabel], include_holdout: bool) -> System {
    let idx = audit_index(audits);
    let (mut agree, mut total) = (0u32, 0u32);
    let (mut changed, mut resolved, mut improved, mut regressed) = (0u32, 0u32, 0u32, 0u32);
    for g in gold.iter().filter(|g| include_holdout || !g.holdout) {
        let Some(h) = idx.get(&g.key()) else { continue };
        total += 1;
        let final_ok = h.final_supported() == g.supported();
        agree += final_ok as u32;
        if h.changed() {
            changed += 1;
            let init_ok = h.initial_supported() == g.supported();
            resolved += final_ok as u32;
            if final_ok && !init_ok {
                improved += 1;
            } else if init_ok && !final_ok {
                regressed += 1;
            }
        }
    }
    System {
        agreement_with_gold: safe_div(agree as f64, total as f64),
        scored_pairs: total,
        changed_pairs: changed,
        disagreement_resolution_rate: safe_div(resolved as f64, changed as f64),
        oracle_improved: improved,
        oracle_regressed: regressed,
    }
}

#[derive(Debug, Serialize)]
pub struct Extraction {
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    pub tp: u32,
    pub fp: u32,
    #[serde(rename = "fn")]
    pub fn_: u32,
    pub unmapped_extractions: u32,
}

/// Extraction P/R/F1 at the HCC level. Predicted HCCs come from the auditor's
/// extracted `icd10` codes (crosswalked + hierarchy-collapsed via the ENGINE);
/// gold = the engine-eligible candidate set.
pub fn extraction_metrics<V: ModelVersion>(
    audits: &[AuditRecord],
    candidates: &[Candidate],
    xwalk: &Crosswalk,
    engine: &Engine<V>,
) -> Extraction {
    let mut gold_by_patient: HashMap<String, BTreeSet<u32>> = HashMap::new();
    for c in candidates {
        gold_by_patient.entry(c.patient_id.clone()).or_default().insert(c.hcc);
    }
    let (mut tp, mut fp, mut fn_, mut unmapped) = (0u32, 0u32, 0u32, 0u32);
    for a in audits {
        let mut pred: BTreeSet<u32> = BTreeSet::new();
        for e in &a.extracted {
            let Some(icd) = e.icd10.as_ref().map(|s| engine::types::normalize_icd10(s)) else {
                unmapped += 1;
                continue;
            };
            if xwalk.describe_icd10(&icd).is_none() {
                unmapped += 1; // out-of-scope ICD-10
                continue;
            }
            if let Some(h) = xwalk.hcc_for_icd10(&icd) {
                pred.insert(h);
            }
            // in-scope but no HCC = near-miss; neither pred nor unmapped.
        }
        let pred = engine.collapse_hccs(&pred);
        let empty = BTreeSet::new();
        let gold = gold_by_patient.get(&a.patient_id).unwrap_or(&empty);
        tp += pred.intersection(gold).count() as u32;
        fp += pred.difference(gold).count() as u32;
        fn_ += gold.difference(&pred).count() as u32;
    }
    let precision = safe_div(tp as f64, (tp + fp) as f64);
    let recall = safe_div(tp as f64, (tp + fn_) as f64);
    Extraction { precision, recall, f1: f1(precision, recall), tp, fp, fn_, unmapped_extractions: unmapped }
}

#[derive(Debug, Serialize)]
pub struct Spans {
    pub span_accuracy: Option<f64>,
    pub spans_checked: u32,
    /// FIXED semantics: citations validated against the regulatory authority list.
    pub citation_validity: Option<f64>,
    pub citations_checked: u32,
    /// FIXED semantics: ungrounded *evidence* quotes only (citations excluded).
    pub hallucination_rate: Option<f64>,
    pub evidence_checked: u32,
    pub patients_without_notes: u32,
}

pub fn span_and_hallucination_metrics(
    audits: &[AuditRecord],
    notes: &HashMap<String, String>,
) -> Spans {
    let (mut span_total, mut span_ok) = (0u32, 0u32);
    let (mut cite_total, mut cite_ok) = (0u32, 0u32);
    let mut no_note = 0u32;
    for a in audits {
        let Some(note) = notes.get(&a.patient_id) else {
            no_note += 1;
            continue;
        };
        for e in &a.extracted {
            if e.evidence.trim().is_empty() {
                continue;
            }
            span_total += 1;
            if crate::notes::quote_in_note(&e.evidence, note) {
                span_ok += 1;
            }
        }
        for h in &a.hccs {
            if h.citation.trim().is_empty() {
                continue;
            }
            cite_total += 1;
            if citation_valid(&h.citation) {
                cite_ok += 1;
            }
        }
    }
    Spans {
        span_accuracy: safe_div(span_ok as f64, span_total as f64),
        spans_checked: span_total,
        citation_validity: safe_div(cite_ok as f64, cite_total as f64),
        citations_checked: cite_total,
        // hallucination = ungrounded EVIDENCE quotes / evidence quotes.
        hallucination_rate: safe_div((span_total - span_ok) as f64, span_total as f64),
        evidence_checked: span_total,
        patients_without_notes: no_note,
    }
}

#[derive(Debug, Serialize)]
pub struct Counts {
    pub patients_audited: usize,
    pub audited_hccs: usize,
    pub gold_labels: usize,
    pub gold_holdout: usize,
    pub gold_injected: usize,
    pub candidates: usize,
    pub notes_available: usize,
}

#[derive(Debug, Serialize)]
pub struct Metrics {
    pub counts: Counts,
    pub substantiation: Substantiation,
    pub system: System,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction: Option<Extraction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spans: Option<Spans>,
}

/// Compute the full metrics bundle. Extraction needs candidates+engine; spans need notes.
pub fn compute_all<V: ModelVersion>(
    audits: &[AuditRecord],
    gold: &[GoldLabel],
    candidates: &[Candidate],
    notes: &HashMap<String, String>,
    xwalk: &Crosswalk,
    engine: &Engine<V>,
    include_holdout: bool,
) -> Metrics {
    Metrics {
        counts: Counts {
            patients_audited: audits.len(),
            audited_hccs: audits.iter().map(|a| a.hccs.len()).sum(),
            gold_labels: gold.len(),
            gold_holdout: gold.iter().filter(|g| g.holdout).count(),
            gold_injected: gold.iter().filter(|g| g.source == "injected").count(),
            candidates: candidates.len(),
            notes_available: notes.len(),
        },
        substantiation: substantiation_metrics(audits, gold, include_holdout),
        system: system_metrics(audits, gold, include_holdout),
        extraction: (!candidates.is_empty())
            .then(|| extraction_metrics(audits, candidates, xwalk, engine)),
        spans: (!notes.is_empty()).then(|| span_and_hallucination_metrics(audits, notes)),
    }
}
