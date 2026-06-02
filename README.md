# HCC Substantiation Auditor

An agentic system that flags unsubstantiated CMS-HCC risk-adjustment codes in
clinical documentation, grounded against a deterministic Rust scoring engine
used as a verifiable oracle.

> **DISCLAIMER — Portfolio demonstration only.**
> This project is a **portfolio demonstration of the substantiation-auditing
> pattern, NOT a certified or production audit tool**. It uses **synthetic data
> only** and pins the **CMS-HCC V28** model. It must not be used for real
> compliance, billing, or clinical decisions.

## Architecture

| Component | Location | Role |
|-----------|----------|------|
| `engine` | `crates/engine` | Deterministic CMS-HCC V28 scorer; acts as a verifiable oracle / source of truth |
| `agent` | `crates/agent` | LLM reasoning layer; implements a verify → self-correct loop against `engine` |
| Eval harness | `harness/` | Python evaluation harness; computes precision/recall/F-score metrics |

## Status

Under active development, building phase by phase.

- [x] Phase 0 — Setup, scaffolding, data acquisition (`scripts/fetch_data.sh`, `DATA.md`)
- [ ] Phase 1 — Deterministic CMS-HCC V28 scoring engine (`engine`): mapping, hierarchy, constraining, demographic factors, provenance
- [ ] Phase 2 — Extraction + single-pass substantiation assessment (`agent`)
- [ ] Phase 3 — Agent verify→self-correct loop + Python eval harness (`harness/`) + `RESULTS.md`

## Data policy

See [DATA.md](DATA.md).
