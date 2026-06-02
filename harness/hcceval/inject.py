"""Controlled error injection: known-truth substantiation cases.

The eval must measure whether the auditor catches *RADV-style* unsupported codes
— a diagnosis is coded (so the engine triggers its HCC) but the note does not
document it to CMS M.E.A.T. standards — rather than merely echoing the coded set.
We cannot read truth off real notes blindly, so we *construct* it: for each target
family we emit two matched cases sharing the same coded HCC,

  * a SUPPORTED case whose note carries full M.E.A.T. (Monitor / Evaluate /
    Assess / Treat) evidence for the condition, and
  * an UNSUPPORTED case where the code is present but the note is silent on it
    (documentation weak/absent) — a textbook over-code.

Each case is written as a minimal Synthea-style FHIR bundle (so the real agent can
audit it post-merge and the harness can reconstruct the note for span checks),
together with a :class:`~hcceval.schema.Candidate` (the engine-eligible HCC) and a
:class:`~hcceval.schema.GoldLabel` whose truth is known by construction. A
configurable slice is flagged ``holdout`` — reserved truth that must never inform
prompt/threshold tuning.
"""
from __future__ import annotations

import base64
import json
from dataclasses import dataclass
from pathlib import Path

from .schema import Candidate, GoldLabel


@dataclass(frozen=True)
class InjectionSpec:
    """One target-family condition with a documented and an undocumented note."""
    family: str
    hcc: int
    icd10: str
    snomed: str
    condition_display: str
    supported_note: str   # full M.E.A.T. — gold "supported"
    unsupported_note: str  # code present, documentation absent — gold "unsupported"


# A small, M.E.A.T.-anchored library across the four target families. Each
# supported_note explicitly Monitors / Evaluates / Assesses / Treats the
# condition; each unsupported_note documents an unrelated visit (the code is
# carried in the problem list / claims but never substantiated in the note).
DEFAULT_SPECS: tuple[InjectionSpec, ...] = (
    InjectionSpec(
        family="diabetes",
        hcc=37,
        icd10="E1122",
        snomed="127013003",
        condition_display="Type 2 diabetes mellitus with diabetic chronic kidney disease",
        supported_note=(
            "# Assessment and Plan\n"
            "1. Type 2 diabetes mellitus with diabetic chronic kidney disease.\n"
            "   - Monitor: HbA1c 8.1% today, up from 7.6%. Reviewing home glucose logs.\n"
            "   - Evaluate: microalbuminuria stable; eGFR 52.\n"
            "   - Assess: suboptimal glycemic control with established diabetic CKD.\n"
            "   - Treat: continue metformin, increase insulin glargine to 24 units nightly; "
            "referral to nephrology.\n"
        ),
        unsupported_note=(
            "# Chief Complaint\n- Ankle sprain.\n\n"
            "# History of Present Illness\nPatient twisted the right ankle stepping off a curb.\n\n"
            "# Assessment and Plan\n1. Right ankle sprain. RICE, ibuprofen PRN, follow up if no "
            "improvement in two weeks.\n"
        ),
    ),
    InjectionSpec(
        family="ckd",
        hcc=327,
        icd10="N184",
        snomed="431857002",
        condition_display="Chronic kidney disease stage 4 (severe)",
        supported_note=(
            "# Assessment and Plan\n"
            "1. Chronic kidney disease, stage 4.\n"
            "   - Monitor: eGFR 22, creatinine 2.9 (prior 2.6). Potassium 5.1.\n"
            "   - Evaluate: renal ultrasound reviewed; no obstruction.\n"
            "   - Assess: progressive stage 4 CKD, nearing transplant evaluation.\n"
            "   - Treat: started on sodium bicarbonate; dietary protein restriction counseled; "
            "nephrology follow-up in 4 weeks.\n"
        ),
        unsupported_note=(
            "# Chief Complaint\n- Annual flu vaccination.\n\n"
            "# Assessment and Plan\n1. Influenza immunization administered. No acute concerns "
            "today. Patient feels well.\n"
        ),
    ),
    InjectionSpec(
        family="heart_failure",
        hcc=226,
        icd10="I509",
        snomed="88805009",
        condition_display="Chronic congestive heart failure",
        supported_note=(
            "# Assessment and Plan\n"
            "1. Chronic congestive heart failure (HFrEF, EF 35%).\n"
            "   - Monitor: daily weights stable, no orthopnea; BNP 540.\n"
            "   - Evaluate: echocardiogram reviewed, EF unchanged.\n"
            "   - Assess: compensated chronic systolic heart failure.\n"
            "   - Treat: continue carvedilol and furosemide; up-titrate lisinopril; "
            "low-sodium diet reinforced.\n"
        ),
        unsupported_note=(
            "# Chief Complaint\n- Seasonal allergies.\n\n"
            "# Assessment and Plan\n1. Allergic rhinitis. Start loratadine daily; nasal saline "
            "rinses. No cardiopulmonary complaints.\n"
        ),
    ),
    InjectionSpec(
        family="copd",
        hcc=280,
        icd10="J439",
        snomed="87433001",
        condition_display="Pulmonary emphysema",
        supported_note=(
            "# Assessment and Plan\n"
            "1. COPD / pulmonary emphysema.\n"
            "   - Monitor: dyspnea on exertion stable; SpO2 93% on room air.\n"
            "   - Evaluate: spirometry FEV1 48% predicted; reviewed today.\n"
            "   - Assess: moderate-to-severe COPD, emphysema-predominant.\n"
            "   - Treat: continue tiotropium and albuterol; pulmonary rehab referral; "
            "smoking-cessation counseling provided.\n"
        ),
        unsupported_note=(
            "# Chief Complaint\n- Wrist laceration.\n\n"
            "# Assessment and Plan\n1. Superficial laceration, left wrist. Cleaned, two sutures "
            "placed, tetanus up to date. Return for suture removal in 10 days.\n"
        ),
    ),
)

