"""Labeling worksheet scaffolds candidates and round-trips back to gold labels."""
import csv
import tempfile
import unittest
from pathlib import Path

from _bootstrap import *  # noqa: F401,F403  (sys.path bootstrap)

from hcceval import schema  # noqa: E402
from hcceval.crosswalk import Crosswalk  # noqa: E402
from hcceval.schema import Candidate  # noqa: E402
from hcceval.worksheet import COLUMNS, build_worksheet, worksheet_to_gold  # noqa: E402


class TestWorksheet(unittest.TestCase):
    def setUp(self):
        self.xwalk = Crosswalk.load()
        self.cands = [
            Candidate("pa", 37, ["E1122"], ["127013003"]),
            Candidate("pb", 327, ["N184"], ["431857002"]),
        ]

    def test_build_then_fill_then_convert(self):
        with tempfile.TemporaryDirectory() as d:
            ws = Path(d) / "ws.csv"
            n = build_worksheet(self.cands, self.xwalk, ws)
            self.assertEqual(n, 2)

            # simulate a human filling the sheet.
            lines = [ln for ln in ws.read_text().splitlines() if not ln.startswith("#")]
            rows = list(csv.DictReader(lines))
            self.assertEqual(set(COLUMNS), set(rows[0].keys()))
            rows[0].update(gold_status="supported", meat_assess="Y", holdout="FALSE",
                           rationale="documented")
            rows[1].update(gold_status="unsupported", holdout="TRUE",
                           rationale="no documentation")
            with open(ws, "w", newline="") as f:
                w = csv.DictWriter(f, fieldnames=COLUMNS)
                w.writeheader()
                w.writerows(rows)

            gold = worksheet_to_gold(ws)
            self.assertEqual(len(gold), 2)
            g = {x.key: x for x in gold}
            self.assertEqual(g[("pa", 37)].gold_status, "supported")
            self.assertFalse(g[("pa", 37)].holdout)
            self.assertEqual(g[("pb", 327)].gold_status, "unsupported")
            self.assertTrue(g[("pb", 327)].holdout)

    def test_blank_rows_skipped(self):
        with tempfile.TemporaryDirectory() as d:
            ws = Path(d) / "ws.csv"
            build_worksheet(self.cands, self.xwalk, ws)  # nothing filled in
            self.assertEqual(worksheet_to_gold(ws), [])

    def test_hcc_description_resolved(self):
        with tempfile.TemporaryDirectory() as d:
            ws = Path(d) / "ws.csv"
            build_worksheet(self.cands, self.xwalk, ws)
            text = ws.read_text()
            self.assertIn("diabetic chronic kidney disease", text.lower())


if __name__ == "__main__":
    unittest.main()
