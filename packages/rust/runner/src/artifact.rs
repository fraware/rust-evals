//! Run artifacts surfaced by the runner to its callers.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Outcome of a single runner invocation (official or strengthened).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    /// The scored check passed.
    Pass,
    /// The scored check failed.
    Fail,
    /// The check could not produce a verdict (harness error).
    Invalid,
}

/// Artifacts produced by a runner invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunArtifact {
    /// Outcome of the check.
    pub outcome: RunOutcome,
    /// Stdout log path inside the evidence bundle.
    pub stdout_path: PathBuf,
    /// Stderr log path inside the evidence bundle.
    pub stderr_path: PathBuf,
    /// Wall time in seconds.
    pub wall_time_secs: f64,
    /// Exit code of the underlying process, when applicable.
    pub exit_code: Option<i32>,
    /// Additional artifact paths (relative to the evidence bundle root).
    #[serde(default)]
    pub extra_artifacts: Vec<PathBuf>,
}
