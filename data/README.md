# Data Provenance

## Purpose & Policy

The `data/` directory holds downloaded reference files and synthetic patient data used for local development, evaluation, and testing of the HCC substantiation auditor.

**Synthetic data only.** This project does not use real patient records at any stage. All patient-level data is either machine-generated (Synthea) or algorithmically derived from Synthea outputs. See [../DATA.md](../DATA.md) for the full data-use and privacy policy.

Large binary files (ZIPs, FHIR bundles, SAS/CSV model outputs) are `.gitignore`d. Only scripts, configuration files, and this README are committed to version control. To recreate the local data directory from scratch, run the acquisition script described in the [Reproducibility](#reproducibility) section below.

---

## Directory Layout

```
data/
├── README.md                  ← this file (committed)
├── cms_hcc_v28/               ← CMS-HCC V28 model software + crosswalk (gitignored)
│   └── <contents of CMS ZIP>  ← filled after manual download; see Source 1
├── synthea/
│   └── coherent/              ← MITRE Coherent dataset, FHIR R4 bundles (gitignored, ~9 GB)
└── synthea_config/            ← Synthea run configuration (pending Phase-0c approval)
    └── synthea.properties     ← not yet committed; drives local generation when unblocked
```

---

## Source 1 — CMS-HCC V28 Model Software + ICD-10-CM → HCC Mapping

| Field | Detail |
|---|---|
| **Page URL** | https://www.cms.gov/medicare/payment/medicare-advantage-rates-statistics/risk-adjustment/2026-model-software-icd-10-mappings |
| **Payment year** | 2026 (PY2026) |
| **Model line** | V28 — operative for 2026 risk adjustment |
| **Acquisition** | MANUAL browser download required (see below) |
| **License** | US-Government public-domain work — 17 U.S.C. §105. No registration, login, or form submission required. |

### CMS distributes TWO separate downloads (important)

The CMS page hosts two distinct artifact families. They are **not** the same ZIP:

1. **ICD-10-CM Mappings** ZIP — the ICD-10-CM → HCC crosswalk only. **✓ OBTAINED.**
2. **Model Software** ZIP (SAS / emerging Python) — the **coefficient tables, HCC hierarchy definitions, demographic factor tables, and interaction/constraining specs**. **✗ STILL NEEDED** — Phase 1 scoring cannot be computed or CMS-validated without it.

### Artifacts obtained (mappings — Source 1a)

Downloaded manually to `data/cms_hcc_v28/` on 2026-06-01:

| File | Extracted to | Notes |
|---|---|---|
| `2026-midyear_final-icd-10-cm-mappings.zip` | `midyear_final/` | **Primary — pinned for V28.** Contains `2026 Final ICD-10-CM Mappings.{csv,xlsx}`, 11,879 rows, covers FY2025–FY2026 codes. |
| `2026-initial-icd-10-cm-mappings_0.zip` | `initial/` | Initial-run variant; kept for reference, not used as the pinned source. |

**CSV column layout** (header spans row 4, after 3 title rows): col 0 `Diagnosis Code`, col 1 `Description`, col 5 `CMS-HCC Model Category V28`, col 10 `CMS-HCC Model Category V28 for 2026 Payment Year` (Yes/No). Also carries ESRD V21/V24, CMS-HCC V22, and RxHCC V08 columns — **ignore all non-V28 columns** (we pin V28 only).

Confirmed V28 coverage for the Phase-0c target families: Diabetes → HCC 36/37/38 (+263/298/383 complications); CKD → 326/327/328/329; Heart Failure → 222/224/225/226; COPD → 280.

### Model software obtained (coefficients + hierarchy — Source 1b)

Downloaded manually to `data/cms_hcc_v28/` on 2026-06-01 (both SAS and Python distributions). Each is an outer ZIP of per-model nested ZIPs:

| File | V28 nested package | Used? |
|---|---|---|
| `python-2026-midyear_final-model-software.zip` | `CMS_HCC_v28_2026_T_package_v3.zip` | **YES — pinned reference oracle** (extracted to `python_v28/`) |
| `2026-midyear_final-model-software.zip` | `CMS-HCC software V2826.115.T1.zip` (SAS) | reference / cross-check |
| `2026-initial-model-software.zip` | `CMS-HCC software V2825.115.T2.zip` (SAS, initial run) | reference only |

The pinned V28 Python package (`python_v28/software/CMS_HCC_v28/`) provides everything Phase 1 needs, all as CSVs under `data/input/internal/`:

| File | Role |
|---|---|
| `ICD10_CC_mappings_CMS_HCC_2026_v28.csv` | ICD-10-CM → condition category (CC) mapping |
| `V28_Diagnosis_Categories.csv` | disease-group variable → member HCCs (e.g. `DIABETES_V28`→HCC35/36/37/38) |
| `V28_HCC_Hierarchies.csv` | hierarchy / trumping rules (HCC → secondary HCCs it suppresses) |
| `V28_Interactions.csv` | disease-interaction terms (incl. `DIABETES_HF_V28`, `HF_KIDNEY_V28`, `HF_CHR_LUNG_V28`) |
| `V28_CE_Relative_Factors.csv` | Continuing-Enrollee coefficients (community segments + institutional + demographic cells) |
| `V28_NE_Relative_Factors.csv` | New-Enrollee coefficients |

`transform.py` / `config.py` / `common/CMS_HCC_utils.py` are CMS's own reference scoring logic — **the oracle the Rust engine is validated against in Phase 1.** `user_runbook.md` documents how to run it.

---

## Source 2 — Synthea Synthetic Patients

### B1 — MITRE Coherent Data Set (default / scriptable)

| Field | Detail |
|---|---|
| **AWS Open Data registry** | https://registry.opendata.aws/synthea-coherent-data/ |
| **S3 path** | `s3://synthea-open-data/coherent/` |
| **Approximate size** | ~9 GB |
| **Contents** | FHIR R4 patient bundles including `DocumentReference` (clinical notes), CCDA exports, and CSV summaries. Notes are template-generated ("simple") — not LLM-enriched. See Known Blockers below if richer notes are needed. |
| **License** | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) — MITRE Corporation. Attribution required in any derivative work or publication. |
| **Acquisition** | `aws s3 sync --no-sign-request` — no credentials needed. Gated behind `FETCH_COHERENT=1` env var to avoid accidental 9 GB download. |

