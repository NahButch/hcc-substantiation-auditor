#!/usr/bin/env bash
# =============================================================================
# fetch_data.sh — Reproducible data-acquisition script
# Project : hcc-substantiation-auditor
# Purpose : Acquire (A) CMS-HCC V28 model reference files and
#           (B) Synthea note-bearing synthetic patient data for local
#           development, evaluation, and testing.
#
# Policy  : This project uses SYNTHETIC DATA ONLY for patient records.
#           See ../DATA.md for the full data-use and privacy policy.
#
# Usage   : bash scripts/fetch_data.sh
#           # Prints guidance; downloads nothing by default.
#
#           CMS_V28_ZIP_URL=<url> bash scripts/fetch_data.sh
#           # Fetches CMS V28 ZIP from the supplied URL.
#
#           FETCH_COHERENT=1 bash scripts/fetch_data.sh
#           # Downloads the MITRE Coherent dataset from AWS (~9 GB).
#
#           CMS_V28_ZIP_URL=<url> FETCH_COHERENT=1 bash scripts/fetch_data.sh
#           # Both sections fully fetched.
# =============================================================================
set -euo pipefail

# ---------------------------------------------------------------------------
# Resolve repo root from the script's own location (works regardless of cwd)
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DATA_DIR="$REPO_ROOT/data"

echo "=== hcc-substantiation-auditor / fetch_data.sh ==="
echo "Repo root : $REPO_ROOT"
echo "Data dir  : $DATA_DIR"
echo ""

# ---------------------------------------------------------------------------
# Tracking flags — updated as sections run, used in the summary at the end.
# ---------------------------------------------------------------------------
CMS_FETCHED=0
COHERENT_FETCHED=0
SYNTHEA_LOCAL_BLOCKED=0


# ===========================================================================
# SECTION A — CMS-HCC V28 Model Software + ICD-10-CM → HCC Mapping
# ===========================================================================
# Source  : https://www.cms.gov/medicare/payment/medicare-advantage-rates-statistics/
#           risk-adjustment/2026-model-software-icd-10-mappings
# Contains: SAS (and emerging Python) model software, ICD-10-CM → HCC
#           crosswalk, and coefficient tables — all as flat CSV/TXT inside
#           the ZIP.
# License : US-Government public-domain works (17 U.S.C. §105).
#           No registration, login, or form submission required.
# Note    : cms.gov actively bot-blocks programmatic fetches, so the exact
#           ZIP filename/URL cannot be resolved without a browser.
#           Set CMS_V28_ZIP_URL to the URL obtained from that page.
# ===========================================================================

echo "--- Section A: CMS-HCC V28 ---"

# Overridable via environment; defaults to empty (guidance-only mode).
CMS_V28_ZIP_URL="${CMS_V28_ZIP_URL:-}"

if [[ -z "$CMS_V28_ZIP_URL" ]]; then
    echo ""
    echo "  [MANUAL STEP REQUIRED] CMS_V28_ZIP_URL is not set."
    echo ""
    echo "  cms.gov blocks automated downloads.  Please:"
    echo "  1. Open in a browser:"
    echo "     https://www.cms.gov/medicare/payment/medicare-advantage-rates-statistics/risk-adjustment/2026-model-software-icd-10-mappings"
    echo "  2. Locate the PY2026 Final (or Midyear) CMS-HCC V28 model"
    echo "     software ZIP — the ICD-10-CM→HCC crosswalk and coefficient"
    echo "     tables are bundled inside it."
    echo "  3. Right-click → Copy link address to obtain the direct URL."
    echo "  4. Re-run this script with the URL:"
    echo "       CMS_V28_ZIP_URL='<paste-url-here>' bash scripts/fetch_data.sh"
    echo "     OR manually save the ZIP to:"
    echo "       $DATA_DIR/cms_hcc_v28/"
    echo "  Skipping Section A (not an error)."
    echo ""
