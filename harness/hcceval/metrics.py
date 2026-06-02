"""Compute the Phase 3c metrics from audit output + gold labels + notes.

The deliverable is the NUMBERS; every value here is computed from data — nothing
is fabricated or defaulted to a placeholder. Each metric is defined precisely in
its docstring (and surfaced in the report), because RADV decisions hinge on
exactly which confusion-matrix cell a rate counts.

Binary convention for substantiation
-------------------------------------
The auditor emits a 3-way ``final_status`` (supported / risky / unsupported);
gold is binary (supported / unsupported). For scoring, the auditor's decision
collapses to a binary *flag*: only ``supported`` clears a code; ``risky`` and
``unsupported`` are both *flagged*. The detection target (positive class) is
"this code is NOT substantiated" — the thing a RADV audit must catch.

  TP = gold unsupported & auditor flagged       (correct catch)
  FP = gold supported   & auditor flagged       (false alarm / over-flag)
  FN = gold unsupported & auditor cleared        (MISSED over-code — RADV $$ risk)
  TN = gold supported   & auditor cleared        (correct clear)
"""
from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any

from .crosswalk import Crosswalk
from .notes import quote_in_note
from .schema import AuditRecord, Candidate, GoldLabel


def _safe_div(num: float, den: float) -> float | None:
    """Ratio, or None when undefined (zero denominator) — never a fake 0.0."""
    return (num / den) if den else None


def _f1(precision: float | None, recall: float | None) -> float | None:
    if not precision or not recall:
        return None
    return 2 * precision * recall / (precision + recall)


# --------------------------------------------------------------------------- #
# Substantiation + system metrics (need gold)
# --------------------------------------------------------------------------- #
@dataclass
class Confusion:
    tp: int = 0
    fp: int = 0
    fn: int = 0
    tn: int = 0

    @property
    def n(self) -> int:
        return self.tp + self.fp + self.fn + self.tn


def substantiation_metrics(
    audits: list[AuditRecord],
    gold: list[GoldLabel],
    include_holdout: bool = True,
) -> dict[str, Any]:
    """Substantiation accuracy / precision / recall, plus the named RADV rates.

    Only (patient, HCC) pairs present in BOTH the audit output and the gold set
    are scored. Gold pairs the auditor never reviewed are reported separately as
    a coverage gap (``gold_pairs_unreviewed``) — they are not silently counted as
    correct or incorrect.
    """
    audit_idx: dict[tuple[str, int], Any] = {
        (a.patient_id, h.hcc): h for a in audits for h in a.hccs
    }
    gold_used = [g for g in gold if include_holdout or not g.holdout]

    c = Confusion()
    unreviewed: list[tuple[str, int]] = []
    for g in gold_used:
        h = audit_idx.get(g.key)
        if h is None:
            unreviewed.append(g.key)
            continue
        flagged = not h.final_supported
        if not g.supported and flagged:
            c.tp += 1
        elif g.supported and flagged:
            c.fp += 1
        elif not g.supported and not flagged:
            c.fn += 1
        else:
            c.tn += 1

    gold_unsupported = c.tp + c.fn
    gold_supported = c.fp + c.tn
    accuracy = _safe_div(c.tp + c.tn, c.n)
    precision = _safe_div(c.tp, c.tp + c.fp)
    recall = _safe_div(c.tp, c.tp + c.fn)

    return {
        "scored_pairs": c.n,
        "gold_pairs_unreviewed": len(unreviewed),
        "unreviewed_keys": unreviewed,
        "confusion": {"tp": c.tp, "fp": c.fp, "fn": c.fn, "tn": c.tn},
        # accuracy = agreement with gold on the binary supported/flagged call.
        "accuracy": accuracy,
        # precision/recall of the *flag* (positive class = unsupported).
        "flag_precision": precision,
        "flag_recall": recall,
        "flag_f1": _f1(precision, recall),
        # OVER-CODING rate = FN / all scored pairs. Share of the reviewed
        # population that is an over-code the auditor WRONGLY cleared. The
        # headline financial-exposure number (weight this — the expensive RADV
        # error).
        "over_coding_rate": _safe_div(c.fn, c.n),
        # UNDER-FLAGGING rate = FN / gold-unsupported = 1 - flag_recall. Of codes
        # that truly warrant a flag, the fraction the auditor failed to flag
        # (miss rate among things that should be caught).
        "under_flagging_rate": _safe_div(c.fn, gold_unsupported),
        # OVER-FLAGGING rate = FP / gold-supported. Of truly-supported codes, the
        # fraction wrongly flagged — clinician burden / unnecessary review.
        "over_flagging_rate": _safe_div(c.fp, gold_supported),
    }


