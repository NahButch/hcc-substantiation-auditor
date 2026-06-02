#!/usr/bin/env python3
"""Scaffold a human labeling worksheet (CSV) from a candidates file.

One row per (patient, engine-eligible HCC), with M.E.A.T. anchor columns the
reviewer fills to decide gold_status. Convert a filled sheet back to gold-label
JSONL with --to-gold.

Examples:
  make_worksheet.py --candidates candidates.jsonl --out worksheet.csv
  make_worksheet.py --to-gold worksheet.csv --out gold_labels.jsonl
"""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hcceval import schema  # noqa: E402
from hcceval.crosswalk import DEFAULT_CROSSWALK, DEFAULT_HIERARCHY, Crosswalk  # noqa: E402
from hcceval.worksheet import build_worksheet, worksheet_to_gold  # noqa: E402


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--candidates", help="candidates JSONL (build a blank sheet)")
    p.add_argument("--to-gold", metavar="WORKSHEET_CSV",
                   help="convert a filled worksheet back to gold-label JSONL")
    p.add_argument("--crosswalk", default=str(DEFAULT_CROSSWALK))
    p.add_argument("--out", required=True, help="output path (CSV or JSONL)")
    args = p.parse_args()

    if args.to_gold:
        gold = worksheet_to_gold(args.to_gold)
        schema.write_jsonl(args.out, gold)
        print(f"wrote {len(gold)} gold labels -> {args.out}", file=sys.stderr)
        return 0

    if not args.candidates:
        p.error("provide --candidates (to build a sheet) or --to-gold (to convert one)")
    xwalk = Crosswalk.load(args.crosswalk, DEFAULT_HIERARCHY)
    cands = schema.load_candidates(args.candidates)
    n = build_worksheet(cands, xwalk, args.out)
    print(f"wrote worksheet with {n} rows -> {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
