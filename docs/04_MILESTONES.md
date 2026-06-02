# Milestones

Phased so each checkpoint is a finishable, demonstrable artifact. The project has portfolio value at the end of Phase 3; Phases 4–5 are upside. **Resist scope creep** (see overview) — a finished narrow system beats an unfinished broad one.

## Phase 0 — Setup & decisions (small, do first)
- [x] Pick and **pin one HCC model version**. Document why. (Do not support multiple versions in v1.)
- [x] Choose the narrow HCC subset to target (a handful of common, well-documented categories — enough to be real, few enough to finish).
- [x] Stand up the repo with `DATA.md`, license notes, and the synthetic-only statement.
- [x] Generate a first small Synthea cohort with notes enabled; confirm notes are rich enough to reason over.

**Done = repo exists, data pipeline produces note-bearing synthetic records.**

## Phase 1 — Deterministic engine (the truth side)
- [x] ICD-10 → HCC mapping for the pinned version and chosen subset.
- [x] HCC → risk-score calculation incl. hierarchy logic and demographic factors.
- [x] Provenance output: which diagnosis drove which HCC.
- [x] Compile-checked model-version typing.
- [x] Unit tests on the scoring logic; a few hand-computed cases as fixtures.

**Done = given diagnoses + demographics, the engine returns a correct, decomposable score. This alone is a credible domain artifact.**

## Phase 2 — Extraction + single-pass assessment (no loop yet)
- [x] LLM extraction: note → documented conditions + supporting spans.
- [x] Wire extraction output into the engine to produce a candidate HCC set + score.
- [x] First-pass substantiation judgment (single prompt, no self-correction).
- [x] Emit flags with regulatory citations.

**Done = end-to-end single pass: note in, flagged-code report out.**

## Phase 3 — The agent loop + evaluation (the credibility core)
- [x] Add the VERIFY step: cross-check LLM claims against engine output.
- [x] Add SELF-CORRECT: on disagreement, re-examine the specific code.
- [x] Build the labeled eval set (hand-label substantiation truth on a clean sample; inject controlled errors).
- [x] Implement the metric set from `03_EVALUATION_HARNESS.md`.
- [x] Write the results table + failure-mode analysis + "what the oracle caught" section.

**Done = a measured, self-correcting agent with published numbers. THIS IS THE PORTFOLIO DELIVERABLE.** Stop here and polish if time is short.

## Phase 4 — Packaging & narrative (cheap, high ROI)
- [x] Clean README: the thesis, the architecture diagram, the results, the honest limitations.
- [ ] A short walkthrough (recorded demo or annotated example) showing one record flowing through the loop.
- [x] Narrative / framing notes (kept in a separate private doc).

**Done = an industry engineer grasps the value in two minutes — and could take it further for applied research.**

## Phase 5 — Optional upside (only after 3–4 are solid)
- [ ] Expose the engine as an **MCP server** (fourth domain-differentiated MCP tool, connects to the existing three).
- [ ] Second HCC model version → demonstrate the version-migration / drift story.
- [ ] Population-level run with score-drift decomposition (the reconciliation story at scale).

**Each Phase-5 item is independently shippable. None should block the Phase-3 deliverable.**

## Sequencing logic
Truth side before probabilistic side (Phase 1 before 2) — you cannot ground an agent against an oracle you haven't built. Loop and eval together (Phase 3) — the self-correction loop is only meaningful if you can measure that it helps. Narrative last and cheap (Phase 4) — but do not skip it; an unread project has no credibility value.

---

## BUILD STATUS — completed (as built)

Repo: `github.com/NahButch/hcc-substantiation-auditor`. Phases 0–3 complete; end-to-end pipeline runs and is measured. As-built notes where it differs from the plan above:

- **`eval` is a Rust crate** (`crates/eval`), ported from the Python prototype; `harness/crosscheck.py` (and `fuzz_engine.py`) stay Python only to drive CMS's own reference software. The plan assumed the harness stayed Python — the Rust port is an improvement.
- **`crosswalks/`** dir added for SNOMED→ICD-10 / ICD-10→HCC mapping data.
- **LLM layer = local model** (`qwen2.5:7b-instruct` via Ollama) behind the swappable interface — concrete choice the plan left open. Keeps it free and private; single run, temperature 0.
- **Engine** implements mapping, hierarchy, constraining, demographic factors, **disease interactions**, and provenance — verified against CMS reference via `harness/crosscheck.py`.
- Phase 0–2 ran as one sequential track; Phase 3's parallel fan-out was not needed at this scale. (The worktree guidance remains valid for future expansion.)

See `08_BUILD_RESULTS_SUMMARY.md` for the measured outcomes.
