"""SNOMED → ICD-10 → V28 HCC lookup, plus the V28 hierarchy collapse.

This is a faithful *replication* of the slice of the Rust ``engine`` that turns a
patient's coded conditions into the set of HCCs that would be billed:

  1. SNOMED → ICD-10 via the committed crosswalk
     (``crosswalks/snomed_to_icd10_v28.csv``).
  2. ICD-10 → HCC via the crosswalk's ``expected_v28_hcc`` column, which the
     crosswalk README documents as *verified against the same CMS V28 mapping the
     engine loads* (not hand-asserted).
  3. Hierarchy collapse via the committed V28 hierarchy table
     (``crates/engine/tests/fixtures/v28/V28_HCC_Hierarchies.csv``) — the same
     table the engine consumes — so a more-severe HCC suppresses its less-severe
     descendants (e.g. diabetes 36 ⟶ trumps 37, 38).

The ENGINE remains the authority. This module deliberately does *not* reproduce
demographics, disease interactions, or coefficients (none of which change the
candidate HCC *set*), and the derived set is meant to be reconciled against the
engine's actual output at integration (see :func:`hcceval.candidates.reconcile`).
"""
from __future__ import annotations

import csv
import re
from dataclasses import dataclass
from pathlib import Path

# Repo layout: harness/hcceval/crosswalk.py → repo root is three parents up.
REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CROSSWALK = REPO_ROOT / "crosswalks" / "snomed_to_icd10_v28.csv"
DEFAULT_HIERARCHY = (
    REPO_ROOT / "crates" / "engine" / "tests" / "fixtures" / "v28"
    / "V28_HCC_Hierarchies.csv"
)


def normalize_icd10(code: str) -> str:
    """Dotless, uppercase ICD-10 — matching ``engine::types::normalize_icd10``."""
    return re.sub(r"[^A-Za-z0-9]", "", code).upper()


@dataclass(frozen=True)
class CrosswalkEntry:
    snomed: str
    snomed_description: str
    icd10: str  # normalized (dotless, upper)
    icd10_description: str
    hcc: int | None  # None == clinical near-miss (documented, not HCC-eligible)
    note: str


class Crosswalk:
    """The SNOMED→ICD-10→HCC crosswalk, plus V28 hierarchy collapse."""

    def __init__(
        self,
        entries: list[CrosswalkEntry],
        hierarchy: dict[int, set[int]] | None = None,
    ):
        self._by_snomed: dict[str, CrosswalkEntry] = {e.snomed: e for e in entries}
        self._icd10_to_hcc: dict[str, int | None] = {}
        self._icd10_desc: dict[str, str] = {}
        for e in entries:
            self._icd10_to_hcc[e.icd10] = e.hcc
            self._icd10_desc[e.icd10] = e.icd10_description
        # hierarchy[h] = set of HCCs that h suppresses (its descendants).
        self.hierarchy: dict[int, set[int]] = hierarchy or {}

    # -- loading ----------------------------------------------------------- #
    @classmethod
    def load(
        cls,
        crosswalk_path: str | Path = DEFAULT_CROSSWALK,
        hierarchy_path: str | Path | None = DEFAULT_HIERARCHY,
    ) -> "Crosswalk":
        entries: list[CrosswalkEntry] = []
        with open(crosswalk_path, newline="", encoding="utf-8") as f:
            for row in csv.DictReader(f):
                raw_hcc = (row.get("expected_v28_hcc") or "").strip()
                hcc = int(raw_hcc) if raw_hcc else None
                entries.append(
                    CrosswalkEntry(
                        snomed=row["snomed_code"].strip(),
                        snomed_description=row.get("snomed_description", "").strip(),
                        icd10=normalize_icd10(row["icd10"]),
                        icd10_description=row.get("icd10_description", "").strip(),
                        hcc=hcc,
                        note=row.get("note", "").strip(),
                    )
                )
        hierarchy = (
            cls._load_hierarchy(hierarchy_path)
            if hierarchy_path and Path(hierarchy_path).exists()
            else {}
        )
        return cls(entries, hierarchy)

    @staticmethod
    def _load_hierarchy(path: str | Path) -> dict[int, set[int]]:
        """Parse V28_HCC_Hierarchies.csv → {hcc: {suppressed descendants}}.

        Rows look like ``HCC36,HCC37,HCC38,,,,`` meaning HCC 36 trumps 37 and 38.
        """
        out: dict[int, set[int]] = {}
        with open(path, newline="", encoding="utf-8") as f:
            reader = csv.reader(f)
            header = next(reader, None)  # HCC,SecondaryHCC_1,...
            for row in reader:
                if not row or not row[0].strip():
                    continue
                parent = _hcc_num(row[0])
                if parent is None:
                    continue
                kids = {n for cell in row[1:] if (n := _hcc_num(cell)) is not None}
                out[parent] = kids
        return out

    # -- lookups ----------------------------------------------------------- #
    def icd10_for_snomed(self, snomed: str) -> str | None:
        e = self._by_snomed.get(str(snomed).strip())
        return e.icd10 if e else None

    def hcc_for_icd10(self, icd10: str) -> int | None:
        return self._icd10_to_hcc.get(normalize_icd10(icd10))

    def describe_icd10(self, icd10: str) -> str:
        return self._icd10_desc.get(normalize_icd10(icd10), "")

    def in_scope_icd10(self, icd10: str) -> bool:
        """Whether this ICD-10 is one the harness recognizes (mappable)."""
        return normalize_icd10(icd10) in self._icd10_to_hcc

    # -- hierarchy --------------------------------------------------------- #
    def collapse(self, hccs: set[int]) -> set[int]:
        """Apply V28 hierarchy: drop any HCC suppressed by a more-severe present HCC."""
        suppressed: set[int] = set()
        for h in hccs:
            suppressed |= self.hierarchy.get(h, set()) & hccs
        return hccs - suppressed


def _hcc_num(cell: str) -> int | None:
    cell = (cell or "").strip().upper()
    if not cell:
        return None
    m = re.match(r"HCC?\s*0*(\d+)", cell)
    if m:
        return int(m.group(1))
    return int(cell) if cell.isdigit() else None
