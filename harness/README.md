# Harness

The evaluation harness has been **ported to Rust** — see the [`eval`](../crates/eval)
crate. The domain logic was first prototyped in Python here (fast, collaborative
iteration), then scaled into Rust once the metric definitions were proven. This
directory now holds only what remains Python by necessity, plus the frozen parity
fixtures.

## What's here

| Path | Role |
|---|---|
| `crosscheck.py` | **Stays Python.** Validates the Rust `engine` against CMS's *own* published Python reference software (`transform.py`) — intrinsically Python; it's validation, not domain logic. |
| `fixtures/` | Frozen reference: a sample audit + gold + candidates + notes, and `expected_metrics.json` (the Python harness's output). The Rust `eval` parity test (`crates/eval/tests/parity.rs`) asserts it reproduces these numbers. |
| `results/` | Real integration-run metrics + `INTEGRATION_NOTES.md`. |

## The eval harness is now Rust (`crates/eval`)

```bash
# metrics
cargo run -p eval --bin hcceval -- --audit A.jsonl --gold G.jsonl \
  [--candidates C.jsonl | --fhir DIR] --tables <cms_v28_internal> \
  --crosswalk crosswalks/snomed_to_icd10_v28.csv [--json OUT]

# eval-set construction
cargo run -p eval --bin inject_errors -- --fhir-out DIR --candidates-out C.jsonl --gold-out G.jsonl
cargo run -p eval --bin make_worksheet -- --candidates C.jsonl --out worksheet.csv
cargo run -p eval --bin make_worksheet -- --to-gold worksheet.csv --out gold.jsonl
```

Two improvements the port made over the Python original (see
`results/INTEGRATION_NOTES.md`):

1. **Candidates come from the engine itself** — no more replicating the
   diagnosis→HCC + hierarchy logic in the harness; the engine is the single source
   of truth, so the reconcile step is gone.
2. **Evidence spans and regulatory citations are scored separately** — evidence
   must occur in the note (span accuracy + hallucination); citations are validated
   against an allowed regulatory-authority list rather than being required to
   appear in the note.

## `crosscheck.py` quick start

```bash
python3 -m venv .venv-ref && .venv-ref/bin/pip install pandas pyyaml
cargo build --example score_csv
.venv-ref/bin/python harness/crosscheck.py
```
