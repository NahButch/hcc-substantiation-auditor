# RESULTS — HCC Substantiation Auditor

> **Portfolio demonstration on synthetic data only.** All numbers below are an
> honest *floor*, not a real-world performance claim. They are computed on a small,
> by-construction "known-truth" evaluation slice, scored against the deterministic
> CMS-HCC **V28** engine and gold labels that are **100% injected** (no manual
> expert labels yet — that slice is pending). See [Limitations](#5-limitations).

Every figure in this document is quoted from the committed metrics artifacts in
[`harness/results/`](harness/results/) — the human-readable
[`integration_metrics.md`](harness/results/integration_metrics.md) and the
machine-readable [`integration_metrics.json`](harness/results/integration_metrics.json).
Nothing here is rounded away or invented; where a ratio is undefined the harness
prints `n/a` and so do we. [Reproducibility](#6-reproducibility) lists the exact
commands that regenerate the artifacts.

---

## 1. What this measures

The system has two distinct LLM jobs, and we score them **separately** so a good
overall number can't hide a weak component:

1. **Extraction** — read a clinical note and pull out the conditions (with an
   ICD-10 code, an evidence quote, and the note section). Scored at the HCC level
   against the engine-eligible candidate set, plus a *span-fidelity* check that
   every evidence quote actually occurs in the note.
2. **Substantiation** — for each HCC the engine says is coded, judge whether the
   note documents it to M.E.A.T./specificity standards. This is the audit call.

On top of those two LLM jobs we measure the **system / oracle-loop** behaviour:
the agent proposes, then a `verify → self-correct` pass re-examines each judgment,
and the deterministic Rust `engine` defines the authoritative coded-HCC set the
agent is allowed to reason about. "Did the second pass help?" is its own metric.

**Binary flag convention.** The auditor's 3-way `final_status` collapses to a
binary *flag*: only `supported` clears a code; `risky` and `unsupported` are both
**flagged**. The detection target (positive class) is "this code is **not**
substantiated". With gold being `supported`/`unsupported`:

```
TP = gold unsupported & flagged    FP = gold supported   & flagged
FN = gold unsupported & cleared    TN = gold supported   & cleared
```

So a *miss* (FN) is the financially expensive RADV error — an over-code the
auditor waved through — and a *false alarm* (FP) is clinician burden.

### The evaluation slice

These numbers come from the **injected known-truth integration set**: 4 condition
families (diabetes, CKD, heart failure, COPD), each with a matched pair —

- a **supported** case whose note carries full M.E.A.T. for the coded HCC, and
- an **unsupported** case where the code is present but the note is silent on it
  (a textbook over-code).

`counts` from the artifact: **8 patients audited, 8 audited HCCs, 8 gold labels
(all injected; 4 flagged `holdout`), 8 candidates, 8 notes**, `gold_pairs_unreviewed: 0`.
The 4 `holdout` labels are exactly the 4 over-code (unsupported) cases — see the
note under the table.

---

## 2. Metric table

All values verbatim from `integration_metrics.json` (the full injected set, n=8,
including the holdout slice).

### Substantiation (positive class = unsupported / flagged)

| metric | value | meaning |
|---|---|---|
| confusion | TP=4, FP=1, FN=0, TN=3 | over 8 scored pairs |
| accuracy (agreement) | **87.5%** | (TP+TN)/N agreement with gold on the binary call |
| flag precision | **80.0%** | TP/(TP+FP) — 1 false alarm in 5 flags |
| flag recall | **100.0%** | TP/(TP+FN) — every over-code was flagged |
| flag F1 | **0.889** | harmonic mean |
| **over-coding rate** | **0.0%** | FN/N — over-codes wrongly cleared (the expensive RADV error) |
| under-flagging rate | **0.0%** | FN/gold-unsupported = 1 − recall |
| over-flagging rate | **25.0%** | FP/gold-supported — false alarms / clinician burden |

> **Read this carefully — the holdout caveat.** The 4 `holdout` labels *are* the
> 4 over-code cases, so **all of the detection signal (recall 100%, TP=4) is
> measured on the held-out, never-for-tuning slice.** Re-scored with
> `--exclude-holdout` (only the 4 supported cases remain): accuracy **75.0%**,
> `TP=0 FP=1 FN=0 TN=3`, precision **0.0%**, recall **n/a**, over-flagging **25.0%**.
> The lone false alarm dominates whatever is left once the positives are removed.

### System / oracle loop

| metric | value | meaning |
|---|---|---|
| agreement with gold | **87.5%** | fraction of scored pairs where the final call matches gold |
| changed by oracle loop | **1** | pairs the `verify→self-correct` pass moved across the supported/flagged line |
| disagreement-resolution | **0.0%** | of changed pairs, how often the *final* call matches gold |
| oracle improved / regressed | **0 / 1** | the single move was a regression (§3) |

### Extraction (HCC-level vs. the engine candidate set)

| metric | value |
|---|---|
| precision | **n/a** (TP+FP = 0) |
| recall | **0.0%** |
| F1 | **n/a** |
| TP / FP / FN | **0 / 0 / 8** |
| unmapped extractions | **8** |

This metric is **degenerate on this slice** — see §4.3 for why (it is not a
working 0%; it is structurally pinned). Span fidelity below is the meaningful
extraction signal here.

### Spans & hallucination

| metric | value | meaning |
|---|---|---|
| span accuracy | **87.5%** | 7 of 8 *evidence* quotes occur verbatim in the note |
| citation grounding | **0.0%** | 0 of 8 *citations* occur in the note (by design — see §4.2) |
| **hallucination rate** | **56.2%** | 9 of 16 total quotes (evidence + citations) ungrounded |

The blended **56.2%** conflates two quote kinds. Split out: **1 of 8 evidence
quotes** is ungrounded (the rest is the 8 citations, which are regulatory
references, not note substrings). The honest note-fidelity figure is the span
accuracy, **87.5%**; the headline hallucination rate overstates true hallucination
and should be read with §4.2.

### Calibration

| metric | value |
|---|---|
| records with confidence | **1 of 8** |
| ECE | **0.850** |
| reliability curve | single bin `[0.8,0.9]`: n=1, mean_conf 0.85, accuracy 0.0 |

Only one record (the corrected COPD case, §3) emitted a `confidence`, and that one
confident call was wrong — hence ECE 0.85. **Calibration is effectively
unmeasurable at n=1**; this number is reported for completeness, not as a claim.

---

## 3. What the verify → self-correct loop did (and what caught the over-codes)

The task here is to show concrete cases where the oracle loop moved a judgment
**toward truth**. The honest finding on the labeled set is: **it did not.**

**On the injected known-truth set the loop changed exactly one judgment, and it
was a regression.** Case `inj-copd-supported` (gold = *supported*, full M.E.A.T.
in the note):

```
hcc 280  initial: supported → final: risky   (corrected=True, confidence=0.85)
reason:  verify[supported_no_evidence]: initial 'supported' → final 'risky' —
         Initial status was 'supported' but lacked verbatim evidence in the note;
         re-examination shows M.E.A.T. criteria are met.
```

This single move is *both* the lone false positive (FP=1) *and* the lone oracle
regression (`oracle_regressed: 1`, `disagreement_resolution_rate: 0.0`). Its
mechanism is traced in §4.1 — the agent's own spliced evidence quote failed the
verbatim check, the verify pass fired `supported_no_evidence`, and it downgraded a
correct call even while its rationale admitted "M.E.A.T. criteria are met."

**Where the over-codes were actually caught — first pass, not the loop.** All 4
injected over-codes were correctly flagged `unsupported` on the *initial* pass
(`initial == final == unsupported`), with clean, specific documentation-gap notes:

| case | hcc | final | documentation_gap (verbatim, agent output) |
|---|---|---|---|
| `inj-ckd-unsupported` | 327 | unsupported | "No mention of chronic kidney disease or any related signs, symptoms, tests, treatments, or assessments." |
| `inj-copd-unsupported` | 280 | unsupported | "No mention of emphysema or any respiratory condition." |
| `inj-diabetes-unsupported` | 37 | unsupported | "No mention of Type 2 diabetes mellitus or chronic kidney disease. The note focuses on an ankle sprain." |
| `inj-heart_failure-unsupported` | 226 | unsupported | "No mention of heart failure or any related symptoms, signs, tests, treatments, or assessments." |

Each had `meat_present: []` and `specificity_supported: false`. So the recall of
100% is real, but it is owed to the **first-pass substantiation judgment**, not to
the self-correct loop.

**Off the labeled set (illustrative only, not scored).** Over the 5-patient
unlabeled Synthea cohort (`data/audits/audit_results.jsonl`) the loop fired three
times and in both directions — e.g. patient `4a1eedf3…`: hcc 37
`unsupported→supported` and hcc 327 `unsupported→risky`; patient `1fb4ead3…`:
hcc 326 `risky→supported` ("re-examination shows clear evidence of assessment
related to the condition"). These show the loop *engaging*, but **manual gold for
the cohort is pending**, so none of them can be scored as right or wrong. The
unit-test fixtures (`harness/fixtures/`) demonstrate the intended toward-truth
behaviour by construction (e.g. `unsupported→supported` when a second pass finds
HbA1c monitoring), but those are hand-built fixtures, not agent output.

**Bottom line:** on data with known truth, the second pass has not yet been shown
to help; its one action hurt. This is a finding, not a tuned result.

---

## 4. Failure-mode analysis

Each failure below is grounded in the committed output, not anticipated in the
abstract.

### 4.1 Spliced evidence quotes break the verbatim check (and trip the loop)

The agent sometimes builds an evidence quote by concatenating **non-contiguous**
note lines. For `inj-copd-supported` it emitted:

```
"1. COPD / pulmonary emphysema.\n   - Assess: moderate-to-severe COPD, emphysema-predominant."
```

The note really contains both lines — but with `Monitor:` and `Evaluate:` lines
*between* them, so the spliced string occurs nowhere verbatim (`quote_in_note`
returns false even with whitespace/case tolerance). This is the **1 ungrounded
evidence span** (span accuracy 7/8). It then cascades: the verify pass couldn't
confirm the quote → fired `supported_no_evidence` → downgraded a genuinely
supported code to `risky`. A quote-construction bug thus produced *both* the lone
FP and the lone oracle regression.

### 4.2 The `citation` field holds regulatory references, not note substrings

All 8 citations are pointers to authority — e.g. *"CMS Medicare Managed Care
Manual, Ch. 7 (risk adjustment)"* and *"ICD-10-CM Official Guidelines for Coding
and Reporting, Section IV"* — which by definition do not appear in the patient
note. The harness checks every quote against the note, so citation grounding is
**0% by construction**, and the blended hallucination rate (56.2%) inherits all 8
of those as "ungrounded." This is a **metric-definition issue, not 8 hallucinated
quotes**: true note-fidelity is the span accuracy (87.5%, one real miss). Both
numbers are reported so the financial/clinical reading isn't masked, but the
citation stream and the evidence stream should be scored on different rubrics.

### 4.3 HCC-level extraction recall is structurally pinned to 0% here

`recall 0.0%`, `unmapped_extractions: 8` is **not** a working measurement on this
slice, for two compounding reasons:

- **The 4 over-codes have no extractable coded condition by design** — the note is
  silent on the coded HCC, so the agent (correctly) extracts the *unrelated* visit
  diagnosis instead (influenza immunization, wrist laceration, ankle sprain,
  allergic rhinitis). Counting these as extraction FNs is an artifact of defining
  the candidate = the coded HCC; correct behaviour scores as a miss.
- **The 4 supported cases extract at a different specificity/format than the
  engine's candidate code**, so the crosswalk (keyed to the engine's dotless
  codes) doesn't line them up. Observed: `inj-ckd-supported` extracted **N18.3**
  (stage 3) against candidate **N184** (stage 4 → HCC 327); COPD extracted
  **J44.81** vs candidate **J439**; heart failure **I50.00** vs **I509**; diabetes
  emitted no ICD-10 at all. None map onto the exact candidate code → all 8 land in
  `unmapped_extractions` (nothing is silently dropped).

So the HCC-level extraction metric is uninformative on an 8-case set that is half
over-codes; **span accuracy is the meaningful extraction signal here.** The
specificity mismatch is itself a real finding — the agent reasons about CKD *stage*
loosely, which is exactly the kind of error that matters when stage drives the HCC.

### 4.4 Confidence is sparse and, where present, miscalibrated

7 of 8 records emitted no `confidence`; the only one that did (0.85) was the wrong
COPD call, giving ECE 0.85. The model is not yet producing usable confidence
signal, and there is far too little of it to calibrate.

### 4.5 Not yet exercised by this slice

The injected over-codes are all "code present, note entirely silent," which is the
easy end of the over-coding spectrum. Two failure modes the literature cares about
— **conditions documented only in a History/Past-Medical-History section** (no
current-visit M.E.A.T.) and **symptom-without-diagnosis over-coding** — are **not
represented** in these 8 cases, so we have **no measurement** of them. Thin
Synthea/injected prose (templated, a few lines per encounter) is also far simpler
than a real chart. These are stated as gaps, not results.

---

## 5. Limitations

- **Synthetic notes only.** Synthea/injected notes are short and templated; real
  charts are longer, messier, and multi-author. Numbers here will not transfer.
- **Gold is 100% injected / by-construction.** `gold_injected: 8`, and there are
  **zero manual expert labels** in this slice — the human-labeled set (via
  the `make_worksheet` CLI) is **pending**. The labeling worksheet exists; the labels
  do not yet.
- **Tiny n.** 8 patient/HCC pairs, 4 of them the holdout slice. A single
  misclassification swings every rate by double digits. Treat these as direction,
  not precision.
- **Detection recall lives entirely on the holdout slice.** The 4 over-codes are
  exactly the 4 `holdout` labels; `--exclude-holdout` leaves no positives to score.
- **Single, local model.** One provider/model — **`qwen2.5:7b-instruct` via
  Ollama** — single run, no ensembling, no temperature sweep.
- **Scope.** CMS-HCC **V28** only, **community continuing-enrollee** segment, four
  condition families. Demographics, interactions, and other segments are out of
  scope for the agent's substantiation judgment (the engine handles full scoring).
- **The extraction and calibration metrics are not yet informative** on this slice
  (§4.3, §4.4); read substantiation + span accuracy as the live signals.

These results are an **honest floor** that exercises the end-to-end pipeline on
known truth, not a claim about real-world auditing performance.

---

## 6. Reproducibility

All paths are relative to the repo root unless absolute. Cohort/audit data is
gitignored and lives only in the main worktree under `data/` (see
[`data/README.md`](data/README.md)); the engine V28 tables are provisioned per
[`harness/README.md`](harness/README.md) / `data/README.md`.

```bash
# The eval harness is Rust (crates/eval) — no Python env needed for metrics.
# (Only harness/crosscheck.py, which drives CMS's own Python reference, needs pandas.)
TABLES=data/cms_hcc_v28/python_v28/software/CMS_HCC_v28/data/input/internal

# (0) Deterministic synthetic cohort — fixed pop/seed, reproduces identically
scripts/generate_cohort.sh                       # POP=400 SEED=20260602 (Java 17+)

# (1) Injected known-truth eval slice (FHIR bundles + candidates + gold labels)
cargo run -p eval --bin inject_errors -- \
  --fhir-out       data/integration/injected_fhir \
  --candidates-out data/integration/injected_candidates.jsonl \
  --gold-out       data/integration/injected_gold.jsonl \
  --holdout-every 2                              # flags every 2nd label holdout

# (2) Agent audit run → audit_results.jsonl  (local Ollama; qwen2.5:7b-instruct)
#     usage: audit_jsonl <v28_tables_dir> <crosswalk.csv> <fhir_dir> <out.jsonl> [N]
export HCC_MODEL=qwen2.5:7b-instruct
cargo run --example audit_jsonl -- \
  "$TABLES" crosswalks/snomed_to_icd10_v28.csv \
  data/integration/injected_fhir data/integration/injected_audit.jsonl 8

# (3) Metrics → the committed artifacts in harness/results/. The candidate set is
#     derived by calling the engine directly (single source of truth — no replica,
#     so no reconcile step), via --tables; --fhir enables span metrics.
cargo run -p eval --bin hcceval -- \
  --audit      data/integration/injected_audit.jsonl \
  --gold       data/integration/injected_gold.jsonl \
  --candidates data/integration/injected_candidates.jsonl \
  --tables     "$TABLES" \
  --crosswalk  crosswalks/snomed_to_icd10_v28.csv \
  --fhir       data/integration/injected_fhir \
  --json harness/results/integration_metrics.json \
  --md   harness/results/integration_metrics.md
#   add --exclude-holdout for the holdout-removed read in §2.
```

The Phase-1 engine itself is validated against CMS's published V28 reference
software by `harness/crosscheck.py` (risk scores agree to 3 decimals); see
[`harness/README.md`](harness/README.md).
