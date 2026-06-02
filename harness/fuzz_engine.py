#!/usr/bin/env python3
"""Differential fuzzer: Rust engine vs. the CMS V28 Python reference.

Generates N random synthetic beneficiaries — random ICD-10 sets drawn from the
FULL package mapping (every HCC, not just the four target families), random age,
sex, OREC, and Medicaid flags — runs both implementations, and asserts the risk
scores agree to 3 decimals on each beneficiary's derived community segment.

This exercises the whole implemented surface at once: mapping (incl. MCE/age/sex
edits), hierarchy, disease interactions, demographic flags (ORIGDIS /
OriginallyDisabled / LTIMCAID / DISABLED_* interactions), and the D1–D10P count
buckets — across the COMMUNITY_NA (aged) and COMMUNITY_ND (disabled) segments
that `score_csv` derives (non-dual). The partial/full-dual columns differ only by
coefficient-column selection over the SAME variable vector, so NA+ND coverage
validates the scoring logic; New-Enrollee and institutional models are out of the
engine's v1 scope and not compared.

Synthetic inputs only. Seeded for reproducibility.

Run:  .venv-ref/bin/python harness/fuzz_engine.py [N] [SEED]
Exit 0 = all agree.
"""
import csv
import io
import random
import subprocess
import sys
from datetime import date
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PKG = REPO / "data/cms_hcc_v28/python_v28"
TABLES = PKG / "software/CMS_HCC_v28/data/input/internal"
REF_INPUT = PKG / "software/CMS_HCC_v28/data/input/user_defined"
REF_OUTPUT = PKG / "software/CMS_HCC_v28/data/output/CMS_HCC_v28_2026_T_scores.csv"
MAPPING = TABLES / "ICD10_CC_mappings_CMS_HCC_2026_v28.csv"
SCORE_CSV = REPO / "target/debug/examples/score_csv"

N = int(sys.argv[1]) if len(sys.argv) > 1 else 400
SEED = int(sys.argv[2]) if len(sys.argv) > 2 else 1234


def icd10_universe():
    with open(MAPPING, encoding="utf-8-sig", newline="") as f:
        return sorted({r["ICD10"].strip() for r in csv.DictReader(f) if r["ICD10"].strip()})


def gen_benes(rng, universe):
    benes = []
    for i in range(1, N + 1):
        m, d, y = rng.randint(1, 12), rng.randint(1, 28), rng.randint(1916, 2026)
        k = rng.randint(0, 10)
        codes = rng.sample(universe, k) if k else []
        benes.append({
            "ID": str(i),
            "DOB": f"{m}/{d}/{y}",
            "SEX": rng.choice([1, 2]),
            "OREC": rng.choice([0, 1, 2, 3]),
            "LTIMCAID": rng.choice([0, 1]),
            "NEMCAID": rng.choice([0, 1]),
            "codes": codes,
        })
    return benes


def write_inputs(benes):
    REF_INPUT.mkdir(parents=True, exist_ok=True)
    with open(REF_INPUT / "beneficiaries.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["ID", "DOB", "SEX", "OREC", "LTIMCAID", "NEMCAID"])
        for b in benes:
            w.writerow([b["ID"], b["DOB"], b["SEX"], b["OREC"], b["LTIMCAID"], b["NEMCAID"]])
    with open(REF_INPUT / "diagnoses.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["ID", "ICD10"])
        for b in benes:
            for c in b["codes"]:
                w.writerow([b["ID"], c])


def run_reference():
    r = subprocess.run([sys.executable, "./software/CMS_HCC_v28/transform.py"],
                       cwd=PKG, capture_output=True, text=True)
    if r.returncode != 0:
        print("reference FAILED:\n", r.stdout, r.stderr); sys.exit(1)
    out = {}
    with open(REF_OUTPUT, newline="") as f:
        for row in csv.DictReader(f):
            out[str(row["ID"])] = row
    return out


def run_engine():
    r = subprocess.run([str(SCORE_CSV), str(TABLES), str(REF_INPUT / "beneficiaries.csv"),
                        str(REF_INPUT / "diagnoses.csv")], capture_output=True, text=True)
    if r.returncode != 0:
        print("engine FAILED:\n", r.stdout, r.stderr); sys.exit(1)
    return {row["ID"]: (row["SEGMENT"], float(row["SCORE"]))
            for row in csv.DictReader(io.StringIO(r.stdout))}


def main():
    rng = random.Random(SEED)
    universe = icd10_universe()
    print(f"fuzzing N={N} benes, seed={SEED}, ICD-10 universe={len(universe)} codes")
    benes = gen_benes(rng, universe)
    by_id = {b["ID"]: b for b in benes}
    write_inputs(benes)
    ref = run_reference()
    eng = run_engine()

    mism = 0
    for bid, (seg, eng_score) in eng.items():
        ref_score = round(float(ref[bid][f"SCORE_{seg}"]), 3)
        if abs(ref_score - eng_score) >= 1e-9:
            mism += 1
            if mism <= 15:
                b = by_id[bid]
                print(f"  MISMATCH id={bid} seg={seg} ref={ref_score} eng={eng_score} "
                      f"| DOB={b['DOB']} SEX={b['SEX']} OREC={b['OREC']} "
                      f"LTIMCAID={b['LTIMCAID']} codes={b['codes']}")
    print("-" * 60)
    if mism:
        print(f"FAIL: {mism}/{len(eng)} beneficiaries disagree.")
        sys.exit(1)
    print(f"PASS: all {len(eng)} random beneficiaries agree to 3 decimals with the CMS reference.")


if __name__ == "__main__":
    main()
