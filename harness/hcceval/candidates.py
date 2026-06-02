"""Mechanical truth helper: derive each patient's engine-eligible HCC set.

Reads the Synthea cohort ``conditions.csv``, maps each SNOMED condition through
the committed crosswalk to an ICD-10 code and then to a V28 HCC, and applies the
V28 hierarchy collapse — producing, per patient, the *candidate set* of coded
HCCs that should be labeled for substantiation.

The ENGINE is the authority for the coded HCC set. This replication exists so the
eval set can be scoped and a labeling worksheet built *before* the agent track
emits ``audit_results.jsonl``; at integration, :func:`reconcile` cross-checks the
derived set against the engine's actual output and reports any drift.
"""
from __future__ import annotations

import csv
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

from .crosswalk import Crosswalk
from .schema import AuditRecord, Candidate

SNOMED_SYSTEM = "http://snomed.info/sct"


def derive_candidates(
    conditions_csv: str | Path,
    xwalk: Crosswalk,
    patient_ids: set[str] | None = None,
) -> list[Candidate]:
    """Per-patient engine-eligible HCC candidates from cohort ``conditions.csv``.

    ``conditions.csv`` columns: START,STOP,PATIENT,ENCOUNTER,SYSTEM,CODE,DESCRIPTION.
    Conditions with no crosswalk entry, or whose ICD-10 maps to no HCC (clinical
    near-misses), are dropped from the candidate set by design. If ``patient_ids``
    is given, only those patients are considered.
    """
    # patient -> hcc -> (icd10s, snomeds) that triggered it (pre-hierarchy)
    triggers: dict[str, dict[int, tuple[set[str], set[str]]]] = defaultdict(
        lambda: defaultdict(lambda: (set(), set()))
    )
    with open(conditions_csv, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            if (row.get("SYSTEM") or "").strip() != SNOMED_SYSTEM:
                continue
            pid = (row.get("PATIENT") or "").strip()
            if patient_ids is not None and pid not in patient_ids:
                continue
            snomed = (row.get("CODE") or "").strip()
            icd10 = xwalk.icd10_for_snomed(snomed)
            if icd10 is None:
                continue  # out of scope (not a target-family SNOMED code)
            hcc = xwalk.hcc_for_icd10(icd10)
            if hcc is None:
                continue  # documented near-miss (no V28 HCC) — not a coded candidate
            icds, snomeds = triggers[pid][hcc]
            icds.add(icd10)
            snomeds.add(snomed)

    out: list[Candidate] = []
    for pid, by_hcc in triggers.items():
        kept = xwalk.collapse(set(by_hcc))  # apply V28 hierarchy
        for hcc in sorted(kept):
            icds, snomeds = by_hcc[hcc]
            out.append(
                Candidate(
                    patient_id=pid,
                    hcc=hcc,
                    triggering_icd10=sorted(icds),
                    triggering_snomed=sorted(snomeds),
                )
            )
    out.sort(key=lambda c: (c.patient_id, c.hcc))
    return out


def candidates_by_patient(cands: list[Candidate]) -> dict[str, set[int]]:
    by: dict[str, set[int]] = defaultdict(set)
    for c in cands:
        by[c.patient_id].add(c.hcc)
    return dict(by)


@dataclass
class ReconcileResult:
    """Drift between the mechanically-derived candidate set and the engine output."""
    patients_compared: int
    agree: int                       # (patient, hcc) pairs present in both
    only_derived: list[tuple[str, int]]   # we listed it; engine did not (over-list)
    only_engine: list[tuple[str, int]]    # engine billed it; we missed (under-list)

    @property
    def exact(self) -> bool:
        return not self.only_derived and not self.only_engine


def reconcile(
    cands: list[Candidate],
    audits: list[AuditRecord],
) -> ReconcileResult:
    """Cross-check derived candidates against the engine's coded HCCs in the audit.

    The agent track derives its ``hccs[]`` from the engine oracle, so this verifies
    that the harness's standalone replication matches the authoritative engine for
    every patient that appears in both inputs.
    """
    derived = candidates_by_patient(cands)
    engine: dict[str, set[int]] = {a.patient_id: {h.hcc for h in a.hccs} for a in audits}
    common = set(derived) & set(engine)
    agree = 0
    only_d: list[tuple[str, int]] = []
    only_e: list[tuple[str, int]] = []
    for pid in sorted(common):
        d, e = derived[pid], engine[pid]
        agree += len(d & e)
        only_d += [(pid, h) for h in sorted(d - e)]
        only_e += [(pid, h) for h in sorted(e - d)]
    return ReconcileResult(len(common), agree, only_d, only_e)


def cohort_patient_ids(patients_csv: str | Path) -> set[str]:
    """All patient IDs in the cohort ``patients.csv`` (column ``Id``)."""
    with open(patients_csv, newline="", encoding="utf-8") as f:
        return {(row.get("Id") or "").strip() for row in csv.DictReader(f)}


def _eprint(*a: object) -> None:
    print(*a, file=sys.stderr)
