//! Public input/output contract for the engine.
//!
//! Input mirrors CMS's PERSON + DIAG shape; output is a fully decomposed risk
//! score with provenance (which diagnosis drove which HCC, what was trumped),
//! which the downstream substantiation agent relies on.

use crate::demographics::Date;

/// Beneficiary sex (CMS codes: 1 = Male, 2 = Female).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sex {
    Male,
    Female,
}

/// Medicaid dual-eligibility status, which selects the community coefficient segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualStatus {
    NonDual,
    PartialDual,
    FullDual,
}

/// CMS-HCC community model segment (v1 scope: the six community continuing-enrollee
/// segments; institutional and new-enrollee models are future work).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    CommunityNonDualAged,
    CommunityPartialDualAged,
    CommunityFullDualAged,
    CommunityNonDualDisabled,
    CommunityPartialDualDisabled,
    CommunityFullDualDisabled,
}

impl Segment {
    /// Derive the segment from age (aged vs. disabled) and dual status.
    pub fn derive(age: i32, dual: DualStatus) -> Segment {
        let aged = age >= 65;
        use DualStatus::*;
        use Segment::*;
        match (aged, dual) {
            (true, NonDual) => CommunityNonDualAged,
            (true, PartialDual) => CommunityPartialDualAged,
            (true, FullDual) => CommunityFullDualAged,
            (false, NonDual) => CommunityNonDualDisabled,
            (false, PartialDual) => CommunityPartialDualDisabled,
            (false, FullDual) => CommunityFullDualDisabled,
        }
    }

    /// The coefficient column name in `V28_CE_Relative_Factors.csv`.
    pub fn column(&self) -> &'static str {
        use Segment::*;
        match self {
            CommunityNonDualAged => "COMMUNITY_NA",
            CommunityPartialDualAged => "COMMUNITY_PBA",
            CommunityFullDualAged => "COMMUNITY_FBA",
            CommunityNonDualDisabled => "COMMUNITY_ND",
            CommunityPartialDualDisabled => "COMMUNITY_PBD",
            CommunityFullDualDisabled => "COMMUNITY_FBD",
        }
    }
}

/// A single ICD-10-CM diagnosis for a beneficiary.
#[derive(Debug, Clone)]
pub struct Diagnosis {
    /// ICD-10-CM code; normalized to uppercase, no dot (e.g. `E1165`).
    pub icd10: String,
    /// Optional source identifier (encounter/claim/note-span id) for provenance.
    pub source_id: Option<String>,
}

impl Diagnosis {
    pub fn new(icd10: impl Into<String>) -> Self {
        Diagnosis { icd10: normalize_icd10(&icd10.into()), source_id: None }
    }
}

/// Normalize an ICD-10 code to the form used in the CMS mapping file: uppercase,
/// no dot, no surrounding whitespace.
pub fn normalize_icd10(code: &str) -> String {
    code.trim().to_uppercase().replace('.', "")
}

/// Beneficiary input — mirrors the CMS `beneficiaries.csv` PERSON record.
#[derive(Debug, Clone)]
pub struct PersonInput {
    pub person_id: String,
    pub date_of_birth: Date,
    pub sex: Sex,
    /// Original Reason for Entitlement Code (0 = aged, 1 = disability, 2/3 = ESRD).
    pub orec: u8,
    /// Long-term institutional Medicaid flag (`LTIMCAID`).
    pub long_term_medicaid: bool,
    /// Medicaid dual-eligibility status, selecting the community segment.
    pub dual_status: DualStatus,
    pub diagnoses: Vec<Diagnosis>,
}

/// One HCC retained in the final risk profile (after hierarchy), with provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct HccAssignment {
    pub hcc: u32,
    pub coefficient: f64,
    /// ICD-10 codes that mapped to this HCC (sorted, deduped).
    pub triggering_diagnoses: Vec<String>,
    /// Less-severe HCCs this one suppressed via the V28 hierarchy.
    pub trumped: Vec<u32>,
}

/// A named model factor contributing to the score (demographic or interaction).
#[derive(Debug, Clone, PartialEq)]
pub struct Factor {
    pub variable: String,
    pub coefficient: f64,
}

/// One step of the diagnosis → CC → HCC trail, including hierarchy outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    pub icd10: String,
    pub cc: u32,
    pub hcc: u32,
    /// Whether the HCC survived the hierarchy (`false` = trumped by a more severe HCC).
    pub kept: bool,
    /// If trumped, the HCC that suppressed it.
    pub trumped_by: Option<u32>,
}

/// Full, decomposed scoring result for one beneficiary.
#[derive(Debug, Clone)]
pub struct ScoreResult {
    pub person_id: String,
    pub model: &'static str,
    pub segment: Segment,
    pub age: i32,
    /// The age/sex demographic factor (e.g. `F70_74`).
    pub demographic: Factor,
    /// Additional demographic factors that apply (e.g. `OriginallyDisabled_Female`).
    pub extra_demographic: Vec<Factor>,
    /// HCCs retained after hierarchy, sorted ascending by HCC number.
    pub hccs: Vec<HccAssignment>,
    /// The HCC-count factor (`D1`..`D10P`) and its coefficient.
    pub hcc_count: Factor,
    /// Disease-interaction factors that fired (e.g. `DIABETES_HF_V28`).
    pub interactions: Vec<Factor>,
    /// Total risk score, rounded to 3 decimals (CMS convention).
    pub raw_score: f64,
    /// Diagnosis → CC → HCC provenance trail (including trumped entries).
    pub provenance: Vec<Provenance>,
}
