# Synthetic Datasets & Benchmarks for Testing

Compiled for the RADV Audit-Defense Agent project. Two needs: (A) **note-bearing records** the agent reasons over, and (B) **ground truth** to score against. The best setup combines a synthetic note source with the *official* CMS scoring logic as the oracle. All current as of the build date below — re-verify links when you start, CMS reorganizes URLs.

---

## Tier 0 — The ground-truth oracle (most important find)

### Official CMS-HCC Model Software + ICD-10 Mappings — *free, authoritative*
CMS publishes the actual risk-adjustment model: the ICD-10-CM → HCC mappings **and** the coefficient/scoring logic (historically as SAS), per payment year. This is not a third-party approximation — it *is* the scoring standard. Your deterministic Rust engine reimplements this, and you validate your engine against CMS's own published logic.

- 2026 / 2025 / 2024 Model Software & ICD-10 Mappings — CMS Risk Adjustment pages (one page per payment year).
- The NBER "Risk Adjustment" page documents the model's expected input structure: a **PERSON** file (enrollment/demographics) and a **DIAG** file (diagnoses), producing risk scores — exactly the input/output contract your engine needs.

**Why this matters for measurement:** it gives you *mechanical ground truth for free*. Feed the same diagnoses to (a) your engine and (b) the CMS reference logic; they must agree. That's a precision/correctness check on the deterministic half before the LLM is even involved.

**Version note (now confirmed):** **V28 is fully operative as of Jan 1, 2026** (100% of MA risk scores). V28 dropped valid ICD-10-CM codes from 9,797 (V24) to 7,770, expanded HCCs from ~86 to ~115, and applies "constraining" (related HCCs share coefficients). **Pin V28** for v1 — it's current, and the V24→V28 delta is your ready-made Phase-5 "version migration / drift" story.

---

## Tier 1 — Note-bearing synthetic records (the agent's input)

### Synthea (primary, recommended)
Fully synthetic patients with complete histories and **clinical notes**; FHIR/C-CDA/CSV; Apache-2.0; no privacy restrictions. You control the disease mix so target HCCs actually appear.
- `synthetichealth/synthea` — generator.
- `synthetichealth/chatty-notes` — richer LLM-generated notes from Synthea bundles (harder, more credible substantiation task).
- **Coherent Data Set** (MITRE, AWS open data) — large prebuilt Synthea export with notes if you'd rather not run the generator.

**Expected results:** you *derive* ground truth — run structured diagnoses through your engine (validated against CMS logic) to get the canonical HCC set + score per patient. So Synthea gives you data **and** mechanical truth in one pipeline. The only hand-labeling needed is the substantiation layer (see Tier 3).

### MedSyn synthetic clinical notes (open)
Large open synthetic clinical-note set (~41k notes over 219 ICD-10 codes). **Caveat: it's Russian-language** — useful as a structural reference or for the extraction sub-task, less so for English RADV. Note it exists; probably not your primary.

---

## Tier 2 — Real (de-identified) note→code datasets *with published benchmark numbers*

These are **not synthetic** and require credentialed access (PhysioNet DUA, training). Use only if you choose to validate against real-world distributions — but their value is the **published precision/recall/F1 baselines you can target**. Stay synthetic-only if you want zero access friction; these are optional rigor.

### MIMIC-IV (note → ICD-10), credentialed
~330k+ de-identified free-text notes with corresponding ICD-10 codes; avg ~5.8 codes/note. The standard academic benchmark for automatic ICD coding.
- **Published baselines to measure against:** multi-label macro/micro/weighted **F1** and **AUROC**, plus **Precision/Recall@k** (k=5,10,20). One recent fine-tune reported exact-match **F1 ≈ 0.18 zero-shot → >0.70 after fine-tuning** — concrete numbers your extraction layer can be compared to.
- Access: PhysioNet credentialing + DUA. Real data, so out of scope if you're holding the synthetic-only line.

### CodiEsp (CLEF eHealth 2020) — gold-standard annotated, openly available
1,000 clinical cases (translated to English), expert-annotated with ICD-10-CM **and** PCS codes, with **text references** (the span that justifies each code). Train/dev/test split published.
- **Why it's a strong fit:** the *text-reference* annotations mirror your "span accuracy / grounding" metric, and there are **published shared-task precision/recall/F1 leaderboards** to benchmark against. Lighter access burden than MIMIC.

### aci-bench — open dialogue→note benchmark
Largest open doctor-patient-dialogue → visit-note corpus, with published SOTA baselines. Relevant only if you ever add a note-*generation* step; not core to coding/substantiation. Listed for completeness.

---

