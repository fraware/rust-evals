//! # eval-ladder-runner
//!
//! The execution engine for `eval-ladder`.
//!
//! This crate hosts the full Milestone C pipeline: given a
//! [`eval_ladder_core::BenchmarkTask`], a
//! [`eval_ladder_core::CandidateResolution`], a container engine, a
//! scorer, and a clock, [`EvaluationPipeline::run`] produces two
//! `EvaluationResult`s (L0 official and L1 trusted rerun), a hash-chained
//! trace, and a sealed evidence bundle.
//!
//! The main contracts are:
//!
//! - [`BenchmarkRunner`]: a lower-level trait retained for adapters that
//!   prefer per-step control (`prepare` -> `run_official` -> `run_strengthened`).
//! - [`ContainerEngine`]: abstracts over a containerized execution backend.
//!   The crate ships two implementations: [`NoopEngine`] (no-op stub for
//!   tests that do not need real execution) and [`LocalProcessEngine`]
//!   (real host subprocess, Docker-free, used to exercise the pipeline
//!   in CI).
//! - [`Clock`]: injectable wall-clock source. [`SystemClock`] for
//!   production; [`FixedClock`] for deterministic test and rerun-hash
//!   acceptance.
//! - [`Scorer`]: benchmark-agnostic pass/fail classifier. The fixture
//!   [`SimpleExitCodeScorer`] is included for smoke tests.
//!
//! The Milestone C acceptance criterion — reruns on fixture tasks are
//! deterministic and bundle hashes are stable across reruns — is proven
//! by the integration test suite and by the `bundle_hash_is_stable_across_reruns`
//! unit test.
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod artifact;
pub mod clock;
pub mod container;
pub mod extension;
pub mod identity;
pub mod patch;
pub mod pipeline;
pub mod prepare;
pub mod runner;
pub mod scorer;
pub mod workspace;

pub use artifact::{RunArtifact, RunOutcome};
pub use clock::{Clock, FixedClock, SystemClock};
pub use container::{
    ContainerEngine, ContainerEngineError, DockerCliEngine, EnvVar, ExecOutcome, ExecSpec,
    LocalProcessEngine, NoopEngine, ResourceLimits,
};
pub use extension::{ExtensionContext, ExtensionError, LevelExtension};
pub use identity::{DeterministicSeed, RunIdentity, EVAL_LADDER_NAMESPACE};
pub use patch::{apply_patch, PatchApplyError, PatchApplyOutcome};
pub use pipeline::{
    EvaluationPipeline, L1Strategy, PipelineError, PipelineInputs, PipelineOutcome, RunManifest,
};
pub use prepare::{PreparedRun, PreparedRunError};
pub use runner::{BenchmarkRunner, RunnerError, StrengtheningRules};
pub use scorer::{Scorer, ScorerVerdict, SimpleExitCodeScorer};
pub use workspace::{prepare_workspace, WorkspaceError};
