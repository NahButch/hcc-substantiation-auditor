#!/usr/bin/env python3
"""Compute the Phase 3c evaluation metrics and emit the report.

Joins the agent's ``audit_results.jsonl`` with gold labels (and, when available,
the engine-eligible candidate set and the source FHIR notes) and prints the
metrics table to stdout. Optionally writes JSON and Markdown summaries.

Examples:
  # fixtures (self-contained, runs offline)
  eval.py --audit fixtures/audit_results.jsonl --gold fixtures/gold_labels.jsonl \\
          --candidates fixtures/candidates.jsonl --fhir fixtures/notes

  # real run, post-merge
  eval.py --audit out/audit_results.jsonl --gold labels/gold.jsonl \\
          --candidates out/candidates.jsonl \\
          --fhir /home/tom_b/code/hcc-substantiation-auditor/data/synthea/cohort/fhir \\
          --json out/metrics.json --md out/metrics.md
"""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hcceval import report, schema  # noqa: E402
from hcceval.crosswalk import DEFAULT_CROSSWALK, DEFAULT_HIERARCHY, Crosswalk  # noqa: E402
from hcceval.metrics import EvalInputs, compute_all  # noqa: E402
from hcceval.notes import load_notes_dir  # noqa: E402


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--audit", required=True, help="agent audit_results.jsonl")
    p.add_argument("--gold", required=True, help="gold labels JSONL")
    p.add_argument("--candidates", help="engine-eligible candidates JSONL (extraction metrics)")
    p.add_argument("--fhir", help="FHIR bundle dir (span + hallucination metrics)")
    p.add_argument("--crosswalk", default=str(DEFAULT_CROSSWALK))
    p.add_argument("--exclude-holdout", action="store_true",
                   help="exclude the held-out (never-for-tuning) slice from scoring")
    p.add_argument("--json", help="write metrics JSON here")
    p.add_argument("--md", help="write Markdown summary here")
    args = p.parse_args()

    audits = schema.load_audits(args.audit)
    gold = schema.load_gold(args.gold)
    candidates = schema.load_candidates(args.candidates) if args.candidates else []
    notes = load_notes_dir(args.fhir) if args.fhir else {}
    xwalk = Crosswalk.load(args.crosswalk, DEFAULT_HIERARCHY) if candidates else None

    metrics = compute_all(EvalInputs(
        audits=audits,
        gold=gold,
        candidates=candidates,
        notes=notes,
        xwalk=xwalk,
        include_holdout=not args.exclude_holdout,
    ))

    print(report.render_text(metrics))
    report.write_outputs(metrics, args.json, args.md)
    if args.json:
        print(f"\nwrote {args.json}", file=sys.stderr)
    if args.md:
        print(f"wrote {args.md}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
