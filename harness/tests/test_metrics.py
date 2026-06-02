"""Known audit + known gold → known metric values (the 3c checkpoint).

The fixtures are hand-constructed so every metric has a value derivable by hand;
these assertions pin those values. See the per-metric definitions in
``hcceval.metrics``.
"""
import math
import unittest

from _bootstrap import FIXTURES  # noqa: E402

from hcceval import schema  # noqa: E402
from hcceval.crosswalk import Crosswalk  # noqa: E402
from hcceval.metrics import EvalInputs, compute_all  # noqa: E402
from hcceval.notes import load_notes_dir  # noqa: E402


def _load(include_holdout: bool) -> dict:
    return compute_all(
        EvalInputs(
            audits=schema.load_audits(FIXTURES / "audit_results.jsonl"),
            gold=schema.load_gold(FIXTURES / "gold_labels.jsonl"),
            candidates=schema.load_candidates(FIXTURES / "candidates.jsonl"),
            notes=load_notes_dir(FIXTURES / "notes"),
            xwalk=Crosswalk.load(),
            include_holdout=include_holdout,
        )
    )


def close(a, b):
    return a is not None and math.isclose(a, b, rel_tol=0, abs_tol=1e-6)


class TestMetricsWithHoldout(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = _load(include_holdout=True)

    def test_substantiation_confusion(self):
        s = self.m["substantiation"]
        self.assertEqual(s["confusion"], {"tp": 2, "fp": 1, "fn": 1, "tn": 2})
        self.assertEqual(s["scored_pairs"], 6)
        self.assertEqual(s["gold_pairs_unreviewed"], 0)

    def test_substantiation_rates(self):
        s = self.m["substantiation"]
        self.assertTrue(close(s["accuracy"], 4 / 6))
        self.assertTrue(close(s["flag_precision"], 2 / 3))
        self.assertTrue(close(s["flag_recall"], 2 / 3))
        self.assertTrue(close(s["flag_f1"], 2 / 3))
        self.assertTrue(close(s["over_coding_rate"], 1 / 6))      # FN / all
        self.assertTrue(close(s["under_flagging_rate"], 1 / 3))   # FN / gold-unsupported
        self.assertTrue(close(s["over_flagging_rate"], 1 / 3))    # FP / gold-supported

    def test_system_oracle_loop(self):
        sy = self.m["system"]
        self.assertTrue(close(sy["agreement_with_gold"], 4 / 6))
        self.assertEqual(sy["changed_pairs"], 3)
        self.assertTrue(close(sy["disagreement_resolution_rate"], 2 / 3))
        self.assertEqual(sy["oracle_improved"], 2)
        self.assertEqual(sy["oracle_regressed"], 1)

    def test_extraction(self):
        e = self.m["extraction"]
        self.assertEqual((e["tp"], e["fp"], e["fn"]), (3, 1, 3))
        self.assertEqual(e["unmapped_extractions"], 1)
        self.assertTrue(close(e["precision"], 0.75))
        self.assertTrue(close(e["recall"], 0.5))
        self.assertTrue(close(e["f1"], 0.6))

    def test_spans_and_hallucination(self):
        sp = self.m["spans"]
        self.assertEqual(sp["spans_checked"], 5)
        self.assertTrue(close(sp["span_accuracy"], 0.8))
        self.assertEqual(sp["citations_checked"], 3)
        self.assertTrue(close(sp["citation_grounding"], 2 / 3))
        self.assertEqual(sp["quotes_checked"], 8)
        self.assertTrue(close(sp["hallucination_rate"], 0.25))
        self.assertEqual(sp["patients_without_notes"], 0)

    def test_calibration_ece(self):
        cal = self.m["calibration"]
        self.assertEqual(cal["n"], 6)
        self.assertTrue(close(cal["ece"], 0.3))


class TestMetricsExcludeHoldout(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = _load(include_holdout=False)

    def test_holdout_removed(self):
        s = self.m["substantiation"]
        self.assertEqual(s["confusion"], {"tp": 1, "fp": 1, "fn": 1, "tn": 1})
        self.assertEqual(s["scored_pairs"], 4)
        self.assertTrue(close(s["accuracy"], 0.5))
        self.assertTrue(close(s["over_coding_rate"], 0.25))


class TestUnreviewedCoverage(unittest.TestCase):
    def test_gold_without_audit_is_reported_not_scored(self):
        audits = schema.load_audits(FIXTURES / "audit_results.jsonl")
        gold = schema.load_gold(FIXTURES / "gold_labels.jsonl")
        gold.append(schema.GoldLabel("ghost", 999, "unsupported", "manual", "no audit"))
        m = compute_all(EvalInputs(audits=audits, gold=gold))
        # the extra gold pair is counted as unreviewed, not folded into the confusion.
        self.assertEqual(m["substantiation"]["gold_pairs_unreviewed"], 1)
        self.assertEqual(m["substantiation"]["scored_pairs"], 6)


if __name__ == "__main__":
    unittest.main()
