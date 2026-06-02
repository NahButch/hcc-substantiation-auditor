# Data Policy

## Synthetic data only

No real patient data, personal health records, or protected health information
(PHI) are ever ingested, requested, stored, or committed in this repository.
All development and evaluation uses exclusively synthetic data.

## Permitted data sources

| Source | Description | License |
|--------|-------------|---------|
| [Synthea](https://github.com/synthetichealth/synthea) | Synthetic patient record generator | Apache-2.0 |
| CMS public model files | CMS-HCC V28 ICD-10-to-HCC mappings and risk coefficients | Public domain (CMS) |

## Storage and reproducibility

Large data files (Synthea output ZIPs/CSVs, downloaded CMS mapping tables) live
in `data/` and are gitignored. Only generation configs, seed files, and fetch
scripts are committed, so the full dataset can be reproduced from scratch by
running `scripts/fetch_data.sh` (added in Phase 0b).

## Out of scope

Real-data benchmarks — including **CodiEsp** and **MIMIC-IV** — are
intentionally excluded from this project.

> [!CAUTION]
> Including them would require **data-use agreements and IRB processes** —
> outside the scope of this research and educational demonstration, and not to be
> attempted without that approval.