else
    # Verify curl is available
    if ! command -v curl &>/dev/null; then
        echo "  ERROR: curl is not installed. Install it and re-run."
        exit 1
    fi

    CMS_OUT_DIR="$DATA_DIR/cms_hcc_v28"
    mkdir -p "$CMS_OUT_DIR"

    # Derive a filename from the URL; fall back to a safe default.
    CMS_ZIP_FILE="$CMS_OUT_DIR/$(basename "$CMS_V28_ZIP_URL")"
    if [[ "$CMS_ZIP_FILE" == "$CMS_OUT_DIR/" || "$CMS_ZIP_FILE" == "$CMS_OUT_DIR" ]]; then
        CMS_ZIP_FILE="$CMS_OUT_DIR/cms_hcc_v28.zip"
    fi

    echo "  Downloading CMS V28 ZIP..."
    echo "    URL  : $CMS_V28_ZIP_URL"
    echo "    Dest : $CMS_ZIP_FILE"
    curl -L -o "$CMS_ZIP_FILE" "$CMS_V28_ZIP_URL"

    echo "  Unzipping..."
    unzip -o "$CMS_ZIP_FILE" -d "$CMS_OUT_DIR"

    echo "  Done. CMS V28 files at: $CMS_OUT_DIR"
    CMS_FETCHED=1
fi


# ===========================================================================
# SECTION B — Synthea Note-Bearing Synthetic Patients
# ===========================================================================
# Two sub-paths are provided.  B1 is the recommended default because local
# Synthea generation (B2) is BLOCKED on this machine due to Java 8 (requires
# Java 17+).
# ===========================================================================

echo "--- Section B: Synthea Note-Bearing Patients ---"
echo ""

# ---------------------------------------------------------------------------
# B1 — MITRE Coherent Data Set  (DEFAULT / scriptable)
# ---------------------------------------------------------------------------
# Registry : https://registry.opendata.aws/synthea-coherent-data/
# S3 path  : s3://synthea-open-data/coherent/
# Size     : ~9 GB
# Contents : FHIR R4 bundles including DocumentReference (clinical notes —
#            described as "simple"/template-generated, not LLM-enriched),
#            plus CCDA, CSV exports.
# License  : CC BY 4.0 — MITRE Corporation.  Attribution required.
#            https://creativecommons.org/licenses/by/4.0/
# Access   : Public, no credentials needed (--no-sign-request).
#
# Gated behind FETCH_COHERENT=1 to avoid an accidental ~9 GB download.
# Single-ZIP alternative (if preferred over sync):
#   aws s3 cp --no-sign-request \
#       s3://synthea-open-data/coherent/<zip-filename> \
#       "$DATA_DIR/synthea/coherent/"
# ---------------------------------------------------------------------------

echo "  -- B1: MITRE Coherent Data Set --"

FETCH_COHERENT="${FETCH_COHERENT:-0}"

if [[ "$FETCH_COHERENT" != "1" ]]; then
    echo "  Coherent download is opt-in (set FETCH_COHERENT=1 to download ~9 GB)."
    echo "  Skipping B1."
    echo ""
else
    echo "  FETCH_COHERENT=1 — proceeding with Coherent download (~9 GB)."

    if ! command -v aws &>/dev/null; then
        echo ""
        echo "  [SKIP] AWS CLI not found.  Install it with:"
        echo "    pip install awscli"
        echo "  Then re-run with FETCH_COHERENT=1."
        echo ""
    else
        COHERENT_OUT="$DATA_DIR/synthea/coherent"
        mkdir -p "$COHERENT_OUT"
        echo "  Syncing from s3://synthea-open-data/coherent/ ..."
        echo "  Destination: $COHERENT_OUT"
        aws s3 sync --no-sign-request \
            s3://synthea-open-data/coherent/ \
            "$COHERENT_OUT/"
        echo "  Coherent sync complete."
        COHERENT_FETCHED=1
    fi
fi

