#!/usr/bin/env python3
"""Generate the controlled error-injection eval slice: known-truth cases.

Writes synthetic Synthea-style FHIR bundles (one per case), a candidates JSONL,
and a gold-labels JSONL. Each target family gets a matched supported case (note
documents full M.E.A.T.) and an unsupported case (code present, note silent) so
the eval measures real RADV catching rather than code-echoing. A configurable
slice is flagged ``holdout`` (never for tuning).

Example:
  inject_errors.py --fhir-out fixtures/notes --candidates-out fixtures/candidates.jsonl \\
                   --gold-out fixtures/gold_labels.jsonl
"""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hcceval import schema  # noqa: E402
from hcceval.inject import build_injection_set, write_injection_set  # noqa: E402


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--fhir-out", required=True, help="dir for synthetic FHIR bundles")
    p.add_argument("--candidates-out", required=True, help="candidates JSONL")
    p.add_argument("--gold-out", required=True, help="gold labels JSONL")
    p.add_argument("--holdout-every", type=int, default=2,
                   help="flag every Nth gold label as holdout (0 = none)")
    args = p.parse_args()

    inj = build_injection_set(holdout_every=args.holdout_every)
    written = write_injection_set(inj, args.fhir_out)
    schema.write_jsonl(args.candidates_out, inj.candidates)
    schema.write_jsonl(args.gold_out, inj.gold)

    n_sup = sum(1 for g in inj.gold if g.supported)
    n_uns = len(inj.gold) - n_sup
    n_hold = sum(1 for g in inj.gold if g.holdout)
    print(f"wrote {len(written)} FHIR bundles -> {args.fhir_out}", file=sys.stderr)
    print(f"wrote {len(inj.candidates)} candidates -> {args.candidates_out}",
          file=sys.stderr)
    print(f"wrote {len(inj.gold)} gold labels -> {args.gold_out} "
          f"(supported {n_sup}, unsupported {n_uns}, holdout {n_hold})",
          file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
