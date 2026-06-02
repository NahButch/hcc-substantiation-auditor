# Evaluation Harness

> A probabilistic system earns trust only when it is measured against ground truth. This harness does exactly that — scoring the two LLM jobs (extraction and substantiation) **separately**, against the deterministic engine and hand-labeled gold.

## Principle

The system has a **probabilistic half** (the LLM) and a **deterministic half** (the engine). The eval scores the probabilistic half against ground truth. Ground truth has two layers:

1. **Mechanical truth** — what the deterministic engine produces from structured diagnoses. Unambiguous.
2. **Substantiation truth** — the hand-labeled judgment of whether documentation supports each code under CMS standards. Authored by domain expertise.

Because the LLM does two distinct jobs (extraction and substantiation judgment), they get measured **separately**. Mixing them hides where the system actually fails.

## Metric set

### Job 1 — Extraction (clinical note → documented conditions)
- **Precision / recall / F1** on extracted conditions vs. the documented-condition labels.
- **Span accuracy** — does the model point to the correct supporting text, not just the right label? (Grounding quality.)
- Report per-condition-type where sample size allows; aggregate otherwise.

### Job 2 — Substantiation judgment (is this HCC supported by the documentation?)
- **Accuracy / precision / recall** against the hand-labeled substantiation truth.
- **Over-coding rate** — how often the agent calls a code supported that the labels say is not. This is the RADV-relevant false-positive; weight it explicitly, because in the real world over-coding is the expensive error.
- **Under-flagging rate** — supported codes the agent wrongly flags as risky (the nuisance error).

### System-level (agent vs. oracle)
- **Agreement-with-ground-truth** — final flag set vs. the combined truth.
- **Disagreement-resolution rate** — when the LLM and engine initially disagree, how often does the self-correction loop reach the correct answer? This number is the headline evidence that the oracle-grounding loop *works*, not just that it exists.
- **Hallucination rate** — flags or citations the agent produces that have no basis in the documentation or the regulations. Target: drive toward zero; report honestly.

### Calibration
- If the agent emits confidence, plot **calibration** (reliability curve). Even a rough version signals seriousness.

## The deliverable: a published results table + failure analysis

A README section (or short report) with:
1. The metric table above, with real numbers on the labeled set.
2. **A failure-mode analysis** — categorize where the agent fails and *why*: e.g. "fails on conditions documented only in history sections," "over-codes when a symptom is mentioned without a confirmed diagnosis," "misattributes specificity." This qualitative layer, grounded in domain knowledge, is what makes the analysis credible.
3. A short **"what the oracle caught"** section — concrete cases where the deterministic engine corrected the LLM, demonstrating the pattern's value.

## The metric shape

This is the same shape as **prediction-vs-actual reconciliation and variance
decomposition**: bind a model's output to a source of truth, compute where and how
much it diverges, and decompose the divergence into interpretable drivers (here:
extraction error vs. substantiation-judgment error vs. citation error).

## Methodology cautions (state these; they signal rigor)
- The substantiation labels are **one expert's judgment** on **synthetic** notes — not a multi-rater gold standard and not real charts. State this limit plainly; it does not weaken the project, and pretending otherwise would.
- Synthea notes are simpler than real clinical documentation; results are a floor, not a real-world performance claim.
- Keep a held-out slice the model/prompts were never tuned on, and report on that slice.
