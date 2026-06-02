"""Note reconstruction mirrors fhir.rs; quote matching is whitespace/case tolerant."""
import base64
import json
import tempfile
import unittest
from pathlib import Path

from _bootstrap import FIXTURES  # noqa: E402

from hcceval.notes import (  # noqa: E402
    NOTE_CAP,
    load_notes_dir,
    note_from_bundle,
    quote_in_note,
)


def _bundle(pid: str, docs: list[tuple[str, str]]) -> dict:
    entries = [{"resource": {"resourceType": "Patient", "id": pid, "birthDate": "1950-01-01",
                             "gender": "male"}}]
    for date, text in docs:
        entries.append({"resource": {
            "resourceType": "DocumentReference", "date": date,
            "content": [{"attachment": {
                "data": base64.b64encode(text.encode()).decode("ascii")}}]}})
    return {"resourceType": "Bundle", "entry": entries}


class TestNotes(unittest.TestCase):
    def test_most_recent_first_and_join(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "x.json"
            p.write_text(json.dumps(_bundle("x", [
                ("2020-01-01", "OLDER"), ("2024-01-01", "NEWER")])))
            note = note_from_bundle(p)
            self.assertTrue(note.startswith("NEWER"))
            self.assertIn("\n\n---\n\n", note)
            self.assertLess(note.index("NEWER"), note.index("OLDER"))

    def test_byte_cap(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "x.json"
            p.write_text(json.dumps(_bundle("x", [("2024-01-01", "A" * (NOTE_CAP + 500))])))
            self.assertLessEqual(len(note_from_bundle(p).encode("utf-8")), NOTE_CAP)

    def test_non_patient_bundle_returns_none(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "hospitalInformation.json"
            p.write_text(json.dumps({"resourceType": "Bundle", "entry": [
                {"resource": {"resourceType": "Organization", "id": "o"}}]}))
            self.assertIsNone(note_from_bundle(p))

    def test_quote_matching_normalizes_whitespace_and_case(self):
        note = "Chronic   kidney\n disease stage 4. eGFR 22."
        self.assertTrue(quote_in_note("chronic kidney disease stage 4", note))
        self.assertFalse(quote_in_note("dialysis dependent", note))
        self.assertFalse(quote_in_note("", note))

    def test_load_fixture_notes_dir(self):
        notes = load_notes_dir(FIXTURES / "notes")
        self.assertEqual(set(notes), {"p1", "p2", "p3", "p4", "p5", "p6"})
        self.assertIn("HbA1c 8.1%", notes["p1"])


if __name__ == "__main__":
    unittest.main()
