//! TOML policy document model.

use std::path::Path;

use eval_ladder_traces::EventType;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Network isolation mode for the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    /// No network access is allowed.
    #[default]
    Disabled,
    /// Access is allowed only to hosts in the allow-list declared by the task.
    HostAllowlist,
    /// Explicit "no network" mode; equivalent to `disabled` but set by tasks
    /// that never need network.
    None,
}

/// A declarative L3 policy.
///
/// Mirrors the shape documented in `docs/evaluation_ladder.md`. Unknown TOML
/// keys are rejected to avoid silent misconfigurations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Policy name (for attribution in reports).
    pub name: String,

    /// Whether the run must declare a reproducible seed.
    #[serde(default)]
    pub requires_reproducible_seed: bool,

    /// Maximum number of files the patch may modify. `None` = unbounded.
    #[serde(default)]
    pub max_modified_files: Option<u32>,

    /// Whether generated tests may be present in the evaluation bundle.
    #[serde(default)]
    pub allow_generated_tests: bool,

    /// Whether dependency lockfile edits are allowed.
    #[serde(default)]
    pub allow_dependency_lockfile_edits: bool,

    /// Network access mode.
    #[serde(default)]
    pub network_mode: NetworkMode,

    /// Commands the run is allowed to execute.
    #[serde(default)]
    pub allowed_commands: Vec<String>,

    /// Commands that are explicitly forbidden.
    #[serde(default)]
    pub forbidden_commands: Vec<String>,

    /// Edit-scope allow-list (glob patterns).
    #[serde(default)]
    pub allowed_edit_globs: Vec<String>,

    /// Edit-scope deny-list (glob patterns).
    #[serde(default)]
    pub forbidden_edit_globs: Vec<String>,

    /// Trace events that must all be present in the run's trace.
    #[serde(default)]
    pub required_trace_events: Vec<EventType>,
}

impl Policy {
    /// Load a policy from a TOML file.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, PolicyLoadError> {
        let bytes = std::fs::read_to_string(path.as_ref()).map_err(|e| PolicyLoadError::Io {
            path: path.as_ref().display().to_string(),
            source: e,
        })?;
        Self::from_toml_str(&bytes)
    }

    /// Parse a policy from a TOML string.
    pub fn from_toml_str(s: &str) -> Result<Self, PolicyLoadError> {
        toml::from_str::<Self>(s).map_err(PolicyLoadError::Parse)
    }
}

/// Errors produced when loading a policy file.
#[derive(Debug, Error)]
pub enum PolicyLoadError {
    /// File system error while reading the policy file.
    #[error("policy io ({path}): {source}")]
    Io {
        /// Path the loader tried to read.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// TOML parse error.
    #[error("policy parse: {0}")]
    Parse(#[from] toml::de::Error),
}