def system_metrics(
    audits: list[AuditRecord],
    gold: list[GoldLabel],
    include_holdout: bool = True,
) -> dict[str, Any]:
    """Agreement-with-gold and disagreement-resolution (does the oracle loop help?).

    ``disagreement_resolution_rate`` looks only at codes whose ``initial_status``
    and ``final_status`` straddle the supported/flagged line (the oracle loop
    changed the call). Of those, it reports how often the FINAL call matches gold
    — i.e. whether the second pass moves decisions toward the truth. ``improved``
    / ``regressed`` split the changes into corrections vs. new mistakes.
    """
    audit_idx: dict[tuple[str, int], Any] = {
        (a.patient_id, h.hcc): h for a in audits for h in a.hccs
    }
    gold_idx = {g.key: g for g in gold if include_holdout or not g.holdout}

    agree = total = 0
    changed = resolved = improved = regressed = 0
    for key, g in gold_idx.items():
        h = audit_idx.get(key)
        if h is None:
            continue
        total += 1
        final_ok = (h.final_supported == g.supported)
        agree += final_ok
        if h.changed:
            changed += 1
            init_ok = (h.initial_supported == g.supported)
            resolved += final_ok
            if final_ok and not init_ok:
                improved += 1
            elif init_ok and not final_ok:
                regressed += 1

    return {
        "agreement_with_gold": _safe_div(agree, total),
        "scored_pairs": total,
        "changed_pairs": changed,
        "disagreement_resolution_rate": _safe_div(resolved, changed),
        "oracle_improved": improved,   # initial wrong -> final right
        "oracle_regressed": regressed,  # initial right -> final wrong
    }


# --------------------------------------------------------------------------- #
# Extraction metrics (HCC-level, vs. the mechanically-derived candidate set)
# --------------------------------------------------------------------------- #
def extraction_metrics(
    audits: list[AuditRecord],
    candidates: list[Candidate],
    xwalk: Crosswalk,
) -> dict[str, Any]:
    """Extraction precision / recall / F1 at the HCC level (micro-averaged).

    Gold = the engine-eligible candidate HCC set per patient (the in-scope coded
    conditions). Predicted = the HCCs implied by the auditor's extracted
    conditions, mapped via their ``icd10`` through the crosswalk. Extracted
    conditions with no ``icd10`` or an out-of-scope code are excluded from both
    sides (they are legitimately documentable near-misses, not extraction
    errors) — the count of such drops is reported as ``unmapped_extractions`` so
    nothing is silently ignored.

    This is a proxy for condition-level extraction quality: did the extractor
    surface evidence for the conditions that drive the coded HCCs? Span accuracy
    (below) is the orthogonal check on quote fidelity.
    """
    gold_by_patient: dict[str, set[int]] = defaultdict(set)
    for c in candidates:
        gold_by_patient[c.patient_id].add(c.hcc)

    tp = fp = fn = 0
    unmapped = 0
    per_patient: list[dict[str, Any]] = []
    for a in audits:
        pred: set[int] = set()
        for e in a.extracted:
            if not e.icd10 or not xwalk.in_scope_icd10(e.icd10):
                unmapped += 1
                continue
            hcc = xwalk.hcc_for_icd10(e.icd10)
            if hcc is None:
                # In-scope ICD-10 that maps to no HCC (near-miss) — not a coded
                # condition; excluded from HCC-level extraction scoring.
                continue
            pred.add(hcc)
        # collapse predicted HCCs through the hierarchy so they're comparable to
        # the (already collapsed) candidate gold set.
        pred = xwalk.collapse(pred)
        gold = gold_by_patient.get(a.patient_id, set())
        p_tp, p_fp, p_fn = len(pred & gold), len(pred - gold), len(gold - pred)
        tp += p_tp
        fp += p_fp
        fn += p_fn
        per_patient.append(
            {"patient_id": a.patient_id, "tp": p_tp, "fp": p_fp, "fn": p_fn}
        )

    precision = _safe_div(tp, tp + fp)
    recall = _safe_div(tp, tp + fn)
    return {
        "precision": precision,
        "recall": recall,
        "f1": _f1(precision, recall),
        "tp": tp,
        "fp": fp,
        "fn": fn,
        "unmapped_extractions": unmapped,
        "per_patient": per_patient,
    }


# --------------------------------------------------------------------------- #
# Span accuracy + hallucination (need note text)
# --------------------------------------------------------------------------- #
def span_and_hallucination_metrics(
    audits: list[AuditRecord],
    notes: dict[str, str],
) -> dict[str, Any]:
    """Span accuracy and hallucination rate against the source note text.

    * span accuracy = fraction of extracted ``evidence`` quotes that actually
      occur in the patient's note (whitespace/case-insensitive substring).
    * hallucination rate = fraction of ALL verifiable auditor quotes (extraction
      ``evidence`` + substantiation ``citation``) that do NOT occur in the note.
      A flag or citation with no textual basis is a hallucination; drive toward 0.

    Patients with no reconstructed note are skipped and counted in
    ``patients_without_notes`` (not scored as pass or fail).
    """
    span_total = span_ok = 0
    cite_total = cite_ok = 0
    no_note = 0
    examples: list[dict[str, str]] = []

    for a in audits:
        note = notes.get(a.patient_id)
        if note is None:
            no_note += 1
            continue
        for e in a.extracted:
            if not e.evidence.strip():
                continue
            span_total += 1
            if quote_in_note(e.evidence, note):
                span_ok += 1
            elif len(examples) < 20:
                examples.append(
                    {"patient_id": a.patient_id, "kind": "evidence", "quote": e.evidence}
                )
        for h in a.hccs:
            if not h.citation.strip():
                continue
            cite_total += 1
            if quote_in_note(h.citation, note):
                cite_ok += 1
            elif len(examples) < 20:
                examples.append(
                    {"patient_id": a.patient_id, "kind": "citation", "quote": h.citation}
                )

    quotes_total = span_total + cite_total
    quotes_ok = span_ok + cite_ok
    return {
        "span_accuracy": _safe_div(span_ok, span_total),
        "spans_checked": span_total,
        "citation_grounding": _safe_div(cite_ok, cite_total),
        "citations_checked": cite_total,
        # combined: any quote (evidence or citation) with no basis in the note.
        "hallucination_rate": _safe_div(quotes_total - quotes_ok, quotes_total),
        "quotes_checked": quotes_total,
        "patients_without_notes": no_note,
        "ungrounded_examples": examples,
    }


