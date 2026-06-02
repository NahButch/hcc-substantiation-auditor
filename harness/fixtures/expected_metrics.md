# HCC Substantiation Auditor — Evaluation Metrics

- Patients audited: **6**, audited HCCs: **6**
- Gold labels: **6** (injected 3, holdout 2)

## Substantiation

| metric | value | definition |
|---|---|---|
| accuracy | 66.7% | agreement with gold (binary) |
| flag precision | 66.7% | TP/(TP+FP) |
| flag recall | 66.7% | TP/(TP+FN) |
| flag F1 | 0.667 | harmonic mean |
| **over-coding rate** | **16.7%** | FN/all — over-codes wrongly cleared (expensive RADV error) |
| under-flagging rate | 33.3% | FN/gold-unsupported = 1−recall |
| over-flagging rate | 33.3% | FP/gold-supported — false alarms |

Confusion: TP=2, FP=1, FN=1, TN=2 (scored pairs 6).

## System

- Agreement with gold: **66.7%**
- Disagreement-resolution: **66.7%** (3 changed; improved 2, regressed 1)

## Extraction (HCC-level)

- Precision **75.0%**, recall **50.0%**, F1 **0.600** (TP/FP/FN = 3/1/3)

## Spans & hallucination

- Span accuracy **80.0%** (5)
- Citation grounding **66.7%** (3)
- **Hallucination rate 25.0%** (8 quotes; target 0)

## Calibration

- n=6, ECE=0.300
