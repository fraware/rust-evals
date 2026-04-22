//! # eval-ladder-policy
//!
//! L3 process-validity policy engine.
//!
//! Policies are declarative TOML documents. They describe what a valid run
//! looks like: what commands may be executed, what files may be edited,
//! whether network access is allowed, whether the trace must be complete,
//! and so on.
//!
//! This crate has four layers:
//!
//! 1. [`spec`] - the TOML data model ([`Policy`]).
//! 2. [`engine`] - evaluation of a policy against a [`RunContext`],
//!    producing a list of [`PolicyFinding`]s with stable
//!    [`eval_ladder_core::PolicyViolation`] codes.
//! 3. [`diff`] - lightweight unified-diff path extractor used to turn
//!    candidate patches into `RunContext::modified_files`.
//! 4. [`context_builder`] + [`extension`] - adapter layer that turns
//!    the runner's [`eval_ladder_runner::ExtensionContext`] plus the
//!    live trace into a [`RunContext`] and plugs the policy engine
//!    into the evaluation pipeline as a
//!    [`eval_ladder_runner::LevelExtension`].
//!
//! Concrete trajectory-capture logic is provided by the runner crate;
//! this crate only judges the captured context.
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod context_builder;
pub mod diff;
pub mod engine;
pub mod extension;
pub mod report;
pub mod spec;

pub use context_builder::{build_run_context, ContextBuildError, L3Observation};
pub use diff::{any_lockfile, modified_paths, LOCKFILE_BASENAMES};
pub use engine::{evaluate, PolicyEngineError, RunContext};
pub use extension::{L3Extension, L3_EXTENSION_NAME, L3_RESULT_FILE};
pub use report::{PolicyFinding, PolicyReport};
pub use spec::{NetworkMode, Policy, PolicyLoadError};
