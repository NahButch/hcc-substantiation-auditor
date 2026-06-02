# Manual annotation rule (cohort gold)

Single independent reviewer pass (LLM-assisted; NOT a credentialed RADV coder).
A coded HCC is gold **supported** iff the note's Assessment & Plan documents
condition-specific M.E.A.T. for that family in the encounter; a condition that
appears only in the past-history/problem list (A&P addresses an unrelated reason)
is gold **unsupported**.

Supported cohort cases (by family), from reading all 30 A&Ps:
- 2a66bf9d… : COPD  — A&P: spirometry + pulmonary rehabilitation
- b4549ab9… : HF    — A&P: echocardiography + heart-failure tracking panel + furosemide
- 92c87506… : diabetes — A&P: diabetic retinal eye exam (complication monitoring)
- 00c4181b… : diabetes — A&P: diabetic retinal eye exam
- 946de649… : diabetes — A&P: diabetic retinal eye exam
All other cohort (patient, HCC) pairs: unsupported (history-list only).
