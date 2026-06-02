# Eval Harness

This directory holds the Python evaluation harness for the HCC Substantiation
Auditor. It scores the `agent`'s output (`audit_results.jsonl`) against gold
substantiation labels and the source clinical notes, on synthetic data only,
pinned to CMS-HCC model **V28** and scoped to the four target families
(diabetes, CKD, heart failure, COPD).

The harness has two halves, implemented in the `hcceval/` package:

* **3b — evaluation set**: a gold-label format, a *mechanical truth* deriver that
  lists each patient's engine-eligible HCCs, a *controlled error injector* that
  builds known-truth cases, and a human *labeling worksheet*.
* **3c — metrics**: extraction, substantiation, and system-level metrics computed
  from the audit JSONL + gold labels + notes, emitted as a table (stdout) plus
  JSON / Markdown summaries.

## Quick start (runs offline on committed fixtures)

```bash
python3 -m venv .venv && .venv/bin/pip install pandas numpy scikit-learn
harness/run_fixtures.sh          # unit tests + the full metrics table on fixtures
```

`run_fixtures.sh` runs the unit-test suite (known audit + known gold → known
metric values) and then prints the metrics computed from
`harness/fixtures/`. Everything is self-contained — no agent run, no network.

## Package layout (`hcceval/`)

| module | role |
|---|---|
| `schema.py` | record model + JSONL I/O for audit / gold / candidate records |
| `crosswalk.py` | SNOMED→ICD-10→V28-HCC lookup + V28 hierarchy collapse |
| `candidates.py` | mechanical truth: cohort CSV → per-patient engine-eligible HCCs |
| `notes.py` | reconstruct the note text the auditor saw (mirrors `fhir.rs`) |
| `inject.py` | controlled error injection (known-truth supported/unsupported cases) |
| `worksheet.py` | M.E.A.T.-anchored labeling worksheet (CSV ⇄ gold labels) |
| `metrics.py` | all metric computations |
| `report.py` | table / JSON / Markdown rendering |

CLIs (each `--help`): `derive_candidates.py`, `inject_errors.py`,
`make_worksheet.py`, `eval.py`.

## 3b — the evaluation set

### Gold-label format (JSONL)

One object per (patient, HCC) substantiation judgment:

```json
{"patient_id": "…", "hcc": 37, "gold_status": "supported|unsupported",
 "source": "manual|injected", "rationale": "…", "holdout": false}
```

`holdout: true` marks the **never-for-tuning** slice — reserved truth that must
not inform prompt or threshold changes. `eval.py --exclude-holdout` scores
without it.

### Mechanical truth helper (`derive_candidates.py`)

Reads the cohort `conditions.csv`, maps each SNOMED condition through the
committed crosswalk to an ICD-10 code and then to a V28 HCC, and applies the V28
hierarchy collapse — producing, per patient, the **candidate set** of coded HCCs
to be labeled.

**The engine remains the authority for the coded HCC set.** This is a faithful
*replication* of the engine's diagnosis→HCC + hierarchy logic, built so the eval
set can be scoped before the agent emits `audit_results.jsonl`. It reuses the
exact artifacts the engine consumes — the committed crosswalk (whose
`expected_v28_hcc` column is verified against the CMS V28 mapping) and the
committed `V28_HCC_Hierarchies.csv` — and deliberately omits demographics,
interactions, and coefficients (none change the candidate *set*). At integration,
`derive_candidates.py --reconcile audit_results.jsonl` cross-checks the derived
set against the engine's authoritative output and reports any drift (over-/under-
listing) per patient.

### Controlled error injection (`inject_errors.py`)

So the eval measures real RADV-style catching rather than code-echoing, the
injector constructs **known-truth** cases. For each target family it emits two
matched patients sharing the same coded HCC: a *supported* case whose note
carries full M.E.A.T. evidence, and an *unsupported* case where the code is
present but the note is silent on it (a textbook over-code). Each case is written
as a Synthea-style FHIR bundle (auditable by the real agent post-merge) plus a
candidate and a gold label whose truth is known by construction. A configurable
slice is flagged `holdout`.

### Labeling worksheet (`make_worksheet.py`)

Scaffolds a CSV with one row per candidate and M.E.A.T. anchor columns
(Monitor / Evaluate / Assess / Treat + specificity) the reviewer fills to decide
`gold_status`. `make_worksheet.py --to-gold worksheet.csv` converts a filled
sheet back to gold-label JSONL.

