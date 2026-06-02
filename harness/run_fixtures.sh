#!/usr/bin/env bash
# One-command end-to-end run of the eval harness on the committed fixtures.
# Runs the unit tests, then computes + prints the full metrics table from the
# fixture audit + gold + candidates + notes. Offline; synthetic data only.
#
#   harness/run_fixtures.sh
#
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY="${PYTHON:-$HERE/../.venv/bin/python}"

echo "### unit tests #############################################"
"$PY" -m unittest discover -s "$HERE/tests" -t "$HERE/tests"

echo
echo "### metrics on fixtures ####################################"
"$PY" "$HERE/eval.py" \
  --audit "$HERE/fixtures/audit_results.jsonl" \
  --gold "$HERE/fixtures/gold_labels.jsonl" \
  --candidates "$HERE/fixtures/candidates.jsonl" \
  --fhir "$HERE/fixtures/notes"
