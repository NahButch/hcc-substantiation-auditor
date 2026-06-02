# Eval Harness

This directory holds the Python evaluation harness for the HCC Substantiation
Auditor (Phase 3). Metrics — precision, recall, and F-score — are computed
here by running the `agent` binary against a labelled synthetic dataset and
comparing its substantiation decisions to ground-truth labels derived from the
`engine` oracle.

Implementation begins in Phase 3 once the Rust crates are functional end-to-end.