## 3c — metrics

`eval.py` joins `audit_results.jsonl` with gold labels (and, when available, the
candidate set and the FHIR notes) and prints the metrics. Undefined ratios
(zero denominator) render as `n/a` — the harness never fabricates a value.

**Binary convention.** The auditor's 3-way `final_status` collapses to a binary
*flag*: only `supported` clears a code; `risky` and `unsupported` are both
flagged. The detection target (positive class) is "this code is NOT
substantiated". With gold being supported/unsupported:

```
TP = gold unsupported & flagged    FP = gold supported   & flagged
FN = gold unsupported & cleared    TN = gold supported   & cleared
```

### Substantiation
| metric | definition |
|---|---|
| accuracy | (TP+TN)/N — agreement with gold on the binary call |
| flag precision / recall / F1 | TP/(TP+FP), TP/(TP+FN), harmonic mean |
| **over-coding rate** | **FN / N** — over-codes wrongly cleared across the reviewed population; the expensive RADV error (weighted) |
| under-flagging rate | FN / gold-unsupported = 1 − recall (miss rate among codes that should be flagged) |
| over-flagging rate | FP / gold-supported — false alarms / clinician burden |

Over-coding and under-flagging share the FN numerator but use different
denominators (population exposure vs. conditional miss rate); both are reported
to keep the financial and the clinical readings distinct. Gold pairs the auditor
never reviewed are reported as a coverage gap (`gold_pairs_unreviewed`), not
scored as right or wrong.

### Extraction (HCC-level vs. the candidate set)
Precision / recall / F1, micro-averaged. Gold = the engine-eligible candidate
HCCs; predicted = the HCCs implied by the auditor's extracted `icd10` codes
(crosswalked, then hierarchy-collapsed). Out-of-scope or HCC-less extractions are
excluded from both sides and counted as `unmapped_extractions` (nothing is
silently dropped). This is a proxy for condition-level extraction; span accuracy
is the orthogonal quote-fidelity check.

### System
* **agreement-with-gold** — fraction of scored pairs where the final call matches gold.
* **disagreement-resolution rate** — of codes the oracle loop moved across the
  supported/flagged line (`initial_status` ≠ `final_status`), how often the
  *final* call matches gold; split into `oracle_improved` vs. `oracle_regressed`.
  Answers "does the second pass help?"
* **span accuracy** — fraction of extracted `evidence` quotes that actually occur
  in the note (whitespace/case-tolerant substring against the reconstructed note).
* **hallucination rate** — fraction of *all* auditor quotes (extraction evidence +
  substantiation citations) with no basis in the note. Target 0.
* **calibration** — if `confidence` is present, a reliability curve (mean
  confidence vs. empirical accuracy per bin) and ECE.

### Output
The metrics table prints to stdout; `--json` / `--md` write machine-readable and
docs-ready summaries. `harness/fixtures/expected_metrics.{json,md}` are committed
reference outputs.

## Integration (post-merge checklist)

1. Run the agent to produce `audit_results.jsonl` over the cohort FHIR bundles.
2. `derive_candidates.py --reconcile audit_results.jsonl` — confirm the
   replication matches the engine (expect EXACT).
3. `make_worksheet.py` from the candidates → human labels the worksheet →
   `--to-gold` → `gold_labels.jsonl` (optionally augmented with `inject_errors.py`).
4. `eval.py --audit … --gold … --candidates … --fhir <cohort/fhir>` → the numbers.

## `crosscheck.py` — engine vs. CMS reference (Phase 1)

Validates the `engine` crate against CMS's own published V28 Python reference
software. It runs an identical battery of synthetic beneficiaries through both
implementations and asserts the risk scores agree to 3 decimals on each
beneficiary's community continuing-enrollee segment. The engine is pointed at the
*full* CMS tables, so this exercises the complete V28 model.

Prerequisites: the CMS V28 model software extracted under `data/cms_hcc_v28/`
(see `data/README.md`) and a Python env with `pandas`:

```bash
python3 -m venv .venv-ref && .venv-ref/bin/pip install pandas pyyaml
cargo build --example score_csv
.venv-ref/bin/python harness/crosscheck.py
```

Exit code 0 = full agreement. Synthetic inputs only — no real patient data.

