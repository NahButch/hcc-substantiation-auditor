# Planning & design docs

Companion planning and design documents for the
[hcc-substantiation-auditor](../README.md) project.

## Read in this order

| File | What it is |
|------|------------|
| [`00_PROJECT_OVERVIEW.md`](00_PROJECT_OVERVIEW.md) | The thesis, the oracle pattern, scope discipline. Start here. |
| [`07_POLICY_FEEDBACK_LOOP.md`](07_POLICY_FEEDBACK_LOOP.md) | The big-picture frame: policy→solution→capture→measure→report→propose loop, AI on the edges, governance caveat. |
| [`circuit_diagram.svg`](circuit_diagram.svg) | The feedback-circuit diagram (teal = built, gray = vision, purple = AI edges). |
| [`01_ARCHITECTURE.md`](01_ARCHITECTURE.md) | System design, components, the agent loop, the oracle pattern. |
| [`02_DATA_SOURCES.md`](02_DATA_SOURCES.md) | Synthetic-data strategy (Synthea primary; why not DE-SynPUF). |
| [`06_DATASETS_AND_BENCHMARKS.md`](06_DATASETS_AND_BENCHMARKS.md) | Datasets + benchmarks, plus government sources (CMS HCC model pages, RADV rule). |
| [`03_EVALUATION_HARNESS.md`](03_EVALUATION_HARNESS.md) | Metric design — extraction vs. substantiation scored separately. |
| [`04_MILESTONES.md`](04_MILESTONES.md) | Phased build plan + as-built status. |
| [`08_BUILD_RESULTS_SUMMARY.md`](08_BUILD_RESULTS_SUMMARY.md) | The measured outcomes and how to read them (see also [`../RESULTS.md`](../RESULTS.md)). |
| [`WALKTHROUGH.md`](WALKTHROUGH.md) | One record traced end-to-end through the loop, from real output. |

## One-line status
Phases 0–3 complete. Engine agrees exactly with the CMS reference (2,100+ fuzzed
claims); agent metrics reported honestly with failure analysis. Research and educational
demonstration, synthetic data only, CMS-HCC V28.
