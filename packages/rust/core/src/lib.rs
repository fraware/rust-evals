//! # eval-ladder-core
//!
//! Core domain types, identifiers, versioned contracts, and canonical JSON
//! hashing primitives for the `eval-ladder` evaluation system.
//!
//! This crate is deliberately dependency-light. It has no knowledge of
//! containers, file systems, or networks. Every other `eval-ladder` crate
//! depends on this one and on a subset of the domain crates; `core` never
//! depends on any of them.
//!
//! The authoritative specification of every persisted artifact lives in
//! `schemas/` at the workspace root. The Rust types in this crate are the
//! runtime embodiment of those schemas.
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod candidate;
pub mod codes;
pub mod error;
pub mod hash;
pub mod ids;
pub mod level;
pub mod result;
pub mod task;
pub mod version;

pub use candidate::{
    CandidateResolution, ContextMode, GenerationMetadata, GenerationMode, PatchFormat,
};
pub use codes::{FailureReason, PolicyViolation, TaxonomyCode};
pub use error::CoreError;
pub use hash::{canonical_json, digest, Sha256Digest};
pub use ids::{BenchmarkId, BundleId, CandidateId, ObligationId, RunId, TaskId};
pub use level::{EvaluationLevel, EvaluationStatus};
pub use result::{ArtifactKind, EvaluationArtifact, EvaluationResult};
pub use task::{BenchmarkLanguage, BenchmarkTask};
pub use version::{EvaluatorVersion, SchemaVersion, EVALUATOR_VERSION, SCHEMA_VERSION};
