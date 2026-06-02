#!/usr/bin/env python3
"""Cross-validate the Rust engine against the CMS V28 *reference* implementation.

Runs an identical battery of synthetic beneficiaries through:
  1. CMS's own published Python reference software (transform.py), and
  2. our Rust `engine` (via the `score_csv` example),
then asserts the risk scores agree to 3 decimals on each beneficiary's community
continuing-enrollee segment (COMMUNITY_NA for aged, COMMUNITY_ND for disabled).

Because the engine is pointed at the FULL CMS tables here (not the test subset),
this validates the complete V28 model, including HCCs outside the project's target
families. Synthetic inputs only — no real patient data.

Run:  .venv-ref/bin/python harness/crosscheck.py
Exit code 0 = full agreement.
"""
import csv
import io
import subprocess
import sys
from datetime import date
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PKG = REPO / "data/cms_hcc_v28/python_v28"
PKG_BASE = PKG  # transform.py uses paths relative to here
TABLES = PKG / "software/CMS_HCC_v28/data/input/internal"
REF_INPUT = PKG / "software/CMS_HCC_v28/data/input/user_defined"
REF_OUTPUT = PKG / "software/CMS_HCC_v28/data/output/CMS_HCC_v28_2026_T_scores.csv"
CUTOFF = date(2026, 2, 1)

# (ID, DOB m/d/Y, SEX 1=M/2=F, OREC, LTIMCAID, NEMCAID, [dotless ICD-10 codes])
BENES = [
    ("1", "3/10/1954", 2, 0, 0, 0, ["E1122", "E1165"]),                       # diabetes hierarchy
    ("2", "6/1/1957", 1, 0, 0, 0, ["E1165", "I501", "N184"]),                 # multi-family + interactions
    ("3", "1/1/1946", 2, 0, 0, 0, ["J449", "N1830", "N185"]),                 # CKD hierarchy + COPD
    ("4", "1/1/1949", 1, 0, 0, 0, ["E1165", "I501", "N185", "J449", "J9621", "C3490"]),  # many HCCs (count)
    ("5", "1/1/1928", 2, 0, 0, 0, ["E1165"]),                                 # age 98 (F95_GT band)
    ("6", "5/1/1959", 1, 0, 0, 0, ["E1110"]),                                 # severe diabetes (HCC36)
    ("7", "1/1/1975", 1, 1, 0, 0, ["I501"]),                                  # disabled <65 (ND segment)
    ("8", "1/1/1950", 2, 0, 0, 0, []),                                        # demographic only
    ("9", "1/1/1945", 2, 0, 0, 0, ["F200", "G20"]),                           # non-family HCCs
    ("10", "1/1/1948", 1, 0, 0, 0, ["E1165", "I5023", "N184", "J449"]),       # acute-on-chronic HF specificity
]


def write_inputs():
    REF_INPUT.mkdir(parents=True, exist_ok=True)
    with open(REF_INPUT / "beneficiaries.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["ID", "DOB", "SEX", "OREC", "LTIMCAID", "NEMCAID"])
        for b in BENES:
            w.writerow([b[0], b[1], b[2], b[3], b[4], b[5]])
    with open(REF_INPUT / "diagnoses.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["ID", "ICD10"])
        for b in BENES:
            for icd in b[6]:
                w.writerow([b[0], icd])


def run_reference():
    r = subprocess.run(
        [sys.executable, "./software/CMS_HCC_v28/transform.py"],
        cwd=PKG_BASE, capture_output=True, text=True,
    )
    if r.returncode != 0:
        print("Reference run FAILED:\n", r.stdout, r.stderr)
        sys.exit(1)
    ref = {}
    with open(REF_OUTPUT, newline="") as f:
        for row in csv.DictReader(f):
            ref[str(row["ID"])] = row
    return ref


def run_engine():
    binary = REPO / "target/debug/examples/score_csv"
    r = subprocess.run(
        [str(binary), str(TABLES), str(REF_INPUT / "beneficiaries.csv"), str(REF_INPUT / "diagnoses.csv")],
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        print("Engine run FAILED:\n", r.stdout, r.stderr)
        sys.exit(1)
    out = {}
    for row in csv.DictReader(io.StringIO(r.stdout)):
        out[row["ID"]] = (row["SEGMENT"], float(row["SCORE"]))
    return out


def main():
    write_inputs()
    ref = run_reference()
    eng = run_engine()

    print(f"{'ID':>3}  {'SEGMENT':<14} {'REFERENCE':>10} {'ENGINE':>10}  RESULT")
    print("-" * 52)
    failures = 0
    for bid, *_ in BENES:
        seg, eng_score = eng[bid]
        ref_score = round(float(ref[bid][f"SCORE_{seg}"]), 3)
        ok = abs(ref_score - eng_score) < 1e-9
        failures += not ok
        print(f"{bid:>3}  {seg:<14} {ref_score:>10.3f} {eng_score:>10.3f}  {'OK' if ok else 'MISMATCH'}")

    print("-" * 52)
    if failures:
        print(f"FAIL: {failures}/{len(BENES)} beneficiaries disagree.")
        sys.exit(1)
    print(f"PASS: all {len(BENES)} beneficiaries agree to 3 decimals with the CMS reference.")


if __name__ == "__main__":
    main()
