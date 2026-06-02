# Architecture

## System at a glance

```
┌─────────────────────────────────────────────────────────────┐
│                      ORCHESTRATION LAYER                       │
│   (agent loop: plan → act → verify → self-correct → report)    │
└───────────────┬───────────────────────────┬───────────────────┘
                │                           │
    ┌───────────▼──────────┐     ┌──────────▼───────────────┐
    │   LLM REASONING       │     │   DETERMINISTIC ENGINE    │
    │   (probabilistic)     │     │   (verifiable oracle)     │
    │                       │     │                           │
    │ - extract diagnoses   │     │ - ICD-10 → HCC mapping    │
    │   from clinical text  │     │ - HCC → risk score calc   │
    │ - judge documentation │◄───►│ - model version logic     │
    │   substantiation      │ tool│   (single version, e.g.   │
    │ - propose flags +     │calls│   pick one and pin it)    │
    │   regulatory citation │     │ - returns ground truth    │
    └───────────────────────┘     └──────────────────────────┘
                │                           │
                └───────────┬───────────────┘
                            │
                  ┌─────────▼──────────┐
                  │   EVAL HARNESS      │
                  │  (separate, offline) │
                  └─────────────────────┘
```

## Component 1 — Deterministic scoring engine (Rust)

The **truth side**. This is where the domain depth lives and where correctness is non-negotiable.

Responsibilities:
- ICD-10-CM diagnosis → HCC condition-category mapping for **one pinned model version** (choose deliberately — see note below).
- HCC → risk-score calculation including the coefficient lookup and the hierarchy logic (where a more severe HCC in a hierarchy trumps a less severe related one).
- Demographic factor handling (age/sex bands) at whatever fidelity the chosen synthetic data supports.
- A clean, typed tool interface the agent calls. Inputs: a set of diagnoses + demographics. Outputs: the HCCs triggered, the score, and *which input drove which HCC* (this provenance is what lets the agent reason about substantiation).

Design properties to make explicit (these are the resume-worthy engineering choices):
- **Deterministic and reproducible** — same input, same output, always.
- **Compile-checked model versioning** — encode the HCC model version as a type-level or clearly-bounded dimension so you cannot accidentally mix V-version coefficients with the wrong mapping. This is the "model governance" story.
- **Auditable** — every score decomposes into its contributing factors. This is the RADV-defense property *and* the thing the agent grounds against.

> **Model-version note:** Do not try to support every version. Pick one publicly documented CMS HCC model version, pin it, and say so. The portfolio point is made with one version done correctly. Multi-version support is a "later" item and is explicitly out of scope for v1.

## Component 2 — LLM reasoning layer

The **proposal side**. Probabilistic, and treated as such.

Two distinct reasoning jobs (keep them separate — they have different failure modes and get measured differently):

1. **Extraction** — from a synthetic clinical note, extract the diagnoses/conditions actually documented, with the supporting text span. This is a structured-extraction problem with a measurable accuracy.
2. **Substantiation judgment** — for each code that *could* be claimed, decide whether the documentation actually supports it under the relevant CMS documentation standard (e.g. is the condition documented to the specificity and with the provider/encounter context the rules require, or is it merely mentioned in history). This is the domain-reasoning core, and it is where the regulatory knowledge is irreplaceable.

The layer never finalizes a substantiation claim that contradicts the engine's logic — it must reconcile against the oracle.

## Component 3 — Orchestration / agent loop

The loop that makes it *agentic* rather than a single prompt:

```
1. PLAN     → given a member's synthetic record, decide what to assess
2. EXTRACT  → LLM pulls documented conditions + spans from the note(s)
3. SCORE    → call deterministic engine with extracted diagnoses
4. JUDGE    → LLM assesses substantiation of each engine-triggered HCC
              against the documentation
5. VERIFY   → cross-check: does the LLM's claim set agree with what the
              engine says the codes actually produce? Flag disagreements.
6. SELF-CORRECT → on disagreement, re-examine the specific code rather
              than accepting either side blindly
7. REPORT   → emit flags (unsupported / risky codes) each with a regulatory
              citation and the documentation gap that drives the flag
```

The **VERIFY → SELF-CORRECT against an oracle** segment is the crux of the design. It is the difference between "an LLM with a medical prompt" and "an agent grounded in a verifiable source of truth."

## Component 4 — Evaluation harness (offline, separate)

Not part of the runtime path. Detailed in `03_EVALUATION_HARNESS.md`. The key architectural point: ground truth comes from the deterministic engine + a hand-labeled substantiation set, so the harness can score the LLM's two jobs independently.

**As built:** the eval harness became a **Rust `eval` crate** (`crates/eval`) rather than Python; only the CMS-reference cross-check and the differential fuzzer (`harness/crosscheck.py`, `harness/fuzz_engine.py`) stayed Python, since they drive CMS's own reference software directly.

## Optional extension (only after v1 ships)

Expose the deterministic engine as an **MCP server** so any LLM client can call it as a scoring/audit tool. A clean, finishable add-on. Do not let it block v1.

## Technology choices

- **Rust** for the engine (deterministic, fast, compile-checked model versioning).
- **LLM via API** for the reasoning layer; keep the provider behind a thin interface so it's swappable.
- **AI-assisted build (Claude Code)** end-to-end — itself part of the applied-AI story.
- Eval harness can be Rust or Python; Python is fine here since the deliverable is the numbers, not the harness.
