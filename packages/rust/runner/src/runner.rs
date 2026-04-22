//! The [`BenchmarkRunner`] trait.

use eval_ladder_core::CoreError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::artifact::RunArtifact;
use crate::container::ContainerEngineError;
use crate::prepare::{PreparedRun, PreparedRunError};

/// Rules controlling which L2 sub-validators to run.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrengtheningRules {
    /// Selected mode name. Matches `configs/strengthening/*.toml`.
    pub mode: String,
    /// Whether to run augmented unit tests.
    #[serde(default)]
    pub run_augmented_unit_tests: bool,
    /// Whether to run differential behaviour check.
    #[serde(default)]
    pub run_differential_behavior: bool,
    /// Whether to run targeted regression.
    #[serde(default)]
    pub run_targeted_regression: bool,
    /// Whether to run property-based fuzz.
    #[serde(default)]
    pub run_property_fuzz: bool,
}

/// Errors produced by any runner implementation.
#[derive(Debug, Error)]
pub enum RunnerError {
    /// Preparation failed.
    #[error("prepare: {0}")]
    Prepare(#[from] PreparedRunError),
    /// Container engine failure.
    #[error("container: {0}")]
    Container(#[from] ContainerEngineError),
    /// Core-layer error.
    #[error("core: {0}")]
    Core(#[from] CoreError),
    /// The runner was asked to perform a step that is not yet implemented on
    /// this build (for example Docker backend without `--features docker`).
    #[error("unimplemented: {0}")]
    Unimplemented(&'static str),
    /// Generic I/O.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// The execution engine contract.
///
/// Implementors own the container lifecycle for a single run. All evaluator
/// entry points (`run_official`, `run_strengthened`) receive an already
/// prepared run and return a [`RunArtifact`].
pub trait BenchmarkRunner: Send + Sync {
    /// Prepare a containerized working tree for the task and candidate.
    ///
    /// Callers provide the task manifest and the candidate; the runner is
    /// responsible for pulling the image, materializing the repo at the
    /// base commit, and applying the patch.
    fn prepare(
        &self,
        task: &eval_ladder_core::BenchmarkTask,
        candidate: &eval_ladder_core::CandidateResolution,
    ) -> Result<PreparedRun, RunnerError>;

    /// Run the official benchmark scorer.
    fn run_official(&self, prepared: &PreparedRun) -> Result<RunArtifact, RunnerError>;

    /// Run the strengthened (L2) validator set selected by `rules`.
    fn run_strengthened(
        &self,
        prepared: &PreparedRun,
        rules: &StrengtheningRules,
    ) -> Result<RunArtifact, RunnerError>;
}
