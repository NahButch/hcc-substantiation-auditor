# Test fixtures — CMS V28 table subset

These CSVs are a **curated subset** of the CMS-published CMS-HCC **V28** model
tables (payment year 2026), extracted verbatim from the official CMS Risk
Adjustment Model Software (Python package). They are committed here so the engine's
unit tests are self-contained and reproducible.

- **Provenance:** U.S. Government work, public domain under 17 U.S.C. §105. No
  values were hand-authored or altered — rows were filtered, not edited.
- **Scope:** rows for the project's target HCC families only — diabetes
  (HCC 35/36/37/38), heart failure (221–227), CKD (326–329), chronic lung
  (276–280), cardiorespiratory failure (211–213), plus HCC 238; all interaction
  rows; age/sex, count, and demographic coefficient rows.
- **Why a subset:** unit tests assert hand-computed scores against these exact
  coefficients. Agreement with the *full* model on the *full* table set is
  validated separately by the Python-reference cross-check harness.

Format matches the CMS files exactly, so `Engine::load_v28(<this dir>)` loads them
the same way it loads the full tables.
