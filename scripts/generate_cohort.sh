#!/usr/bin/env bash
# =============================================================================
# generate_cohort.sh — Reproducible Synthea synthetic cohort generation.
#
# Produces note-bearing synthetic patients (US Core DocumentReference clinical
# notes + structured CSV conditions) for the HCC substantiation auditor.
# Synthetic data only — see ../DATA.md.
#
# Deterministic: fixed population size, population seed, and clinician seed, so
# re-running reproduces the identical cohort. Requires Java 17+ (this machine:
# /home/tom_b/jdk17; the system default java is 8).
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

JAVA="${JAVA17:-/home/tom_b/jdk17/bin/java}"
JAR="$REPO_ROOT/data/synthea/synthea-with-dependencies.jar"
OUT="$REPO_ROOT/data/synthea/cohort"

POP="${POP:-400}"
SEED="${SEED:-20260602}"
CLINICIAN_SEED="${CLINICIAN_SEED:-20260602}"
STATE="${STATE:-Massachusetts}"

echo "Generating $POP patients (seed=$SEED) → $OUT"
"$JAVA" -jar "$JAR" \
  -p "$POP" -s "$SEED" -cs "$CLINICIAN_SEED" \
  --exporter.baseDirectory "$OUT" \
  --exporter.fhir.export true \
  --exporter.fhir.use_us_core_ig true \
  --exporter.csv.export true \
  --exporter.csv.folder_per_run false \
  "$STATE"

echo "Done. FHIR bundles + CSV in $OUT"
