# Integration metrics — scaled run (N=36 patients, 47 gold pairs)

```
================================================================
HCC Substantiation Auditor — Evaluation Metrics (Rust)
================================================================
patients audited: 36   audited HCCs: 47
gold labels: 47 (injected 8, holdout 4)   candidates: 50   notes: 38

-- Substantiation (positive class = unsupported/flagged) -------
  scored pairs:            47
  confusion:               TP=37 FP=5 FN=1 TN=4
  accuracy (agreement):    87.2%
  flag precision:          88.1%
  flag recall:             97.4%
  flag F1:                 0.925
  OVER-CODING rate:        2.1%   (FN/all — expensive RADV error)
  under-flagging rate:     2.6%   (FN/gold-unsupported = 1-recall)
  over-flagging rate:      55.6%   (FP/gold-supported)

-- System -----------------------------------------------------
  agreement with gold:     87.2%
  changed by oracle loop:  4
  disagreement-resolution: 25.0%   (improved 1, regressed 3)

-- Extraction (HCC-level vs. candidate set) --------------------
  precision:               72.7%
  recall:                  17.0%
  F1:                      0.276
  TP/FP/FN:                8/3/39   (unmapped extractions: 174)

-- Spans, citations & hallucination ----------------------------
  span accuracy:           59.7%   (186 spans)
  citation validity:       100.0%   (47 citations, vs authority list)
  HALLUCINATION rate:      40.3%   (186 evidence quotes; target 0)

-- Calibration (n=14, ECE=0.193) ----------------------------
  [0.8,0.9]   n=10   mean_conf=0.800  acc=0.900
  [0.9,1.0]   n=4    mean_conf=0.925  acc=0.500
================================================================
```
