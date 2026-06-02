# RESULTS — HCC Substantiation Auditor

> **Research & educational demonstration on synthetic data only.** Every number is an honest
> *floor*, not a real-world performance claim. Figures are quoted verbatim from the
> committed artifacts in [`harness/results/`](harness/results/)
> ([`integration_metrics.md`](harness/results/integration_metrics.md),
> [`integration_metrics.json`](harness/results/integration_metrics.json)); where a
> ratio is undefined the harness prints `n/a`. See [§5 Limitations](#5-limitations).

## 0. Headline (scaled run)

**36 patients, 47 coded HCCs scored against gold.** Gold = **8 injected**
known-truth pairs (by construction) + **39 manual annotations** on the synthetic
cohort by a single independent reviewer (LLM-assisted, **not** a credentialed RADV
coder — see [§3](#3-evaluation-set--gold)).

| Metric | Value | Read |
|---|---|---|
| Substantiation accuracy | **87.2%** | agreement with gold (binary flag) |
| Flag precision / recall / F1 | 88.1% / **97.4%** / 0.925 | catches almost every over-code |
| **Over-coding rate** | **2.1%** | FN/all — the expensive RADV error (1 missed) |
| Over-flagging rate | **55.6%** | FP/supported — 5 of 9 supported codes wrongly flagged |
| Disagreement-resolution | **25%** (1 improved, 3 regressed) | the oracle loop is **net-negative** here |
| Extraction P / R / F1 | 72.7% / **17.0%** / 0.276 | recall is the weak spot |
| Span accuracy | 59.7% | evidence quotes that occur in the note |
| Citation validity | **100%** | citations match a real regulatory authority |
| Hallucination rate | **40.3%** | evidence quotes with no basis in the note |
| Calibration (ECE) | 0.193 (n=14) | over-confident in the top bin (0.9–1.0 → 50% acc) |

**Engine.** Separately, a differential fuzzer (`harness/fuzz_engine.py`) ran
**2,100 random synthetic claims** (full ICD-10/HCC set, random age/sex/OREC/Medicaid)
through both the Rust engine and CMS's own `transform.py`: **exact agreement to 3
decimals** — after the fuzzer surfaced and we fixed one real bug (see [§4](#4-what-the-fuzzer-caught)).

## 1. What this measures

Two LLM jobs, scored **separately** so a good aggregate can't hide a weak part:

1. **Extraction** — read the note, pull conditions (ICD-10 + evidence quote +
   section). Scored at the HCC level vs. the engine-eligible candidate set, plus a
   span-fidelity check that each evidence quote actually occurs in the note.
2. **Substantiation** — for each engine-coded HCC, judge whether the note documents
   it to M.E.A.T./specificity. This is the audit call.

Plus the **system / oracle-loop** behaviour: the agent proposes, a
`verify → self-correct` pass re-examines, and the deterministic engine owns the
authoritative coded-HCC set. "Did the second pass help?" is its own metric.

**Binary flag convention.** Only `supported` clears a code; `risky`/`unsupported`
are **flagged**. Positive class = "not substantiated":
`TP` = gold-unsupported & flagged, `FP` = gold-supported & flagged, `FN` =
gold-unsupported & cleared (the expensive RADV miss), `TN` = gold-supported & cleared.
This run: **TP=37, FP=5, FN=1, TN=4** over 47 scored pairs.

## 2. What the numbers say

- **Detection is strong.** Recall **97.4%**, over-coding **2.1%** — the auditor
  catches all but one of the unsupported codes (the financially expensive direction).
- **It over-flags supported codes.** Over-flagging **55.6%**: of 9 genuinely
  supported codes it wrongly flagged 5. The auditor errs heavily toward "flag",
  which is safe for RADV exposure but high clinician burden.
- **The oracle loop is currently net-negative.** Of 4 judgments it moved across the
  supported/flagged line, **3 regressed** and 1 improved. The `verify → self-correct`
  pass needs tuning before it can be trusted at scale (see [§4](#4-what-the-fuzzer-caught)).
- **Extraction recall is the weak spot (17%).** The agent surfaces the visit-reason
  conditions but misses most *coded* chronic conditions, which Synthea buries in the
  HPI past-history list rather than the Assessment & Plan (174 extractions were
  out-of-scope/unmappable). The ICD-10 backfill made this *measurable* (it was 0%
  before); the low value is a real finding about both the notes and the extractor.
- **Hallucination 40%.** Two-fifths of evidence quotes don't verbatim-match the
  note — the model paraphrases. Citations, by contrast, are 100% valid regulatory
  authorities (scored against an allow-list, not the note text).

## 3. Evaluation set & gold

- **Injected known-truth (8):** for each family, a matched pair — a *supported* note
  with full M.E.A.T. and an *unsupported* note where the code is present but the note
  is silent (a textbook over-code). Truth known by construction; 4 flagged `holdout`.
- **Manual cohort annotation (39):** a single independent reviewer pass over 30
  HCC-positive Synthea patients, per a documented rule
  ([`harness/results/manual_label_rule.md`](harness/results/manual_label_rule.md)):
  a coded HCC is *supported* iff the note's Assessment & Plan documents
  condition-specific M.E.A.T.; a condition appearing only in the past-history list is
  *unsupported*. Five cohort patients had genuine in-encounter management (e.g.
  spirometry + pulmonary rehab for COPD; echo + HF panel + furosemide for HF;
  diabetic-retinal-eye-exam monitoring) → supported; the rest were history-list-only.
  **Caveat:** these are annotator labels (LLM-assisted, single reviewer), **not**
  credentialed RADV expert gold. The build is scripted and reproducible
  ([`build_combined_gold.py`](harness/results/build_combined_gold.py)).

## 4. What the fuzzer caught

The differential fuzzer flagged **8/600** beneficiaries on its first run — all
**negative-age** patients (born after the Feb-1 payment-year cutoff). The engine was
defaulting them into the `0_34` age/sex cell and adding its coefficient, whereas CMS
assigns *no* age/sex cell when the age falls in no band. Fixed (`age_sex_variable`
now returns `None`), regression-tested, and re-verified: **600 + 1,500 random claims
agree exactly** with the CMS reference. This is a bug the original 10 hand-picked
validation cases never exercised — the value of differential fuzzing.

(The oracle-loop regression in §2 is a separate, still-open issue: the self-correct
prompt should be tuned on borderline cases before the loop is trusted.)

## 5. Limitations

- **Synthetic, templated notes.** Synthea notes are short and list chronic
  conditions in history rather than managing them; real charts differ.
- **Gold is partly by-construction and partly single-annotator.** No credentialed
  expert labels yet; the manual slice is one LLM-assisted reviewer.
- **Small n (47 pairs).** A single flip swings rates by points; treat as direction.
- **Single local model.** `qwen2.5:7b-instruct` via Ollama, one run, temperature 0.
- **Scope.** CMS-HCC **V28**, community continuing-enrollee segments only (the engine
  fuzz covers NA/ND; New-Enrollee and institutional models are future work).
- **Oracle loop net-negative on this slice** — do not read the loop as a win yet.

## 6. Reproducibility

```bash
TABLES=data/cms_hcc_v28/python_v28/software/CMS_HCC_v28/data/input/internal

# (0) deterministic cohort
scripts/generate_cohort.sh                       # POP=2000 SEED=20260602 (Java 17+)

# (1) engine differential fuzz vs CMS reference (LLM-free)
.venv-ref/bin/python harness/fuzz_engine.py 1500 99

# (2) injected known-truth + agent audit (local Ollama; qwen2.5:7b-instruct)
cargo run -p eval --bin inject_errors -- --fhir-out data/integration/injected_fhir \
  --candidates-out data/integration/injected_candidates.jsonl --gold-out data/integration/injected_gold.jsonl
export HCC_MODEL=qwen2.5:7b-instruct
cargo run --example audit_jsonl -- "$TABLES" crosswalks/snomed_to_icd10_v28.csv \
  data/integration/eval_fhir data/integration/eval_audit.jsonl 200

# (3) combined gold (injected + manual) + metrics
.venv-ref/bin/python harness/results/build_combined_gold.py
cargo run -p eval --bin hcceval -- \
  --audit data/integration/eval_audit.jsonl --gold data/integration/combined_gold.jsonl \
  --tables "$TABLES" --crosswalk crosswalks/snomed_to_icd10_v28.csv \
  --fhir data/integration/eval_fhir \
  --json harness/results/integration_metrics.json
```
