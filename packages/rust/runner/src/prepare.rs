//! Prepared-run descriptor.

use std::path::PathBuf;

use eval_ladder_core::{BenchmarkId, CandidateId, RunId, TaskId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::container::ResourceLimits;

/// A run that has been prepared but not yet executed.
///
/// This is the handoff value between adapter-specific preparation logic and
/// the generic [`crate::BenchmarkRunner`] execution path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedRun {
    /// Run identifier. Propagated into every trace event.
    pub run_id: RunId,
    /// Candidate identifier.
    pub candidate_id: CandidateId,
    /// Task identifier.
    pub task_id: TaskId,
    /// Benchmark identifier.
    pub benchmark_id: BenchmarkId,
    /// Resolved image digest (`sha256:<64-hex>`).
    pub image_digest: String,
    /// Workspace directory on the host (already populated with the patched repo).
    pub workspace_dir: PathBuf,
    /// Resource limits selected for this run.
    pub resource_limits: ResourceLimits,
}

/// Errors produced while preparing a run.
#[derive(Debug, Error)]
pub enum PreparedRunError {
    /// Required input file is missing.
    #[error("missing input: {0}")]
    MissingInput(String),
    /// Patch could not be applied.
    #[error("patch apply failed: {0}")]
    PatchApplyFailed(String),
    /// Image digest resolution failed.
    #[error("image digest unresolved for {0}")]
    ImageDigestUnresolved(String),
}
