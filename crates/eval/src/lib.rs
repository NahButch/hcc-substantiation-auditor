//! # Rust evaluation harness (`eval`)
//!
//! Port of the Python `hcceval` harness. Scores the agent's `audit_results.jsonl`
//! against gold substantiation labels, the engine-eligible candidate set, and the
//! source notes. Two deliberate improvements over the Python original (see
//! `harness/results/INTEGRATION_NOTES.md`):
//!
//! 1. **Candidates come from the engine itself** (`candidates::derive`), not a
//!    re-implementation — the engine is the single source of truth.
//! 2. **Citations and evidence spans are scored separately**: evidence must be in
//!    the note (span accuracy + hallucination), citations are validated against an
//!    allowed regulatory-authority list.

pub mod candidates;
pub mod metrics;
pub mod notes;
pub mod report;
pub mod schema;
