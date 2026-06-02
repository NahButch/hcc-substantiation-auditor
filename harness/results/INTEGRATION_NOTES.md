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
