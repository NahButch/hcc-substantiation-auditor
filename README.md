# HCC Substantiation Auditor

Flags **unsubstantiated CMS-HCC risk-adjustment codes** — codes that are billed
but not backed by M.E.A.T.-grade evidence in the clinical note. The design
pattern is **the LLM proposes, a deterministic engine disposes**:

- a deterministic Rust **`engine`** computes the CMS-HCC **V28** coded-HCC set and
  risk score — reproducible and validated against CMS's own reference software
  (it is the **oracle / source of truth**);
- an LLM **`agent`** does the open-ended reading — extract conditions from the
  note, then judge each engine-coded HCC for M.E.A.T./specificity — and runs a
  **`verify → self-correct`** loop, but is never allowed to invent the code set the
  engine already decided.

Deterministic where correctness must be guaranteed; probabilistic only where
judgment is unavoidable. A Rust **`eval`** crate then scores the two LLM jobs
(extraction, substantiation) separately against gold labels.

> **DISCLAIMER — Portfolio demonstration only.**
> This project is a **portfolio demonstration of the substantiation-auditing
> pattern, NOT a certified or production audit tool**. It uses **synthetic data
> only** and pins the **CMS-HCC V28** model. It must not be used for real
> compliance, billing, or clinical decisions.

## Where to look

| Component | Location | Role |
|-----------|----------|------|
| `engine` | [`crates/engine`](crates/engine) | Deterministic CMS-HCC V28 scorer — the verifiable oracle / source of truth |
| `agent` | [`crates/agent`](crates/agent) | LLM layer: note → conditions extraction, per-HCC M.E.A.T. judgment, `verify → self-correct` loop against `engine` |
| `eval` | [`crates/eval`](crates/eval) | Rust eval harness: gold-label eval set + extraction/substantiation/system metrics (`harness/crosscheck.py` stays Python — it drives CMS's own reference) |
| **Results** | [**`RESULTS.md`**](RESULTS.md) | Real numbers on the known-truth slice, with failure analysis and honest limitations |

**New here?** Read [`RESULTS.md`](RESULTS.md) for what it does and how well it
does it, then [`crates/engine`](crates/engine) for the deterministic core.

## Status

Built phase by phase; the end-to-end pipeline runs and is measured.

- [x] Phase 0 — Setup, scaffolding, data acquisition (`scripts/fetch_data.sh`, `DATA.md`)
- [x] Phase 1 — Deterministic CMS-HCC V28 scoring engine (`engine`): mapping, hierarchy, constraining, demographic factors, interactions, provenance — verified against the CMS reference (`harness/crosscheck.py`)
- [x] Phase 2 — Extraction + single-pass substantiation assessment (`agent`): local LLM (Ollama) behind a swappable interface, note→conditions extraction, per-HCC M.E.A.T. judgment with citations, run over a Synthea cohort
- [x] Phase 3 — Agent `verify → self-correct` loop (`agent`) + eval harness (Rust `crates/eval`, ported from the Python prototype) + results: see [`RESULTS.md`](RESULTS.md)

Numbers are an **honest floor** on synthetic, partly by-construction data — not a
real-world performance claim. See [`RESULTS.md`](RESULTS.md) § Limitations.

## Data policy

See [DATA.md](DATA.md).