# Synthea-style demographics for the synthetic patients (kept old enough that
# the engine treats them as community aged continuing-enrollees).
_DOB = "1950-01-01"
_SEX = "male"


def _fhir_bundle(patient_id: str, snomed: str, display: str, note: str) -> dict:
    """Minimal Synthea-shaped bundle the harness reader + agent both understand."""
    return {
        "resourceType": "Bundle",
        "type": "collection",
        "entry": [
            {
                "resource": {
                    "resourceType": "Patient",
                    "id": patient_id,
                    "birthDate": _DOB,
                    "gender": _SEX,
                    "name": [{"given": ["Synthetic"], "family": patient_id}],
                }
            },
            {
                "resource": {
                    "resourceType": "Condition",
                    "code": {
                        "coding": [
                            {
                                "system": "http://snomed.info/sct",
                                "code": snomed,
                                "display": display,
                            }
                        ]
                    },
                }
            },
            {
                "resource": {
                    "resourceType": "DocumentReference",
                    "date": "2026-01-15T09:00:00-05:00",
                    "content": [
                        {
                            "attachment": {
                                "contentType": "text/plain; charset=utf-8",
                                "data": base64.b64encode(note.encode("utf-8")).decode("ascii"),
                            }
                        }
                    ],
                }
            },
        ],
    }


@dataclass
class InjectionSet:
    candidates: list[Candidate]
    gold: list[GoldLabel]
    bundles: dict[str, dict]  # patient_id -> FHIR bundle


def build_injection_set(
    specs: tuple[InjectionSpec, ...] = DEFAULT_SPECS,
    holdout_every: int = 2,
) -> InjectionSet:
    """Construct the matched supported/unsupported cases for every spec.

    ``holdout_every`` flags every Nth emitted gold label as held-out (default:
    every 2nd), reserving a never-for-tuning slice. Patient IDs are deterministic
    (``inj-<family>-<status>``) so re-runs are reproducible.
    """
    candidates: list[Candidate] = []
    gold: list[GoldLabel] = []
    bundles: dict[str, dict] = {}

    idx = 0
    for spec in specs:
        for status in ("supported", "unsupported"):
            pid = f"inj-{spec.family}-{status}"
            note = spec.supported_note if status == "supported" else spec.unsupported_note
            bundles[pid] = _fhir_bundle(pid, spec.snomed, spec.condition_display, note)
            # The code is coded in BOTH cases (engine triggers the HCC either way);
            # only the documentation — hence the gold truth — differs.
            candidates.append(
                Candidate(
                    patient_id=pid,
                    hcc=spec.hcc,
                    triggering_icd10=[spec.icd10],
                    triggering_snomed=[spec.snomed],
                )
            )
            gold.append(
                GoldLabel(
                    patient_id=pid,
                    hcc=spec.hcc,
                    gold_status=status,
                    source="injected",
                    rationale=(
                        f"Constructed {status} case for {spec.family} HCC {spec.hcc}: "
                        + (
                            "note documents full M.E.A.T. for the coded condition."
                            if status == "supported"
                            else "code present but note documents an unrelated visit "
                            "with no M.E.A.T. for the coded condition (over-code)."
                        )
                    ),
                    holdout=(holdout_every > 0 and idx % holdout_every == 1),
                )
            )
            idx += 1

    return InjectionSet(candidates, gold, bundles)


def write_injection_set(inj: InjectionSet, fhir_dir: str | Path) -> list[Path]:
    """Write each synthetic bundle to ``fhir_dir`` as ``<patient_id>.json``."""
    out_dir = Path(fhir_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for pid, bundle in inj.bundles.items():
        p = out_dir / f"{pid}.json"
        p.write_text(json.dumps(bundle, indent=2), encoding="utf-8")
        written.append(p)
    return written