# ---------------------------------------------------------------------------
# B2 — Local Synthea Generation  (OPTIONAL; requires Java 17+)
# ---------------------------------------------------------------------------
# Synthea GitHub : https://github.com/synthetichealth/synthea
# License        : Apache-2.0
# Requirement    : Java 17+ (this machine has Java 8 — BLOCKED until upgraded)
#
# To unblock: sudo apt install openjdk-17-jdk   (or equivalent)
#
# The following commands are COMMENTED OUT as reference only.
# They should NOT be executed until Java 17+ is confirmed on this machine,
# and Phase-0c dataset configuration (data/synthea_config/synthea.properties)
# has been approved and committed.
#
# Reference commands (do not uncomment without reading the above caveats):
#
#   SYNTHEA_JAR="$DATA_DIR/synthea-with-dependencies.jar"
#   SYNTHEA_VERSION="3.3.0"   # pin to a known-good release
#
#   # Download the fat JAR from GitHub Releases:
#   # curl -L -o "$SYNTHEA_JAR" \
#   #   "https://github.com/synthetichealth/synthea/releases/download/v${SYNTHEA_VERSION}/synthea-with-dependencies.jar"
#
#   # Run Synthea with US Core IG enabled to emit clinical notes:
#   # java -jar "$SYNTHEA_JAR" \
#   #   -c "$DATA_DIR/synthea_config/synthea.properties" \
#   #   --exporter.fhir.export=true \
#   #   --exporter.fhir.use_us_core_ig=true \
#   #   --exporter.fhir.us_core_version=6.1.0 \
#   #   -p 1000 \
#   #   Massachusetts
#
#   # synthea.properties should also set:
#   #   exporter.fhir.use_us_core_ig=true
#   #   exporter.fhir.export=true
#   #   (refer to data/synthea_config/synthea.properties — pending Phase-0c approval)
# ---------------------------------------------------------------------------

echo "  -- B2: Local Synthea Generation --"

# Detect Java major version
JAVA_MAJOR=0
if command -v java &>/dev/null; then
    # Extract major version from "java version "1.8.0_xxx"" or "openjdk version "17.0.x""
    _java_ver_str="$(java -version 2>&1 | awk -F '"' '/version/ {print $2}')"
    _major="${_java_ver_str%%.*}"
    # Java 8 reports "1.8.x" — strip leading "1." if present
    if [[ "$_major" == "1" ]]; then
        _second="${_java_ver_str#*.}"
        _major="${_second%%.*}"
    fi
    JAVA_MAJOR="$_major"
fi

if [[ "$JAVA_MAJOR" -lt 17 ]]; then
    SYNTHEA_LOCAL_BLOCKED=1
    echo ""
    echo "  [BLOCKED] Local Synthea generation requires Java 17+."
    echo "  Detected Java major version: ${JAVA_MAJOR} (Java 8 on this machine)."
    echo "  To unblock, install Java 17+, e.g.:"
    echo "    sudo apt install openjdk-17-jdk"
    echo "  Then re-run this script (B2 reference commands are in the script comments)."
    echo ""
else
    echo "  Java $JAVA_MAJOR detected — Java 17+ requirement satisfied."
    echo "  To run local Synthea generation, see the commented-out B2 reference"
    echo "  commands inside this script."
    echo ""
fi


# ===========================================================================
# SUMMARY
# ===========================================================================
echo "==================================================="
echo " fetch_data.sh — Summary"
echo "==================================================="

if [[ "$CMS_FETCHED" -eq 1 ]]; then
    echo " [DONE] Section A : CMS-HCC V28 files fetched to $DATA_DIR/cms_hcc_v28/"
else
    echo " [SKIP] Section A : CMS_V28_ZIP_URL not set — manual browser download required."
fi

if [[ "$COHERENT_FETCHED" -eq 1 ]]; then
    echo " [DONE] Section B1: Coherent dataset synced to $DATA_DIR/synthea/coherent/"
else
    echo " [SKIP] Section B1: Set FETCH_COHERENT=1 to download Coherent (~9 GB)."
fi

if [[ "$SYNTHEA_LOCAL_BLOCKED" -eq 1 ]]; then
    echo " [BLKD] Section B2: Local Synthea generation blocked (Java 8; needs Java 17+)."
else
    echo " [INFO] Section B2: See commented-out reference commands in this script."
fi

echo ""
echo " Data provenance and directory layout: $DATA_DIR/README.md"
echo "==================================================="