To download:

```bash
FETCH_COHERENT=1 bash scripts/fetch_data.sh
```

If the AWS CLI is not installed:

```bash
pip install awscli
```

Single-ZIP alternative (if sync is not preferred):

```bash
aws s3 cp --no-sign-request \
    s3://synthea-open-data/coherent/<zip-filename> \
    data/synthea/coherent/
```

### B2 — Local Synthea Generation (optional; requires Java 17+)

| Field | Detail |
|---|---|
| **Repository** | https://github.com/synthetichealth/synthea |
| **License** | Apache-2.0 |
| **Java requirement** | Java 17+ |
| **Status on this machine** | **UNBLOCKED.** Temurin JDK 17 (17.0.19) installed (no sudo) at `/home/tom_b/jdk17`. The system default `java` is still Java 8, so run Synthea with `JAVA_HOME=/home/tom_b/jdk17` (or call `/home/tom_b/jdk17/bin/java` directly). |

Clinical notes are emitted via the US Core IG exporter (`exporter.fhir.use_us_core_ig=true`). Run configuration lives in `data/synthea_config/synthea.properties` (pending Phase-0c approval; not yet committed).

Reference commands are provided as commented-out blocks inside `scripts/fetch_data.sh` — do not execute until Java 17+ is confirmed and the Phase-0c configuration is approved.

### chatty-notes (DEFERRED)

| Field | Detail |
|---|---|
| **Repository** | https://github.com/synthetichealth/chatty-notes |
| **License** | Apache-2.0 |
| **Purpose** | Post-processes Synthea FHIR output through an LLM to produce richer, less template-ish clinical notes |
| **Status** | **DEFERRED** — requires an OpenAI API key and a small SDK patch; not yet integrated |

---

## Known Blockers / Decisions Pending

| Blocker | Detail | Resolution path |
|---|---|---|
| ~~Java 8 on this machine~~ | RESOLVED — Temurin JDK 17 installed at `/home/tom_b/jdk17`; use `JAVA_HOME=/home/tom_b/jdk17` for Synthea. | — |
| ~~CMS V28 Model Software not downloaded~~ | RESOLVED — mappings + SAS + Python model software all obtained; V28 Python package pinned as oracle. | — |
| **Coherent note richness** | Coherent `DocumentReference` notes are "simple"/template-ish. For realistic NLP targets, chatty-notes LLM enrichment may be needed. | Evaluate Coherent notes first; defer chatty-notes unless quality is insufficient. |
| **Disease-mix / cohort configuration** | `data/synthea_config/synthea.properties` (HCC-relevant conditions, population size, state) is not yet committed. | Pending Phase-0c approval. |

---

## Reproducibility

To recreate the data directory:

```bash
# Step 1: print guidance only (safe, downloads nothing)
bash scripts/fetch_data.sh

# Step 2: fetch CMS V28 (after obtaining URL from cms.gov browser visit)
CMS_V28_ZIP_URL='<url>' bash scripts/fetch_data.sh

# Step 3: fetch Coherent dataset (~9 GB, requires AWS CLI)
FETCH_COHERENT=1 bash scripts/fetch_data.sh

# Step 4: both at once
CMS_V28_ZIP_URL='<url>' FETCH_COHERENT=1 bash scripts/fetch_data.sh
```

For local Synthea generation (B2), install Java 17+ first, ensure `data/synthea_config/synthea.properties` is committed, then follow the commented-out reference commands in `scripts/fetch_data.sh`.
