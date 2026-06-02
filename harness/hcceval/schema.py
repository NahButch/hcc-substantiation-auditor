"""Data model + JSONL I/O for the evaluation harness.

Three record types cross the harness boundary:

  * :class:`AuditRecord` — one per patient, the agent track's output
    (``audit_results.jsonl``). This mirrors the *Shared INPUT contract*; we code
    against it directly and tolerate missing optional fields so the harness keeps
    working as the agent track fills the contract out.
  * :class:`GoldLabel` — one per (patient, HCC) substantiation judgment, the
    ground truth the metrics are scored against.
  * :class:`Candidate` — one per (patient, HCC), the engine-eligible coded HCCs
    that *should* be labeled (produced mechanically; see :mod:`hcceval.candidates`).

All three are persisted as JSONL (one JSON object per line). Loading is lenient
about unknown keys (forward-compatible with the evolving agent contract) but
strict about the few fields the metrics genuinely depend on.
"""
from __future__ import annotations

import json
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Iterable, Iterator

# Substantiation status vocabulary. The agent emits a 3-way status; gold labels
# are binary (a code is either documented to CMS standards or it is not).
AUDIT_STATUSES = ("supported", "risky", "unsupported")
GOLD_STATUSES = ("supported", "unsupported")
GOLD_SOURCES = ("manual", "injected")


def _supported(status: str) -> bool:
    """True iff a status string denotes a substantiated code.

    The auditor's 3-way status collapses to a binary decision for scoring:
    only an explicit ``supported`` clears the code; ``risky`` and
    ``unsupported`` both mean *flagged* (the auditor is asking for review).
    """
    return status.strip().lower() == "supported"


# --------------------------------------------------------------------------- #
# Audit records (agent-track output; the metrics INPUT contract)
# --------------------------------------------------------------------------- #
@dataclass
class ExtractedCondition:
    name: str
    evidence: str = ""
    section: str = ""
    icd10: str | None = None

    @classmethod
    def from_obj(cls, o: dict[str, Any]) -> "ExtractedCondition":
        return cls(
            name=str(o.get("name", "")),
            evidence=str(o.get("evidence", "") or ""),
            section=str(o.get("section", "") or ""),
            icd10=(o["icd10"] if o.get("icd10") else None),
        )


@dataclass
class HccAudit:
    hcc: int
    coefficient: float = 0.0
    triggering_diagnoses: list[str] = field(default_factory=list)
    initial_status: str = ""
    final_status: str = ""
    corrected: bool = False
    correction_reason: str = ""
    meat_present: list[str] = field(default_factory=list)
    specificity_supported: bool = False
    citation: str = ""
    documentation_gap: str = ""
    confidence: float | None = None

    @classmethod
    def from_obj(cls, o: dict[str, Any]) -> "HccAudit":
        # The contract carries both initial_status and final_status. Older /
        # single-pass output may only carry a single "status"; treat it as final.
        final = o.get("final_status") or o.get("status") or ""
        initial = o.get("initial_status") or final
        conf = o.get("confidence", None)
        return cls(
            hcc=int(o["hcc"]),
            coefficient=float(o.get("coefficient", 0.0) or 0.0),
            triggering_diagnoses=list(o.get("triggering_diagnoses", []) or []),
            initial_status=str(initial),
            final_status=str(final),
            corrected=bool(o.get("corrected", initial and final and initial != final)),
            correction_reason=str(o.get("correction_reason", "") or ""),
            meat_present=list(o.get("meat_present", []) or []),
            specificity_supported=bool(o.get("specificity_supported", False)),
            citation=str(o.get("citation", "") or ""),
            documentation_gap=str(o.get("documentation_gap", "") or ""),
            confidence=(float(conf) if conf is not None else None),
        )

    @property
    def final_supported(self) -> bool:
        return _supported(self.final_status)

    @property
    def initial_supported(self) -> bool:
        return _supported(self.initial_status)

    @property
    def changed(self) -> bool:
        """The oracle loop moved this code across the supported/flagged line."""
        return self.initial_status and self.final_status and (
            self.initial_supported != self.final_supported
        )