## Tier 3 — The labels only you can author (the differentiator)

No public dataset labels **HCC substantiation under RADV documentation standards** (the M.E.A.T. / "Condition + Causality + Status + Plan", specificity, laterality judgment). Commercial tools (John Snow Labs GenAI Lab, ForeSee, etc.) do this proprietarily; there's no open gold set. **That gap is your opportunity:** hand-label a small, clean substantiation set over Synthea notes using your domain expertise. This is the rare asset — author it, document the rubric, and it becomes the thing your eval is measured against.

- Anchor the rubric to CMS's published documentation expectations and the V28 mapping files (Tier 0) so your labels are defensible, not arbitrary.

---

## Recommended testing stack (the practical answer)

1. **Oracle:** reimplement CMS V28 mappings/coefficients (Tier 0); validate your Rust engine against CMS's own logic → *mechanical ground truth, free.*
2. **Records:** generate a Synthea cohort with notes (Tier 1), disease mix chosen to surface target HCCs + near-misses.
3. **Extraction metrics:** score note→diagnosis extraction; optionally sanity-check method against **CodiEsp's** published P/R/F1 to see if your extraction is in a sane range.
4. **Substantiation metrics:** score against your hand-labeled set (Tier 3) — the metric no public dataset provides and the heart of the project.
5. **(Optional rigor):** if you ever want a real-world reference point, cite MIMIC-IV/CodiEsp published baselines rather than ingesting real data.

**Bottom line on "expected results to measure against":**
- *Deterministic side* — exact agreement with CMS published model logic (Tier 0). Pass/fail, not fuzzy.
- *Extraction side* — comparable to CodiEsp/MIMIC published P/R/F1 leaderboards (Tier 2).
- *Substantiation side* — your authored gold set (Tier 3); no external baseline exists, which is precisely why building it is credible.

---
*Compiled 2026-06-01.*

---

## Government Sources (the policy this project implements)

Two distinct policy layers — the scoring model (what the deterministic engine implements) and the audit/documentation standard (what the agent's substantiation judgment implements). Verify links when you start; CMS reorganizes URLs.

### Layer 1 — The scoring model (deterministic engine)
CMS Risk Adjustment, Model Software + ICD-10 Mappings, per payment year:
- Hub: `https://www.cms.gov/medicare/payment/medicare-advantage-rates-statistics/risk-adjustment`
- 2026 (V28 — pin this): `https://www.cms.gov/medicare/payment/medicare-advantage-rates-statistics/risk-adjustment/2026-model-software-icd-10-mappings`
- 2025: `https://www.cms.gov/medicare/payment/medicare-advantage-rates-statistics/risk-adjustment/2025-model-software/icd-10-mappings`
- 2024: `https://www.cms.gov/medicare/health-plans/medicareadvtgspecratestats/risk-adjustors/2024-model-software/icd-10-mappings`
- These pages carry the ICD-10→HCC mapping files and the coefficient/scoring logic (historically SAS) — the standard the engine reimplements and validates against (mechanical ground truth).

### Layer 2 — The audit + documentation standard (agent substantiation judgment)
- **RADV Final Rule (CMS-4185-F2)**, Federal Register, eff. April 3, 2023: `https://www.federalregister.gov/documents/2023/02/01/2023-01942/medicare-and-medicaid-programs-policy-and-technical-changes-to-the-medicare-advantage-medicare`
- CMS fact sheet: `https://www.cms.gov/newsroom/fact-sheets/medicare-advantage-risk-adjustment-data-validation-final-rule-cms-4185-f2-fact-sheet`
- The rule codifies how CMS validates that submitted diagnoses comply with documentation requirements, determines improper payments, and recoups them — essentially the tool's job description.
- The documentation-sufficiency concepts the agent reasons with (M.E.A.T., valid data sources, specificity) trace to CMS risk-adjustment guidance and the annual Rate Announcements / Advance Notices, not a single statute. This diffuseness is exactly why the hand-labeled substantiation set is the rare asset.

### Important status note (cite this, it strengthens the framing)
The 2023 RADV Final Rule was **vacated on procedural (APA) grounds** in *Humana v. Becerra* (N.D. Tex., Sept. 25, 2025) — the court found CMS's evidence→policy reasoning (eliminating the fee-for-service adjuster) was not a "logical outgrowth" of its proposed rule. It did not rule on the substance. This is a live example of the regulatory feedback loop breaking at the analyze→propose edge, and motivates the project's thesis (see `07_POLICY_FEEDBACK_LOOP.md`). Because the policy is in flux, treat RADV specifics as a moving target and re-verify current status before relying on any particular provision.
