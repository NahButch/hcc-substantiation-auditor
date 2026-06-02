# Data Sources

## The constraint that drives everything

**Never use real patient data, and never anyone's real medical or claims records.** This is an ethical/privacy line: a project built openly on synthetic data is publishable and shareable. Stated up front, it is a strength.

## The key data design problem

The agent needs to reason over **clinical documentation (free text / notes)** to judge whether a code is *substantiated*. That requirement splits the data sources cleanly:

- **Claims-level synthetic data (DE-SynPUF)** is structurally realistic for *codes and claims* but contains **no clinical notes** — there's no documentation for the agent to reason about substantiation against. It is also coarsened/synthetic enough that CMS itself notes it has limited inferential value.
- **Synthea** generates full synthetic medical histories **including clinical notes** (via `DocumentReference` / `DiagnosticReport` for notes when US Core is enabled), exported as FHIR, C-CDA, or CSV — and it's explicitly free of cost, privacy, and security restrictions.

**Conclusion: Synthea is the primary data source for this project.** DE-SynPUF is an optional secondary source for stress-testing the deterministic engine against realistic claims *structure*, but it cannot drive the substantiation-judgment task.

## Primary: Synthea

What it provides:
- Per-patient FHIR bundles with Conditions, Encounters, Observations, Procedures, MedicationRequests, etc., grouped by encounter in chronological order.
- **Clinical notes** as `DocumentReference` / `DiagnosticReport` resources (requires US Core enabled in config).
- Deterministic generation from open modules — any number of patients can be generated, with full control of the disease mix.
- Apache-2.0 tooling; outputs usable without legal/privacy concern.

Useful related repos (verify current state at the outset — these move):
- `synthetichealth/synthea` — the generator itself.
- `synthetichealth/chatty-notes` — a tool for generating clinical notes from Synthea FHIR bundles (was actively updated as of mid-2025). This is directly relevant: richer notes = a harder, more credible substantiation task.
- The **Coherent Data Set** (MITRE, on AWS open data) — a large prebuilt Synthea export including FHIR + simple clinical notes, for data without running the generator.

What to watch:
- Synthea populates required FHIR fields but few optional ones; note richness depends on config and on the notes tooling. Plan to tune the generator/notes config so documentation is rich enough that substantiation judgment is non-trivial.
- Synthea's condition prevalence is model-driven, not a real epidemiology — fine for this project, but don't claim population-level realism.

## Secondary (optional): DE-SynPUF

- CMS 2008–2010 Data Entrepreneurs' Synthetic Public Use File: Beneficiary Summary, Inpatient, Outpatient, Carrier, and PDE claims. Released in 20 samples (~0.25% each); structurally mirrors CMS Limited Data Sets.
- Use only to validate that the deterministic engine ingests realistic claims/diagnosis structures and demographic fields. Not for the notes-based agent task.

## Constructing the labeled evaluation set

This is the part that makes the eval credible, and it is where domain expertise produces something rare. See `03_EVALUATION_HARNESS.md` for how the labels feed the metrics. The data-side steps:

1. **Generate a Synthea cohort** with a deliberately chosen disease mix so the relevant HCCs actually appear (and so do near-misses — conditions that look HCC-eligible but aren't documented to standard).
2. **Derive engine ground truth.** Run the deterministic engine over the *structured* diagnoses to get the canonical HCC set and score per patient. This is mechanical truth — what the codes *do* produce under the pinned model version.
3. **Hand-label substantiation truth** on a sample. For each candidate HCC, a domain expert labels whether the *note text* actually substantiates the code under the relevant CMS documentation standard, or whether it would be an over-code / unsupported in a RADV review. This human-authored layer is the rare asset.
4. **Inject controlled error.** Deliberately create records where the structured code is present but the documentation is weak/absent, and vice versa — this is what lets the eval measure whether the agent catches real RADV-style problems rather than just echoing the codes.

Keep the labeled set **small but clean** — a few hundred carefully labeled candidate-HCC decisions beats thousands of noisy ones. The credibility is in the labeling rigor, not the volume.

## Data hygiene / repo rules

- Commit generation configs and seeds, not giant data dumps — make the cohort reproducible.
- Put a clear `DATA.md` in the repo stating: synthetic only, source, generation command, and the explicit statement that no real or personal health data is used anywhere.
- License-check anything redistributed (Synthea outputs and Coherent set have permissive terms; cite them).
