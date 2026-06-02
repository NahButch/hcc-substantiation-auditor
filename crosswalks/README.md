# Crosswalks

## `snomed_to_icd10_v28.csv` — SNOMED CT → ICD-10-CM (target families)

Synthea codes conditions in **SNOMED CT**, but the CMS-HCC V28 engine consumes
**ICD-10-CM**. This is a small, curated crosswalk covering only the SNOMED codes
Synthea emits for the project's target families (diabetes, CKD, heart failure,
COPD), each mapped to a clinically-equivalent ICD-10-CM code.

### How it was built (and why it's trustworthy)
- The **SNOMED side** is the exact set of codes observed in the generated cohort
  (`scripts/generate_cohort.sh`), filtered to the target families.
- The **ICD-10 side** is a standard clinical equivalent for each SNOMED concept.
- The **`expected_v28_hcc`** column is **verified** against the CMS V28 package
  mapping the engine actually loads (`ICD10_CC_mappings_CMS_HCC_2026_v28.csv`) —
  it is not hand-asserted. An empty value means the code intentionally maps to
  **no HCC** (a clinical near-miss).

### Near-misses (deliberate)
Prediabetes, CKD stages 1–2, and renal-transplant status map to ICD-10 codes that
carry **no V28 HCC**. These are the "documented but not HCC-eligible" cases that
let the Phase 3 evaluation measure real substantiation judgment rather than
code-echoing.

### Known simplification
Proliferative diabetic retinopathy's specific ICD-10 code is absent from the V28
package mapping, so it is collapsed to a diabetic-retinopathy code that is present
(both still resolve to a diabetes chronic-complication HCC). Noted in the row.

### Scope / limitations
Covers the four target families only. Conditions outside these families (or SNOMED
codes not listed here) are dropped when deriving ground-truth ICD-10 from the
synthetic cohort — by design, since the engine and evaluation are scoped to these
families in v1.
