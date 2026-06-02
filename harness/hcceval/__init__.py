"""hcceval — evaluation harness for the HCC Substantiation Auditor (Phase 3b/3c).

Two halves:
  * the *evaluation set* tooling (3b): gold-label schema, a mechanical
    engine-eligible-HCC candidate deriver, controlled error injection, and a
    human labeling worksheet; and
  * the *metrics* harness (3c): extraction, substantiation, and system-level
    metrics computed from the agent's ``audit_results.jsonl`` plus gold labels
    and the source clinical notes.

Everything is synthetic-data only and pinned to CMS-HCC model V28, scoped to the
four target families (diabetes, CKD, heart failure, COPD).
"""

__all__ = [
    "schema",
    "crosswalk",
    "candidates",
    "notes",
    "inject",
    "worksheet",
    "metrics",
    "report",
]
