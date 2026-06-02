//! # HCC Substantiation Auditor — agent
//!
//! The LLM reasoning layer: extracts documented conditions from clinical notes,
//! wires them to the deterministic [`engine`] oracle, and judges whether
//! documentation substantiates each engine-triggered HCC (Phase 2 single pass;
//! Phase 3 adds the verify→self-correct loop). The LLM sits behind the swappable
//! [`llm::LlmProvider`] interface.

pub mod audit;
pub mod crosswalk;
pub mod extraction;
pub mod fhir;
pub mod groundtruth;
pub mod json;
pub mod llm;
pub mod substantiation;
