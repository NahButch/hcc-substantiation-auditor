//! # CMS-HCC V28 deterministic scoring engine
//!
//! A faithful, dependency-free reimplementation of the CMS-HCC **V28** community
//! continuing-enrollee risk-score logic, used as a *verifiable oracle* for the
//! substantiation auditor: identical input always yields identical output, and the
//! result is fully decomposed (which diagnosis drove which HCC, what the hierarchy
//! trumped) so downstream reasoning can be checked against ground truth.
//!
//! The model version is a compile-checked, type-level dimension (see [`version`]),
//! so V28 coefficients can never be silently combined with another version's tables.
//!
//! All model data (mappings, hierarchies, coefficients, interactions) is loaded
//! verbatim from the CMS-published V28 tables — none of it is hand-authored here.
//!
//! ## Example
//! ```no_run
//! use engine::{Engine, PersonInput, Diagnosis, Sex, DualStatus};
//! use engine::demographics::Date;
//!
//! let engine = Engine::load_v28("path/to/cms/v28/internal").unwrap();
//! let person = PersonInput {
//!     person_id: "1".into(),
//!     date_of_birth: Date::new(1950, 1, 1),
//!     sex: Sex::Female,
//!     orec: 0,
//!     long_term_medicaid: false,
//!     dual_status: DualStatus::NonDual,
//!     diagnoses: vec![Diagnosis::new("E11.65")],
//! };
//! let result = engine.score(&person);
//! println!("score = {}", result.raw_score);
//! ```

pub mod csv;
pub mod demographics;
pub mod engine;
pub mod tables;
pub mod types;
pub mod version;

pub use engine::Engine;
pub use tables::RiskTables;
pub use types::{
    Diagnosis, DualStatus, Factor, HccAssignment, PersonInput, Provenance, ScoreResult, Segment,
    Sex,
};
pub use version::{ModelVersion, V28};
