# Build Results Summary (as built)

Captured from the completed repo (`github.com/NahButch/hcc-substantiation-auditor`, `RESULTS.md`). All figures are an honest **floor** on synthetic, partly by-construction data — not a real-world performance claim.

## The headline result (lead with this)

**The deterministic engine agrees exactly with CMS's own reference software** — to 3 decimals across **2,100 randomized synthetic claims** (full ICD-10/HCC set, random age/sex/OREC/Medicaid), via a differential fuzzer that runs the same inputs through both the Rust engine and CMS's `transform.py`. The fuzzer **caught a real bug** the 10 hand-picked validation cases never exercised: negative-age beneficiaries (born after the Feb-1 payment-year cutoff) were being defaulted into the `0_34` age/sex cell instead of receiving no cell. Fixed, regression-tested, re-verified.

This is the strongest single line in the project: it's a verifiable, exact-agreement correctness claim against the government reference, and it demonstrates differential fuzzing surfacing a bug that example-based tests missed — a mature testing instinct.

## Agent / audit metrics (36 patients, 47 coded HCCs vs gold)

Gold = 8 injected known-truth pairs (by construction) + 39 single-reviewer manual annotations (LLM-assisted, not a credentialed RADV coder).

| Metric | Value | Read |
|---|---|---|
| Substantiation accuracy | 87.2% | agreement with gold (binary flag) |
| Flag precision / recall / F1 | 88.1% / 97.4% / 0.925 | catches almost every over-code |
| Over-coding rate | 2.1% | the expensive RADV error — 1 missed |
| Over-flagging rate | 55.6% | 5 of 9 supported codes wrongly flagged |
| Disagreement-resolution | 25% (1 improved, 3 regressed) | oracle loop net-negative here |
| Extraction P / R / F1 | 72.7% / 17.0% / 0.276 | recall is the weak spot |
| Span accuracy | 59.7% | evidence quotes found in the note |
| Citation validity | 100% | citations match a real regulatory authority |
| Hallucination rate | 40.3% | evidence quotes with no basis in the note |
| Calibration (ECE) | 0.193 (n=14) | over-confident in the top bin |

Confusion (47 pairs): TP=37, FP=5, FN=1, TN=4. Positive class = "not substantiated".

## How to talk about these numbers (they're a strength, not a weakness)

The honest, mixed result is *more* credible than a clean sweep — and every weak number has a clear, domain-grounded explanation that demonstrates understanding:

- **Detection is strong, in the direction that matters.** 97.4% recall / 2.1% over-coding means it catches almost every unsupported code — the financially expensive RADV direction. That's the right error profile for an audit-defense tool.
- **It over-flags (55.6%).** Errs toward "flag" — safe for RADV exposure, high clinician burden. A real, namable tradeoff, not a mystery.
- **The oracle loop is currently net-negative** (3 regressed, 1 improved). Stated plainly as an open issue: the self-correct prompt needs tuning on borderline cases before the loop is trusted. *Reporting a feature that didn't work yet is exactly the rigor that builds trust.*
- **Extraction recall is low (17%)** because Synthea buries chronic conditions in the HPI past-history list rather than the Assessment & Plan. The ICD-10 backfill made this *measurable* (was 0%); the low value is a real finding about both the notes and the extractor — not a silent failure.
- **Hallucination 40%** = the model paraphrases evidence quotes rather than quoting verbatim; citations, scored against an allow-list, are 100% valid. Precise about *which* kind of grounding failed.

The interview move: lead with the exact-CMS-agreement and the fuzzer-caught bug (unambiguous engineering win), then walk the agent metrics as a candid error-profile analysis — what works, what doesn't, and exactly why, in domain terms. That candor, plus knowing the worst failure mode cold, is the credibility signal.

## As-built environment / scope

- CMS-HCC **V28**, community continuing-enrollee segments (NA/ND); New-Enrollee and institutional models are future work.
- LLM: `qwen2.5:7b-instruct` via Ollama, single run, temperature 0.
- n=47 scored pairs — small; a single flip swings rates by points. Treat as direction, not precision.
- Fully reproducible: scripted cohort generation (seeded), fuzz, audit, gold build, and metrics (commands in `RESULTS.md` §6).

## Open items / natural next steps (the Phase-5 vision, now evidence-backed)

- Tune the self-correct prompt so the oracle loop becomes net-positive (current top open issue).
- Improve extraction recall (richer notes via chatty-notes; target the history-list problem).
- Add a credentialed-coder gold slice to replace single-reviewer labels.
- Expand engine scope (New-Enrollee, institutional models).
- V24→V28 drift analysis; MCP-server wrapper (the original Phase-5 items).
