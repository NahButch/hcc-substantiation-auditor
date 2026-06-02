# Eval Harness

This directory holds the Python evaluation harness for the HCC Substantiation
Auditor (Phase 3). Metrics — precision, recall, and F-score — are computed
here by running the `agent` binary against a labelled synthetic dataset and
comparing its substantiation decisions to ground-truth labels derived from the
`engine` oracle.

Implementation begins in Phase 3 once the Rust crates are functional end-to-end.

## `crosscheck.py` — engine vs. CMS reference (Phase 1)

Validates the `engine` crate against CMS's own published V28 Python reference
software. It runs an identical battery of synthetic beneficiaries through both
implementations and asserts the risk scores agree to 3 decimals on each
beneficiary's community continuing-enrollee segment. The engine is pointed at the
*full* CMS tables, so this exercises the complete V28 model.

Prerequisites: the CMS V28 model software extracted under `data/cms_hcc_v28/`
(see `data/README.md`) and a Python env with `pandas`:

```bash
python3 -m venv .venv-ref && .venv-ref/bin/pip install pandas pyyaml
cargo build --example score_csv
.venv-ref/bin/python harness/crosscheck.py
```

Exit code 0 = full agreement. Synthetic inputs only — no real patient data.

