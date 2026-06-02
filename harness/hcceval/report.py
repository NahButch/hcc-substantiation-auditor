"""Render the computed metrics: a plain-text table for stdout, plus JSON and
Markdown summaries the docs track can consume verbatim.

Undefined metrics (zero denominator) print as ``n/a`` rather than a fake ``0.0``
— the harness never invents a number it could not compute.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def _pct(x: float | None) -> str:
    return "n/a" if x is None else f"{x * 100:.1f}%"


def _num(x: float | None, nd: int = 3) -> str:
    return "n/a" if x is None else f"{x:.{nd}f}"


def render_text(m: dict[str, Any]) -> str:
    """Human-readable metrics table for stdout."""
    L: list[str] = []
    a = L.append
    a("=" * 64)
    a("HCC Substantiation Auditor — Evaluation Metrics")
    a("=" * 64)

    c = m["counts"]
    a(f"patients audited: {c['patients_audited']}   audited HCCs: {c['audited_hccs']}")
    a(f"gold labels: {c['gold_labels']} "
      f"(injected {c['gold_injected']}, holdout {c['gold_holdout']})   "
      f"candidates: {c['candidates']}   notes: {c['notes_available']}")

    s = m["substantiation"]
    cf = s["confusion"]
    a("")
    a("-- Substantiation (positive class = unsupported/flagged) " + "-" * 7)
    a(f"  scored pairs:            {s['scored_pairs']}"
      + (f"   (unreviewed gold: {s['gold_pairs_unreviewed']})"
         if s["gold_pairs_unreviewed"] else ""))
    a(f"  confusion:               TP={cf['tp']} FP={cf['fp']} "
      f"FN={cf['fn']} TN={cf['tn']}")
    a(f"  accuracy (agreement):    {_pct(s['accuracy'])}")
    a(f"  flag precision:          {_pct(s['flag_precision'])}")
    a(f"  flag recall:             {_pct(s['flag_recall'])}")
    a(f"  flag F1:                 {_num(s['flag_f1'])}")
    a(f"  OVER-CODING rate:        {_pct(s['over_coding_rate'])}   "
      "(FN/all — over-codes wrongly cleared; the expensive RADV error)")
    a(f"  under-flagging rate:     {_pct(s['under_flagging_rate'])}   "
      "(FN/gold-unsupported = 1 - recall)")
    a(f"  over-flagging rate:      {_pct(s['over_flagging_rate'])}   "
      "(FP/gold-supported — false alarms)")

    sy = m["system"]
    a("")
    a("-- System " + "-" * 53)
    a(f"  agreement with gold:     {_pct(sy['agreement_with_gold'])}")
    a(f"  changed by oracle loop:  {sy['changed_pairs']}")
    a(f"  disagreement-resolution: {_pct(sy['disagreement_resolution_rate'])}   "
      f"(improved {sy['oracle_improved']}, regressed {sy['oracle_regressed']})")

    if "extraction" in m:
        e = m["extraction"]
        a("")
        a("-- Extraction (HCC-level vs. candidate set) " + "-" * 20)
        a(f"  precision:               {_pct(e['precision'])}")
        a(f"  recall:                  {_pct(e['recall'])}")
        a(f"  F1:                      {_num(e['f1'])}")
        a(f"  TP/FP/FN:                {e['tp']}/{e['fp']}/{e['fn']}"
          f"   (unmapped extractions: {e['unmapped_extractions']})")

    if "spans" in m:
        sp = m["spans"]
        a("")
        a("-- Spans & hallucination " + "-" * 38)
        a(f"  span accuracy:           {_pct(sp['span_accuracy'])}"
          f"   ({sp['spans_checked']} spans)")
        a(f"  citation grounding:      {_pct(sp['citation_grounding'])}"
          f"   ({sp['citations_checked']} citations)")
        a(f"  HALLUCINATION rate:      {_pct(sp['hallucination_rate'])}"
          f"   ({sp['quotes_checked']} quotes; target 0)")
        if sp["patients_without_notes"]:
            a(f"  patients without notes:  {sp['patients_without_notes']} (skipped)")

    if "calibration" in m:
        cal = m["calibration"]
        a("")
        a(f"-- Calibration (n={cal['n']}, ECE={_num(cal['ece'])}) " + "-" * 28)
        a(f"  {'bin':>12}  {'n':>5}  {'mean_conf':>9}  {'accuracy':>8}")
        for b in cal["curve"]:
            if not b["n"]:
                continue
            lo, hi = b["bin"]
            a(f"  [{lo:.1f},{hi:.1f}]{'':>3}  {b['n']:>5}  "
              f"{_num(b['mean_confidence']):>9}  {_num(b['accuracy']):>8}")

    a("=" * 64)
    return "\n".join(L)


def render_markdown(m: dict[str, Any]) -> str:
    """Markdown summary for the docs track."""
    s, sy = m["substantiation"], m["system"]
    cf = s["confusion"]
    L: list[str] = ["# HCC Substantiation Auditor — Evaluation Metrics", ""]
    c = m["counts"]
    L += [
        f"- Patients audited: **{c['patients_audited']}**, audited HCCs: "
        f"**{c['audited_hccs']}**",
        f"- Gold labels: **{c['gold_labels']}** "
        f"(injected {c['gold_injected']}, holdout {c['gold_holdout']})",
        "",
        "## Substantiation",
        "",
        "| metric | value | definition |",
        "|---|---|---|",
        f"| accuracy | {_pct(s['accuracy'])} | agreement with gold (binary) |",
        f"| flag precision | {_pct(s['flag_precision'])} | TP/(TP+FP) |",
        f"| flag recall | {_pct(s['flag_recall'])} | TP/(TP+FN) |",
        f"| flag F1 | {_num(s['flag_f1'])} | harmonic mean |",
        f"| **over-coding rate** | **{_pct(s['over_coding_rate'])}** | "
        "FN/all — over-codes wrongly cleared (expensive RADV error) |",
        f"| under-flagging rate | {_pct(s['under_flagging_rate'])} | "
        "FN/gold-unsupported = 1−recall |",
        f"| over-flagging rate | {_pct(s['over_flagging_rate'])} | "
        "FP/gold-supported — false alarms |",
        "",
        f"Confusion: TP={cf['tp']}, FP={cf['fp']}, FN={cf['fn']}, TN={cf['tn']} "
        f"(scored pairs {s['scored_pairs']}).",
        "",
        "## System",
        "",
        f"- Agreement with gold: **{_pct(sy['agreement_with_gold'])}**",
        f"- Disagreement-resolution: **{_pct(sy['disagreement_resolution_rate'])}** "
        f"({sy['changed_pairs']} changed; improved {sy['oracle_improved']}, "
        f"regressed {sy['oracle_regressed']})",
    ]
    if "extraction" in m:
        e = m["extraction"]
        L += [
            "",
            "## Extraction (HCC-level)",
            "",
            f"- Precision **{_pct(e['precision'])}**, recall **{_pct(e['recall'])}**, "
            f"F1 **{_num(e['f1'])}** (TP/FP/FN = {e['tp']}/{e['fp']}/{e['fn']})",
        ]
    if "spans" in m:
        sp = m["spans"]
        L += [
            "",
            "## Spans & hallucination",
            "",
            f"- Span accuracy **{_pct(sp['span_accuracy'])}** ({sp['spans_checked']})",
            f"- Citation grounding **{_pct(sp['citation_grounding'])}** "
            f"({sp['citations_checked']})",
            f"- **Hallucination rate {_pct(sp['hallucination_rate'])}** "
            f"({sp['quotes_checked']} quotes; target 0)",
        ]
    if "calibration" in m:
        L += ["", f"## Calibration", "",
              f"- n={m['calibration']['n']}, ECE={_num(m['calibration']['ece'])}"]
    L.append("")
    return "\n".join(L)


def write_outputs(
    m: dict[str, Any],
    json_path: str | Path | None = None,
    md_path: str | Path | None = None,
) -> None:
    if json_path:
        Path(json_path).write_text(json.dumps(m, indent=2), encoding="utf-8")
    if md_path:
        Path(md_path).write_text(render_markdown(m), encoding="utf-8")
