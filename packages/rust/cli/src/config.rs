//! Evaluator configuration loaded from `configs/evaluator/*.toml`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Evaluator configuration.
///
/// Mirrors the TOML files under `configs/evaluator/`. Unknown keys are
/// rejected to prevent silent misconfiguration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorConfig {
    /// Human-readable profile name.
    pub name: String,
    /// Path to the policy TOML document used for L3.
    pub policy_path: String,
    /// Strengthening mode short code (`tests_only`, `tests_plus_diff`,
    /// `tests_plus_regression`, `full_l2`).
    pub strengthening_mode: String,
    /// Path to the ingest manifest directory for this benchmark profile.
    pub manifest_dir: String,
    /// Path to the output directory for evidence bundles.
    pub output_dir: String,
}

/// Errors produced loading an evaluator configuration.
#[derive(Debug, Error)]
pub enum EvaluatorConfigError {
    /// File system error.
    #[error("config io ({path}): {source}")]
    Io {
        /// Path attempted.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// TOML parse error.
    #[error("config parse: {0}")]
    Parse(#[from] toml::de::Error),
}

impl EvaluatorConfig {
    /// Load a configuration from a TOML file.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, EvaluatorConfigError> {
        let p = path.as_ref();
        let bytes = std::fs::read_to_string(p).map_err(|source| EvaluatorConfigError::Io {
            path: p.display().to_string(),
            source,
        })?;
        toml::from_str(&bytes).map_err(EvaluatorConfigError::Parse)
    }
}
