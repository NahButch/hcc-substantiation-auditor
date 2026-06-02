"""Reconstruct the clinical-note text the auditor saw, from Synthea FHIR bundles.

Span-accuracy and citation-hallucination checks must run against *exactly* the
note text the agent was given. This module mirrors the Rust reader
(``crates/agent/src/fhir.rs``): it pulls the base64 ``DocumentReference``
attachments from a patient bundle, sorts them most-recent-first, joins them with
``\\n\\n---\\n\\n``, and caps the result at :data:`NOTE_CAP` bytes.

Keep this in lockstep with ``fhir.rs``; if the Rust cap or join changes, the span
check would otherwise silently disagree with what the agent actually read.
"""
from __future__ import annotations

import base64
import json
import re
from pathlib import Path

# Must match crates/agent/src/fhir.rs::NOTE_CAP.
NOTE_CAP = 12_000

SNOMED_SYSTEM = "http://snomed.info/sct"


def note_from_bundle(path: str | Path) -> str | None:
    """Concatenated note text for one patient bundle (None if not a patient bundle)."""
    try:
        v = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    entries = v.get("entry")
    if not isinstance(entries, list):
        return None

    notes: list[tuple[str, str]] = []  # (date, text)
    is_patient = False
    for e in entries:
        r = e.get("resource") or {}
        rtype = r.get("resourceType")
        if rtype == "Patient":
            is_patient = True
        elif rtype == "DocumentReference":
            date = str(r.get("date", ""))
            for c in r.get("content", []) or []:
                data = (c.get("attachment") or {}).get("data")
                if not data:
                    continue
                try:
                    txt = base64.b64decode(data).decode("utf-8", "replace")
                except (ValueError, TypeError):
                    continue
                notes.append((date, txt))
    if not is_patient:
        return None

    # Most-recent first, joined and capped — identical to fhir.rs.
    notes.sort(key=lambda t: t[0], reverse=True)
    note = ""
    for _, txt in notes:
        if len(note.encode("utf-8")) >= NOTE_CAP:
            break
        if note:
            note += "\n\n---\n\n"
        note += txt
    return _byte_truncate(note, NOTE_CAP)


def _byte_truncate(s: str, cap: int) -> str:
    """Truncate to at most ``cap`` UTF-8 bytes without splitting a codepoint."""
    b = s.encode("utf-8")
    if len(b) <= cap:
        return s
    return b[:cap].decode("utf-8", "ignore")


def patient_id_from_bundle(path: str | Path) -> str | None:
    """The Patient resource ``id`` for a bundle (for mapping notes → patient)."""
    try:
        v = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    for e in v.get("entry", []) or []:
        r = e.get("resource") or {}
        if r.get("resourceType") == "Patient":
            return r.get("id")
    return None


def load_notes_dir(fhir_dir: str | Path) -> dict[str, str]:
    """Map patient_id → reconstructed note text for every bundle in a FHIR dir.

    Skips the non-patient bundles Synthea emits (hospital/practitioner info).
    """
    out: dict[str, str] = {}
    for p in sorted(Path(fhir_dir).glob("*.json")):
        name = p.name
        if "hospitalInformation" in name or "practitionerInformation" in name:
            continue
        pid = patient_id_from_bundle(p)
        if pid is None:
            continue
        note = note_from_bundle(p)
        if note is not None:
            out[pid] = note
    return out


# --------------------------------------------------------------------------- #
# Quote-occurrence check (used by span accuracy + hallucination rate)
# --------------------------------------------------------------------------- #
_WS = re.compile(r"\s+")


def _normalize(s: str) -> str:
    """Case-fold and collapse runs of whitespace — tolerant exact-quote matching."""
    return _WS.sub(" ", s).strip().casefold()


def quote_in_note(quote: str, note: str) -> bool:
    """Whether a claimed quote/span occurs in the note under :func:`_normalize`.

    Whitespace-insensitive, case-insensitive substring match. An empty quote is
    treated as *not* a verifiable occurrence (callers exclude empties).
    """
    q = _normalize(quote)
    if not q:
        return False
    return q in _normalize(note)
