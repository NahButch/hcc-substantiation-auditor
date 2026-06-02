# HCC Substantiation Auditor — Evaluation Metrics

- Patients audited: **8**, audited HCCs: **8**
- Gold labels: **8** (injected 8, holdout 4)

## Substantiation

| metric | value | definition |
|---|---|---|
| accuracy | 87.5% | agreement with gold (binary) |
| flag precision | 80.0% | TP/(TP+FP) |
| flag recall | 100.0% | TP/(TP+FN) |
| flag F1 | 0.889 | harmonic mean |
| **over-coding rate** | **0.0%** | FN/all — over-codes wrongly cleared (expensive RADV error) |
| under-flagging rate | 0.0% | FN/gold-unsupported = 1−recall |
| over-flagging rate | 25.0% | FP/gold-supported — false alarms |

Confusion: TP=4, FP=1, FN=0, TN=3 (scored pairs 8).

## System

- Agreement with gold: **87.5%**
- Disagreement-resolution: **0.0%** (1 changed; improved 0, regressed 1)

## Extraction (HCC-level)

- Precision **n/a**, recall **0.0%**, F1 **n/a** (TP/FP/FN = 0/0/8)

## Spans & hallucination

- Span accuracy **87.5%** (8)
- Citation grounding **0.0%** (8)
- **Hallucination rate 56.2%** (16 quotes; target 0)

## Calibration

- n=1, ECE=0.850
