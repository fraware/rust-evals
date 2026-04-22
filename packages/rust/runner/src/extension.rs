//! Post-L1 level extensions (L2/L3/L4).
//!
//! The Milestone C pipeline (`EvaluationPipeline::run`) runs L0 and L1 in
//! the same crate that owns [`ContainerEngine`], [`Scorer`] and
//! [`Clock`]. Higher rungs of the ladder live in their own crates
//! (`eval-ladder-strengthening` for L2, `eval-ladder-policy` for L3,
//! `eval-ladder-lean` for L4) so that adding a new validator family
//! never requires editing the runner.
//!
//! To keep that layering strict, the runner defines a small trait,
//! [`LevelExtension`], that higher crates implement. The pipeline calls
//! each configured extension after L1 and before writing `RunFinished`
//! and sealing the bundle. Every extension sees the same
//! [`ExtensionContext`] and shares the open [`TraceWriter`] so its
//! events join the same hash chain as L0 and L1.
//!
//! # Determinism
//!
//! Extensions must be deterministic w.r.t. their declared inputs.
//! Timestamps must be sourced from [`Clock::now`] on the supplied clock,
//! never from `Utc::now()`. Any auxiliary artifacts written under
//! `ExtensionContext::bundle_dir` are folded into the bundle hash by the
//! closing [`BundleBuilder::finalize_at`] call, so non-determinism there
//! will surface as a hash divergence in the Milestone C acceptance test.

use std::path::Path;

use chrono::{DateTime, Utc};
use eval_ladder_core::{
    BenchmarkTask, CandidateId, CandidateResolution, EvaluationLevel, EvaluationResult, RunId,
    TaskId,
};
use eval_ladder_traces::{TraceWriter, TraceWriterError};
use thiserror::Error;

use crate::clock::Clock;
use crate::container::{ContainerEngine, ContainerEngineError, EnvVar, ResourceLimits};

/// Per-invocation context handed to every [`LevelExtension`].
///
/// Lifetimes are bounded to the single `EvaluationPipeline::run` call
/// that spawns the extension.
pub struct ExtensionContext<'a> {
    /// Normalized task manifest.
    pub task: &'a BenchmarkTask,
    /// Candidate resolution.
    pub candidate: &'a CandidateResolution,
    /// Patch bytes (may be empty).
    pub patch_bytes: &'a [u8],
    /// Unpatched workspace template. Never mutated.
    pub workspace_template: &'a Path,
    /// Per-run staging root. Extensions may create fresh
    /// `workspace_<level>_<n>` subdirectories here. Contents are not
    /// hashed.
    pub staging_root: &'a Path,
    /// Bundle directory. Extensions may write additional artifacts here;
    /// every artifact is included in the bundle hash by
    /// [`BundleBuilder::finalize_at`].
    pub bundle_dir: &'a Path,
    /// Resolved image reference from [`ContainerEngine::prepare_image`].
    pub image_ref: &'a str,
    /// Environment variables shared across the pipeline.
    pub env: &'a [EnvVar],
    /// Resource limits applied per exec.
    pub resource_limits: &'a ResourceLimits,
    /// Container engine. Passed as a trait object so the extension
    /// crate does not need a generic parameter.
    pub engine: &'a dyn ContainerEngine,
    /// Clock. Extensions must route every timestamp through this.
    pub clock: &'a dyn Clock,
    /// L0 verdict (already computed).
    pub l0: &'a EvaluationResult,
    /// L1 verdict (already computed and reconciled).
    pub l1: &'a EvaluationResult,
    /// Deterministic run id for this invocation.
    pub run_id: RunId,
    /// Candidate id (cached for convenience).
    pub candidate_id: CandidateId,
    /// Task id (cached for convenience).
    pub task_id: TaskId,
}

/// Error returned by an extension. Wraps an arbitrary boxed error so
/// downstream crates can use their own error types without the runner
/// having to know about them.
#[derive(Debug, Error)]
pub enum ExtensionError {
    /// The extension's own error type, boxed.
    #[error("extension {name} failed: {source}")]
    Inner {
        /// Stable extension name (same as [`LevelExtension::name`]).
        name: &'static str,
        /// Underlying error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Trace writer failure.
    #[error("trace: {0}")]
    Trace(#[from] TraceWriterError),
    /// Container engine failure surfaced by the extension.
    #[error("container: {0}")]
    Container(#[from] ContainerEngineError),
    /// Filesystem I/O failure surfaced by the extension.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl ExtensionError {
    /// Wrap a boxable error as [`ExtensionError::Inner`].
    #[must_use]
    pub fn inner<E: Into<Box<dyn std::error::Error + Send + Sync>>>(
        name: &'static str,
        e: E,
    ) -> Self {
        Self::Inner {
            name,
            source: e.into(),
        }
    }
}

/// Trait implemented by every post-L1 extension.
///
/// Implementers typically hold the spec and rules they were configured
/// with and produce a single [`EvaluationResult`] for their level. They
/// may append any number of trace events in addition, and may write any
/// number of auxiliary artifacts under [`ExtensionContext::bundle_dir`].
pub trait LevelExtension: Send + Sync + std::fmt::Debug {
    /// Stable short name (for example `"l2_strengthening"`).
    fn name(&self) -> &'static str;
    /// Evaluation level produced by this extension.
    fn level(&self) -> EvaluationLevel;
    /// Filename inside the bundle for this extension's
    /// `EvaluationResult` (for example `"strengthened_results.json"`).
    fn result_file(&self) -> &'static str;
    /// Execute the extension. May append events to `trace` and may
    /// write artifacts under `ctx.bundle_dir`.
    fn run(
        &self,
        ctx: &ExtensionContext<'_>,
        trace: &mut TraceWriter,
    ) -> Result<EvaluationResult, ExtensionError>;
}

/// Convenience: the current clock reading.
#[must_use]
pub fn now(ctx: &ExtensionContext<'_>) -> DateTime<Utc> {
    ctx.clock.now()
}
