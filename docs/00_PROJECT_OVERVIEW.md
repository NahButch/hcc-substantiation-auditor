# hcc-substantiation-auditor

**RADV Audit-Defense Agent — a portfolio project demonstrating domain + AI fusion for applied-AI roles.**

*(Project/repo name: `hcc-substantiation-auditor`. It audits whether clinical documentation substantiates the HCC risk-adjustment codes a record would claim. "Auditor" names the function — a pre-audit RADV review — not branding. Keep the README explicit that this is a portfolio demonstration, not a certified audit tool.)*

---

## One-line pitch

An agentic system that reads (synthetic) clinical documentation, reasons about which CMS-HCC risk-adjustment codes are and are not substantiated by that documentation, flags unsupported codes before a RADV audit would, and cites the specific regulatory basis — using a deterministic Rust scoring engine as the verifiable ground-truth oracle the agent checks itself against.

## Why this project (the credibility thesis)

The value is the **inseparability of the two halves**:

- The LLM/agent layer is useless without the regulatory knowledge — anyone can prompt a model with "find unsupported codes," but only correct HCC/RADV domain logic makes the output trustworthy.
- The regulatory knowledge is what makes the AI *checkable* — the deterministic engine is a source of truth the probabilistic agent must agree with.

A generalist ML engineer cannot build the domain side. A domain expert without AI skills cannot build the agent side. The candidate sits in the only position that can build both. That is the entire story "domain + AI fusion" needs to tell, and it is verifiable in code.

## The core architectural pattern

**"LLM proposes, verified engine disposes."**

A non-deterministic model paired with a checkable source of truth. The agent reasons in natural language about clinical documentation and regulatory sufficiency; the deterministic engine scores risk and provides ground truth; the agent cannot assert a code is supported if the engine's logic disagrees. This pattern — grounding probabilistic reasoning against a verifiable oracle — is exactly what serious applied-AI teams care about right now, and the candidate is unusually positioned to build the *truth side* correctly.

## What "done" looks like

1. A deterministic Rust risk-scoring engine (HCC mapping + risk-score calculation) with a clean tool interface.
2. An agent loop that ingests synthetic clinical documentation, proposes a coding/substantiation assessment, calls the engine to verify, and self-corrects.
3. Every flag carries a specific regulatory citation.
4. A **published evaluation harness** with real numbers: extraction accuracy, substantiation-judgment accuracy, hallucination/over-coding rate, agreement-with-ground-truth, and a failure-mode analysis.
5. Open-sourced, AI-assisted (Claude Code), built in Rust, on CMS synthetic data.

## The non-negotiable guardrail

**CMS synthetic data only.** Never the candidate's own medical or claims records, never real patient data. Use CMS DE-SynPUF (synthetic Medicare claims) and/or Synthea-generated synthetic populations. Stated up front — "built on CMS synthetic beneficiary data" — this is a **credibility plus**, not a limitation, and it removes the privacy problem entirely.

## Document set in this package

| File | Purpose |
|------|---------|
| `00_PROJECT_OVERVIEW.md` | This file — the thesis and the shape. |
| `01_ARCHITECTURE.md` | System design, components, the agent loop, the oracle pattern. |
| `02_DATA_SOURCES.md` | Synthetic data options, tradeoffs, how to construct the labeled set. |
| `03_EVALUATION_HARNESS.md` | Metrics, ground-truth construction, failure-mode analysis framing. |
| `04_MILESTONES.md` | Phased build plan with finishable checkpoints. |
| `06_DATASETS_AND_BENCHMARKS.md` | Datasets, benchmarks, and government sources. |
| `07_POLICY_FEEDBACK_LOOP.md` | The policy → solution → capture → measure → report → propose frame. |
| `08_BUILD_RESULTS_SUMMARY.md` | Measured outcomes and how to read them. |

## Scope discipline (read before starting)

This project has an obvious failure mode: scope creep into "rebuild all of CMS risk adjustment." Resist it. The portfolio value is reached at a **single HCC model version, a narrow set of HCCs, and one clean agent loop with a real eval**. Breadth can come later; a finished, measured, narrow system beats an unfinished broad one every time.
