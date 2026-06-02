"""Controlled error injection produces matched, known-truth cases."""
import tempfile
import unittest
from pathlib import Path

from _bootstrap import *  # noqa: F401,F403  (sys.path bootstrap)

from hcceval.inject import DEFAULT_SPECS, build_injection_set, write_injection_set  # noqa: E402
from hcceval.notes import note_from_bundle  # noqa: E402


class TestInjection(unittest.TestCase):
    def test_matched_supported_unsupported_pairs(self):
        inj = build_injection_set()
        # one supported + one unsupported per family, sharing the same coded HCC.
        self.assertEqual(len(inj.gold), 2 * len(DEFAULT_SPECS))
        self.assertEqual(sum(1 for g in inj.gold if g.supported), len(DEFAULT_SPECS))
        self.assertEqual(sum(1 for g in inj.gold if not g.supported), len(DEFAULT_SPECS))
        # every gold label is sourced "injected" and has a coded candidate.
        self.assertTrue(all(g.source == "injected" for g in inj.gold))
        cand_keys = {c.key for c in inj.candidates}
        self.assertEqual({g.key for g in inj.gold}, cand_keys)

    def test_holdout_slice_present(self):
        inj = build_injection_set(holdout_every=2)
        self.assertTrue(any(g.holdout for g in inj.gold))
        self.assertTrue(any(not g.holdout for g in inj.gold))
        # holdout=0 disables the reserved slice entirely.
        self.assertFalse(any(g.holdout for g in build_injection_set(holdout_every=0).gold))

    def test_bundles_roundtrip_through_note_reader(self):
        inj = build_injection_set()
        with tempfile.TemporaryDirectory() as d:
            paths = write_injection_set(inj, d)
            self.assertEqual(len(paths), len(inj.bundles))
            # the supported diabetes note must carry M.E.A.T.; the unsupported one must not.
            sup = note_from_bundle(Path(d) / "inj-diabetes-supported.json")
            uns = note_from_bundle(Path(d) / "inj-diabetes-unsupported.json")
            self.assertIn("Monitor", sup)
            self.assertIn("HbA1c", sup)
            self.assertNotIn("diabetes", uns.lower())


if __name__ == "__main__":
    unittest.main()
