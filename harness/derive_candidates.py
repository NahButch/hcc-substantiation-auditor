#!/usr/bin/env python3
"""Derive each patient's engine-eligible HCC candidate set from the cohort CSV.

Reads Synthea ``conditions.csv``, maps SNOMED → ICD-10 → V28 HCC via the committed
crosswalk, applies the V28 hierarchy collapse, and writes candidate records
(JSONL). Optionally reconciles the derived set against the engine's authoritative
output in an ``audit_results.jsonl`` (the engine remains the source of truth).

Examples:
  derive_candidates.py --conditions DATA/csv/conditions.csv --out candidates.jsonl
  derive_candidates.py --conditions ... --reconcile audit_results.jsonl
"""
import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hcceval import schema  # noqa: E402
from hcceval.candidates import (  # noqa: E402
    cohort_patient_ids,
    derive_candidates,
    reconcile,
)
from hcceval.crosswalk import DEFAULT_CROSSWALK, DEFAULT_HIERARCHY, Crosswalk  # noqa: E402

DEFAULT_COHORT = "/home/tom_b/code/hcc-substantiation-auditor/data/synthea/cohort/csv"


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--conditions", default=f"{DEFAULT_COHORT}/conditions.csv",
                   help="Synthea conditions.csv")
    p.add_argument("--patients", default=f"{DEFAULT_COHORT}/patients.csv",
                   help="Synthea patients.csv (optional, restricts to cohort IDs)")
    p.add_argument("--crosswalk", default=str(DEFAULT_CROSSWALK))
    p.add_argument("--hierarchy", default=str(DEFAULT_HIERARCHY))
    p.add_argument("--out", default="-", help="output JSONL ('-' = stdout)")
    p.add_argument("--reconcile", metavar="AUDIT_JSONL",
                   help="cross-check derived set against engine output")
    args = p.parse_args()

    xwalk = Crosswalk.load(args.crosswalk, args.hierarchy)
    pids = cohort_patient_ids(args.patients) if Path(args.patients).exists() else None
    cands = derive_candidates(args.conditions, xwalk, pids)

    n_patients = len({c.patient_id for c in cands})
    print(f"derived {len(cands)} candidate HCC(s) across {n_patients} patient(s)",
          file=sys.stderr)

    if args.out == "-":
        for c in cands:
            print(json.dumps({"patient_id": c.patient_id, "hcc": c.hcc,
                              "triggering_icd10": c.triggering_icd10,
                              "triggering_snomed": c.triggering_snomed}))
    else:
        schema.write_jsonl(args.out, cands)
        print(f"wrote {args.out}", file=sys.stderr)

    if args.reconcile:
        audits = schema.load_audits(args.reconcile)
        r = reconcile(cands, audits)
        print(f"\nreconcile vs engine ({r.patients_compared} patients in common):",
              file=sys.stderr)
        print(f"  agree: {r.agree}   only-derived (over-list): {len(r.only_derived)}"
              f"   only-engine (under-list): {len(r.only_engine)}", file=sys.stderr)
        if not r.exact:
            for pid, hcc in (r.only_derived + r.only_engine)[:20]:
                side = "derived-only" if (pid, hcc) in set(r.only_derived) else "engine-only"
                print(f"    {side}: {pid} HCC {hcc}", file=sys.stderr)
            return 1
        print("  EXACT match with engine.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
