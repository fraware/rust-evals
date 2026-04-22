//! # eval-ladder-lean
//!
//! L4 semantic validator. Plugs a Lean-checker invocation into the
//! runner's [`eval_ladder_runner::LevelExtension`] seam so tasks in the
//! curated proof-carrying subset can be validated against a
//! machine-checkable obligation.
//!
//! # Scope boundary
//!
//! L4 is intentionally narrow. It does **not** translate Rust code to
//! Lean, parse arbitrary diffs, or attempt whole-repository
//! verification. Each obligation is a curated entry in
//! `datasets/derived/proof_subset/manifest.jsonl` that references a
//! pre-landed Lean declaration under
//! `packages/lean/EvalLadder/Obligations/`. The checker runs the
//! exact command declared by that entry and reads back a small,
//! versioned JSON verdict.
//!
//! # Determinism
//!
//! The extension preserves the Milestone C bundle-hash invariant: with
//! a [`eval_ladder_runner::FixedClock`] and a deterministic checker
//! implementation (for example the [`ScriptedChecker`] used by
//! acceptance tests) reruns produce byte-identical trace files and
//! bundle hashes.
//!
//! # Layers
//!
//! 1. [`spec`] - typed mirror of the `ProofObligation` JSON schema.
//! 2. [`manifest`] - JSONL loader keyed by `TaskId`.
//! 3. [`checker`] - the [`LeanChecker`] trait and its default
//!    [`ExternalProcessChecker`].
//! 4. [`scripted`] - a deterministic test-double checker.
//! 5. [`report`] - the `proof_results.json` bundle artifact.
//! 6. [`extension`] - the [`L4Extension`] implementation.
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod checker;
pub mod extension;
pub mod manifest;
pub mod report;
pub mod scripted;
pub mod spec;

pub use checker::{
    ExternalProcessChecker, LeanCheckContext, LeanCheckError, LeanCheckOutcome, LeanChecker,
    LeanStatus,
};
pub use extension::{L4Extension, L4_EXTENSION_NAME, L4_RESULT_FILE};
pub use manifest::{ObligationManifest, ObligationManifestError};
pub use report::{ProofReport, PROOF_REPORT_SCHEMA_VERSION};
pub use scripted::ScriptedChecker;
pub use spec::{
    Difficulty, ObligationProofChecker, ProofObligation, ProofObligationLoadError, PropertyType,
    SelectionRationale, WitnessInput,
};