# --------------------------------------------------------------------------- #
# Calibration (only if confidence present)
# --------------------------------------------------------------------------- #
def calibration_metrics(
    audits: list[AuditRecord],
    gold: list[GoldLabel],
    bins: int = 10,
    include_holdout: bool = True,
) -> dict[str, Any] | None:
    """Reliability curve + ECE for the auditor's confidence, if any is present.

    Confidence is interpreted as P(the auditor's binary call is correct). Each
    scored pair is binned by confidence; per bin we report mean confidence vs.
    empirical accuracy. ECE is the sample-weighted mean |confidence − accuracy|.
    Returns None when no confidence values are present (nothing to calibrate).
    """
    audit_idx: dict[tuple[str, int], Any] = {
        (a.patient_id, h.hcc): h for a in audits for h in a.hccs
    }
    gold_idx = {g.key: g for g in gold if include_holdout or not g.holdout}

    pts: list[tuple[float, bool]] = []  # (confidence, correct)
    for key, g in gold_idx.items():
        h = audit_idx.get(key)
        if h is None or h.confidence is None:
            continue
        correct = (h.final_supported == g.supported)
        pts.append((max(0.0, min(1.0, h.confidence)), correct))
    if not pts:
        return None

    edges = [i / bins for i in range(bins + 1)]
    curve: list[dict[str, Any]] = []
    ece = 0.0
    n = len(pts)
    for i in range(bins):
        lo, hi = edges[i], edges[i + 1]
        # last bin is closed on the right so confidence == 1.0 lands somewhere.
        in_bin = [
            (conf, ok)
            for conf, ok in pts
            if (lo <= conf < hi) or (i == bins - 1 and conf == hi)
        ]
        if not in_bin:
            curve.append({"bin": [round(lo, 3), round(hi, 3)], "n": 0,
                          "mean_confidence": None, "accuracy": None})
            continue
        m = len(in_bin)
        mean_conf = sum(c for c, _ in in_bin) / m
        acc = sum(1 for _, ok in in_bin if ok) / m
        ece += (m / n) * abs(mean_conf - acc)
        curve.append({
            "bin": [round(lo, 3), round(hi, 3)],
            "n": m,
            "mean_confidence": mean_conf,
            "accuracy": acc,
        })
    return {"n": n, "bins": bins, "ece": ece, "curve": curve}


# --------------------------------------------------------------------------- #
# Top-level driver
# --------------------------------------------------------------------------- #
@dataclass
class EvalInputs:
    audits: list[AuditRecord]
    gold: list[GoldLabel]
    candidates: list[Candidate] = field(default_factory=list)
    notes: dict[str, str] = field(default_factory=dict)
    xwalk: Crosswalk | None = None
    include_holdout: bool = True


def compute_all(inp: EvalInputs) -> dict[str, Any]:
    """Compute the full metrics bundle. Sections needing absent inputs are skipped."""
    out: dict[str, Any] = {
        "counts": {
            "patients_audited": len(inp.audits),
            "audited_hccs": sum(len(a.hccs) for a in inp.audits),
            "gold_labels": len(inp.gold),
            "gold_holdout": sum(1 for g in inp.gold if g.holdout),
            "gold_injected": sum(1 for g in inp.gold if g.source == "injected"),
            "candidates": len(inp.candidates),
            "notes_available": len(inp.notes),
        },
        "substantiation": substantiation_metrics(inp.audits, inp.gold, inp.include_holdout),
        "system": system_metrics(inp.audits, inp.gold, inp.include_holdout),
    }
    if inp.xwalk is not None and inp.candidates:
        out["extraction"] = extraction_metrics(inp.audits, inp.candidates, inp.xwalk)
    if inp.notes:
        out["spans"] = span_and_hallucination_metrics(inp.audits, inp.notes)
    cal = calibration_metrics(inp.audits, inp.gold, include_holdout=inp.include_holdout)
    if cal is not None:
        out["calibration"] = cal
    return out