@dataclass
class AuditRecord:
    patient_id: str
    age: int = 0
    segment: str = ""
    raw_score: float = 0.0
    extracted: list[ExtractedCondition] = field(default_factory=list)
    hccs: list[HccAudit] = field(default_factory=list)

    @classmethod
    def from_obj(cls, o: dict[str, Any]) -> "AuditRecord":
        return cls(
            patient_id=str(o["patient_id"]),
            age=int(o.get("age", 0) or 0),
            segment=str(o.get("segment", "") or ""),
            raw_score=float(o.get("raw_score", 0.0) or 0.0),
            extracted=[ExtractedCondition.from_obj(e) for e in o.get("extracted", [])],
            hccs=[HccAudit.from_obj(h) for h in o.get("hccs", [])],
        )


# --------------------------------------------------------------------------- #
# Gold labels (substantiation ground truth)
# --------------------------------------------------------------------------- #
@dataclass
class GoldLabel:
    patient_id: str
    hcc: int
    gold_status: str  # "supported" | "unsupported"
    source: str = "manual"  # "manual" | "injected"
    rationale: str = ""
    # Held-out slice flag: labels marked True must never inform prompt/threshold
    # tuning — they exist only to measure generalization. Optional extension to
    # the base contract; defaults False.
    holdout: bool = False

    @classmethod
    def from_obj(cls, o: dict[str, Any]) -> "GoldLabel":
        status = str(o["gold_status"]).strip().lower()
        if status not in GOLD_STATUSES:
            raise ValueError(
                f"gold_status must be one of {GOLD_STATUSES}, got {status!r} "
                f"(patient {o.get('patient_id')}, hcc {o.get('hcc')})"
            )
        src = str(o.get("source", "manual")).strip().lower()
        if src not in GOLD_SOURCES:
            raise ValueError(f"source must be one of {GOLD_SOURCES}, got {src!r}")
        return cls(
            patient_id=str(o["patient_id"]),
            hcc=int(o["hcc"]),
            gold_status=status,
            source=src,
            rationale=str(o.get("rationale", "") or ""),
            holdout=bool(o.get("holdout", False)),
        )

    @property
    def supported(self) -> bool:
        return self.gold_status == "supported"

    @property
    def key(self) -> tuple[str, int]:
        return (self.patient_id, self.hcc)


# --------------------------------------------------------------------------- #
# Candidate records (engine-eligible HCCs to be labeled)
# --------------------------------------------------------------------------- #
@dataclass
class Candidate:
    patient_id: str
    hcc: int
    triggering_icd10: list[str] = field(default_factory=list)
    triggering_snomed: list[str] = field(default_factory=list)

    @classmethod
    def from_obj(cls, o: dict[str, Any]) -> "Candidate":
        return cls(
            patient_id=str(o["patient_id"]),
            hcc=int(o["hcc"]),
            triggering_icd10=list(o.get("triggering_icd10", []) or []),
            triggering_snomed=list(o.get("triggering_snomed", []) or []),
        )

    @property
    def key(self) -> tuple[str, int]:
        return (self.patient_id, self.hcc)


# --------------------------------------------------------------------------- #
# JSONL helpers
# --------------------------------------------------------------------------- #
def read_jsonl(path: str | Path) -> Iterator[dict[str, Any]]:
    """Yield JSON objects from a JSONL file, skipping blank lines."""
    with open(path, "r", encoding="utf-8") as f:
        for n, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                yield json.loads(line)
            except json.JSONDecodeError as e:
                raise ValueError(f"{path}:{n}: invalid JSON: {e}") from e


def write_jsonl(path: str | Path, rows: Iterable[Any]) -> int:
    """Write dataclass instances or dicts to JSONL. Returns the row count."""
    n = 0
    with open(path, "w", encoding="utf-8") as f:
        for r in rows:
            obj = asdict(r) if hasattr(r, "__dataclass_fields__") else r
            f.write(json.dumps(obj, ensure_ascii=False) + "\n")
            n += 1
    return n


def load_audits(path: str | Path) -> list[AuditRecord]:
    return [AuditRecord.from_obj(o) for o in read_jsonl(path)]


def load_gold(path: str | Path) -> list[GoldLabel]:
    labels = [GoldLabel.from_obj(o) for o in read_jsonl(path)]
    seen: dict[tuple[str, int], GoldLabel] = {}
    for g in labels:
        if g.key in seen:
            raise ValueError(f"duplicate gold label for {g.key}")
        seen[g.key] = g
    return labels


def load_candidates(path: str | Path) -> list[Candidate]:
    return [Candidate.from_obj(o) for o in read_jsonl(path)]
