# Integration run — findings

First end-to-end pass wiring the Rust `agent` (verify→self-correct) to the Python
`hcceval` harness. Synthetic data only; local `qwen2.5:7b-instruct` via Ollama.
Numbers are an honest floor on a tiny set — directional, not statistical.

## Setup
- **Substantiation metrics:** 8 controlled-error-injection patients (4 supported / 4
  unsupported, by construction) → `integration_metrics.{md,json}`.
- **Engine reconciliation:** agent run over 12 cohort patients, compared to the
  harness's independent HCC derivation.

## Results
- **Engine reconciliation: EXACT.** 16 HCCs across 12 patients — 0 over-list, 0
  under-list. The harness's Python replication of diagnosis→HCC + V28 hierarchy
  matches the Rust engine exactly. (Two independent implementations agree.)
- **Substantiation (N=8 known-truth):** accuracy 87.5%, flag precision 80%,
  recall 100%, F1 0.889, **over-coding 0%** (every unsupported code caught — the
  expensive RADV error), over-flagging 25% (1 supported case wrongly flagged).
- **Span accuracy 87.5%** — extracted evidence quotes occur in the notes.

## Issues surfaced (to fix)
1. **Citation grounding / "hallucination" is a metric artifact (semantic mismatch).**
   The agent emits *regulatory* citations (e.g. "CMS Managed Care Manual Ch.7") as
   external references; the harness scores every quote — citations included — as
   text that must appear *in the note*, inflating "hallucination" to ~56%. Fix:
   separate note-grounded evidence spans (should be in-note) from regulatory
   citations (should match an allowed authority list), and score them differently.
2. **Extraction HCC-recall 0% (8 unmapped).** The agent's extracted conditions don't
   carry mappable ICD-10, so the HCC-level extraction metric can't link them. Fix:
   have extraction emit ICD-10 (or map condition names), or score extraction at the
   condition level instead of HCC level.
3. **Oracle loop regressed 1 case** (small N). Worth inspecting the self-correct
   prompt on borderline "supported" cases before trusting the loop at scale.

## Update — scaled run + fuzzer (supersedes the n=8 numbers above)

The notes above are the first n=8 integration pass. Current authoritative numbers
are the scaled run (36 patients / 47 gold pairs; injected + manual annotation) in
`integration_metrics.{md,json}` and [`../../RESULTS.md`](../../RESULTS.md). Status
of the three findings:

1. **Citation/hallucination metric** — FIXED in the Rust `eval` port: citations are
   validated against a regulatory-authority allow-list; hallucination is now
   evidence-only (40.3% at scale — the model paraphrases evidence spans).
2. **Extraction recall** — the agent now emits ICD-10 (prompt + family backfill), so
   recall is measurable: 17.0% (still low — coded chronic conditions are buried in
   the HPI history list). Was 0% / unmeasurable before.
3. **Oracle-loop regression** — CONFIRMED at scale and still open: of 4 changed
   judgments, 3 regressed and 1 improved (resolution 25%). The self-correct prompt
   needs tuning before the loop is trusted.

**New (differential fuzzer, `harness/fuzz_engine.py`):** 2,100 random claims agree
exactly with CMS `transform.py` after fixing a negative-age age/sex bug the fuzzer
surfaced (engine `age_sex_variable` now returns `None` for out-of-band ages).

## Tuning attempt — inconclusive (recorded, reverted)

We tried to fix the two open weaknesses with prompt changes: (a) a *conservative*
self-correct prompt (don't flip on an automated-extractor miss; re-read the note),
and (b) inclusive + strictly-verbatim extraction. A second scaled audit (v2) gave a
**mixed, noise-dominated** result that did not beat v1:

| metric | v1 | v2 (tuned) |
|---|---|---|
| over-flagging | 55.6% | 33.3% (better) |
| over-coding | 2.1% | 7.0% (worse) |
| oracle disagreement-resolution | 1↑/3↓ | 1↑/4↓ (still net-negative) |
| extraction recall | 17.0% | 11.6% (worse) |
| hallucination | 40.3% | 45.4% (worse) |

Two reasons it's not a trustworthy delta: the runs scored **different patient sets**
(34 vs 36 audited — varying LLM JSON-parse dropouts → 43 vs 47 pairs), and at
n≈43 with a stochastic 7B local model the deltas are within run-to-run variance.
**Conclusion:** prompt tweaks alone don't reliably move these metrics at this
scale/model. The real levers are a **stronger model**, **multi-run averaging on a
fixed patient set**, and **less-templated notes** (Synthea buries chronic conditions
in a history list, capping extraction recall). The prompt changes were reverted to
keep the repo consistent with the reported v1 numbers.

## Reproduce
```
# injected known-truth metrics
.venv-ref/bin/python harness/inject_errors.py --fhir-out data/integration/injected_fhir \
  --candidates-out data/integration/injected_candidates.jsonl --gold-out data/integration/injected_gold.jsonl
./target/debug/examples/audit_jsonl <tables> crosswalks/snomed_to_icd10_v28.csv \
  data/integration/injected_fhir data/integration/injected_audit.jsonl 50
.venv-ref/bin/python harness/eval.py --audit data/integration/injected_audit.jsonl \
  --gold data/integration/injected_gold.jsonl --candidates data/integration/injected_candidates.jsonl \
  --fhir data/integration/injected_fhir --md harness/results/integration_metrics.md

# engine reconciliation
./target/debug/examples/audit_jsonl <tables> crosswalks/snomed_to_icd10_v28.csv \
  data/synthea/cohort/fhir data/integration/cohort_audit.jsonl 12
.venv-ref/bin/python harness/derive_candidates.py --reconcile data/integration/cohort_audit.jsonl
```
