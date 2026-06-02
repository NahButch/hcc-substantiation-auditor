"""Human labeling worksheet — a CSV a reviewer fills to produce gold labels.

One row per (patient, engine-eligible HCC) candidate. The reviewer reads the
patient's note and records a substantiation judgment anchored to the CMS V28
M.E.A.T. expectations (Monitor / Evaluate / Assess / Treat) plus diagnostic
specificity. The filled sheet converts back to gold-label JSONL via
:func:`worksheet_to_gold`.
"""
from __future__ import annotations

import csv
from pathlib import Path

from .crosswalk import Crosswalk
from .schema import Candidate, GoldLabel

# Columns: provenance the reviewer needs, the M.E.A.T. checkboxes that anchor the
# call, and the decision fields that map to a GoldLabel.
COLUMNS = [
    "patient_id",
    "hcc",
    "triggering_icd10",
    "triggering_snomed",
    "hcc_description",
    # --- M.E.A.T. anchors (reviewer marks Y/N from the note) -------------- #
    "meat_monitor",       # condition status/progress monitored?
    "meat_evaluate",      # test results / exam findings evaluated?
    "meat_assess",        # condition explicitly assessed/addressed?
    "meat_treat",         # treatment / management documented?
    "specificity_ok",     # documentation supports the coded specificity?
    # --- decision (maps to GoldLabel) ------------------------------------- #
    "gold_status",        # supported | unsupported
    "holdout",            # TRUE to reserve from tuning
    "rationale",          # free-text justification (required for unsupported)
]

INSTRUCTIONS = (
    "Fill one row per coded HCC. Read the patient note. Mark each M.E.A.T. anchor "
    "Y/N (Monitor, Evaluate, Assess, Treat) and whether the documentation supports "
    "the coded specificity. Then set gold_status to 'supported' only if the note "
    "documents the condition to CMS V28 standards (generally >=1 M.E.A.T. element "
    "clearly tied to the coded condition in the service period); otherwise "
    "'unsupported'. Set holdout=TRUE for the reserved slice. Give a rationale for "
    "every 'unsupported' (and ideally all) rows."
)


def build_worksheet(
    candidates: list[Candidate],
    xwalk: Crosswalk,
    path: str | Path,
    include_instructions: bool = True,
) -> int:
    """Write a blank worksheet (one row per candidate). Returns the row count."""
    rows = sorted(candidates, key=lambda c: (c.patient_id, c.hcc))
    with open(path, "w", newline="", encoding="utf-8") as f:
        if include_instructions:
            f.write(f"# {INSTRUCTIONS}\n")
        w = csv.DictWriter(f, fieldnames=COLUMNS)
        w.writeheader()
        for c in rows:
            # Resolve a human-readable HCC description from the first triggering code.
            desc = ""
            for icd in c.triggering_icd10:
                desc = xwalk.describe_icd10(icd)
                if desc:
                    break
            w.writerow(
                {
                    "patient_id": c.patient_id,
                    "hcc": c.hcc,
                    "triggering_icd10": ";".join(c.triggering_icd10),
                    "triggering_snomed": ";".join(c.triggering_snomed),
                    "hcc_description": desc,
                    "meat_monitor": "",
                    "meat_evaluate": "",
                    "meat_assess": "",
                    "meat_treat": "",
                    "specificity_ok": "",
                    "gold_status": "",
                    "holdout": "",
                    "rationale": "",
                }
            )
    return len(rows)


def _truthy(v: str) -> bool:
    return str(v).strip().lower() in ("y", "yes", "true", "1", "t")


def worksheet_to_gold(path: str | Path) -> list[GoldLabel]:
    """Parse a filled worksheet back into gold labels (rows with a gold_status).

    Comment lines (starting ``#``) and rows with a blank ``gold_status`` are
    skipped, so a partially-filled sheet still yields the rows that are done.
    """
    out: list[GoldLabel] = []
    with open(path, newline="", encoding="utf-8") as f:
        lines = [ln for ln in f if not ln.lstrip().startswith("#")]
    for row in csv.DictReader(lines):
        status = (row.get("gold_status") or "").strip().lower()
        if not status:
            continue
        out.append(
            GoldLabel(
                patient_id=(row["patient_id"]).strip(),
                hcc=int(row["hcc"]),
                gold_status=status,
                source="manual",
                rationale=(row.get("rationale") or "").strip(),
                holdout=_truthy(row.get("holdout", "")),
            )
        )
    return out
