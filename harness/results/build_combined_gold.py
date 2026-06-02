#!/usr/bin/env python3
"""Build combined gold: injected known-truth + manual cohort annotation.

Cohort labels follow data/integration/manual_label_rule.md — a single independent
reviewer pass (LLM-assisted, NOT a credentialed RADV coder): a (patient, HCC) is
supported iff the note's A&P documents condition-specific M.E.A.T. for that family.
Gold is keyed to the exact (patient, HCC) pairs the engine/audit produced.
"""
import json, sys

# Cohort patients whose A&P documents the family (else everything unsupported).
SUPPORTED_FAMILY = {
    "2a66bf9d-a7cf-c577-c208-ab7280a0a3d1": "copd",
    "b4549ab9-493e-43fd-f248-d96fec4f7739": "hf",
    "92c87506-0eee-4ad1-c2b1-828f087b6fde": "dm",
    "00c4181b-e18c-7e53-4116-b453d84f04a2": "dm",
    "946de649-8845-015f-5686-0e1bd40baf30": "dm",
}
FAMILY = {
    "dm": {35, 36, 37, 38},
    "ckd": {326, 327, 328, 329},
    "hf": {221, 222, 223, 224, 225, 226, 227},
    "copd": {276, 277, 278, 279, 280},
}

def fam_of(hcc):
    for f, s in FAMILY.items():
        if hcc in s:
            return f
    return None

audit = [json.loads(l) for l in open("data/integration/eval_audit.jsonl") if l.strip()]
injected = {(g["patient_id"], g["hcc"]): g
            for g in (json.loads(l) for l in open("data/integration/injected_gold.jsonl") if l.strip())}

gold = []
for a in audit:
    pid = a["patient_id"]
    for h in a["hccs"]:
        hcc = h["hcc"]
        if pid.startswith("inj-"):
            g = injected.get((pid, hcc))
            if g:
                gold.append(g)
            continue
        sup_fam = SUPPORTED_FAMILY.get(pid)
        supported = sup_fam is not None and fam_of(hcc) == sup_fam
        gold.append({
            "patient_id": pid, "hcc": hcc,
            "gold_status": "supported" if supported else "unsupported",
            "source": "manual",
            "rationale": ("A&P documents condition-specific M.E.A.T."
                          if supported else
                          "condition appears only in history/problem list; A&P addresses an unrelated reason"),
            "holdout": False,
        })

with open("data/integration/combined_gold.jsonl", "w") as f:
    for g in gold:
        f.write(json.dumps(g) + "\n")

sup = sum(1 for g in gold if g["gold_status"] == "supported")
print(f"wrote {len(gold)} gold labels (supported {sup}, unsupported {len(gold)-sup})")
print(f"  injected: {sum(1 for g in gold if g['source']=='injected')}, manual: {sum(1 for g in gold if g['source']=='manual')}")
