"""Mechanical candidate derivation: SNOMED→ICD10→HCC, hierarchy collapse, reconcile."""
import csv
import tempfile
import unittest
from pathlib import Path

from _bootstrap import FIXTURES  # noqa: E402

from hcceval import schema  # noqa: E402
from hcceval.candidates import derive_candidates, reconcile  # noqa: E402
from hcceval.crosswalk import Crosswalk  # noqa: E402


def _write_conditions(rows: list[tuple[str, str]]) -> str:
    """rows = [(patient_id, snomed_code)] -> path to a Synthea-shaped conditions.csv."""
    fd = tempfile.NamedTemporaryFile("w", suffix=".csv", delete=False, newline="")
    w = csv.writer(fd)
    w.writerow(["START", "STOP", "PATIENT", "ENCOUNTER", "SYSTEM", "CODE", "DESCRIPTION"])
    for pid, code in rows:
        w.writerow(["2025-01-01", "", pid, "enc", "http://snomed.info/sct", code, ""])
    fd.close()
    return fd.name


class TestCandidates(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.xwalk = Crosswalk.load()

    def test_hierarchy_collapse_and_near_miss_drop(self):
        # Patient A: diabetic-CKD (127013003 -> E1122 -> HCC37) AND plain T2DM
        # (44054006 -> E119 -> HCC38) AND prediabetes (714628002 -> R7303 -> no HCC).
        # Expect: HCC37 only — 38 is trumped by 37, prediabetes is a near-miss.
        path = _write_conditions([
            ("A", "127013003"),
            ("A", "44054006"),
            ("A", "714628002"),
            ("B", "46177005"),   # ESRD -> N186 -> HCC326
        ])
        cands = derive_candidates(path, self.xwalk)
        by = {(c.patient_id, c.hcc) for c in cands}
        self.assertEqual({c.hcc for c in cands if c.patient_id == "A"}, {37})
        self.assertIn(("B", 326), by)
        Path(path).unlink()

    def test_out_of_scope_dropped(self):
        # A SNOMED code not in the crosswalk yields no candidate.
        path = _write_conditions([("C", "999999999")])
        self.assertEqual(derive_candidates(path, self.xwalk), [])
        Path(path).unlink()

    def test_reconcile_against_engine_output(self):
        # The fixture candidates should match the fixture audit's coded HCCs exactly.
        cands = schema.load_candidates(FIXTURES / "candidates.jsonl")
        audits = schema.load_audits(FIXTURES / "audit_results.jsonl")
        r = reconcile(cands, audits)
        self.assertTrue(r.exact, msg=f"drift: {r.only_derived} / {r.only_engine}")
        self.assertEqual(r.patients_compared, 6)


if __name__ == "__main__":
    unittest.main()
