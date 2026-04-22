//! Shared types for L2 sub-validators.
//!
//! A "validator" is a single L2 sub-check family such as augmented
//! tests, targeted regression, or differential behaviour. Each family
//! operates over the same [`crate::context::ValidationContext`] and
//! emits a [`ValidatorVerdict`] with a per-sub-check breakdown in
//! `sub_checks`.

use eval_ladder_core::{CoreError, FailureReason};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use eval_ladder_runner::{ContainerEngineError, PatchApplyError, WorkspaceError};

/// Per sub-validator verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubVerdict {
    /// Passed.
    Pass,
    /// Failed (see `primary_reason`).
    Fail,
    /// Could not produce a verdict (harness error).
    Invalid,
    /// Does not apply.
    NotApplicable,
}

impl SubVerdict {
    /// True if this verdict counts as a definitive fail that should
    /// drive the aggregate L2 verdict to fail.
    #[must_use]
    pub const fn is_fail(self) -> bool {
        matches!(self, Self::Fail)
    }
    /// True if this verdict counts as a non-fail (pass or not-applicable).
    #[must_use]
    pub const fn is_non_fail(self) -> bool {
        matches!(self, Self::Pass | Self::NotApplicable)
    }
}

/// Result of one sub-check inside a validator family.
#[allow(clippy::derive_partial_eq_without_eq)] // `metrics` is `Value`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubCheckResult {
    /// Stable id from the spec, e.g. `"aug_edge_cases"`.
    pub id: String,
    /// Outcome.
    pub verdict: SubVerdict,
    /// Observed exit code if the sub-check ran a command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Whether the sub-check timed out.
    #[serde(default)]
    pub timed_out: bool,
    /// Short human-readable explanation (for example, a truncated stderr
    /// head). Bounded to ~1 KiB to keep bundles tight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Family-specific metrics.
    #[serde(default)]
    pub metrics: Value,
}

/// Verdict of a single [`Validator`] invocation.
///
/// `metrics` is a free-form `serde_json::Value`, which is not `Eq`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatorVerdict {
    /// Short, machine-stable name of the validator (e.g. `"augmented_unit_tests"`).
    pub validator: String,
    /// Aggregate outcome (conjunction of sub-checks).
    pub verdict: SubVerdict,
    /// Primary failure reason, using the shared [`FailureReason`] code
    /// set. For a passing validator this is
    /// [`FailureReason::PASS`].
    pub primary_reason: FailureReason,
    /// Per-sub-check breakdown.
    #[serde(default)]
    pub sub_checks: Vec<SubCheckResult>,
    /// Family-level metrics.
    pub metrics: Value,
}

impl ValidatorVerdict {
    /// Build a [`ValidatorVerdict::NotApplicable`] with a canned metrics
    /// object noting the reason.
    #[must_use]
    pub fn not_applicable(name: &str, reason: &str) -> Self {
        Self {
            validator: name.to_owned(),
            verdict: SubVerdict::NotApplicable,
            primary_reason: FailureReason::PASS,
            sub_checks: Vec::new(),
            metrics: serde_json::json!({ "reason": reason }),
        }
    }
}

/// Errors produced by validators.
#[derive(Debug, Error)]
pub enum ValidatorError {
    /// Core-layer error.
    #[error("core: {0}")]
    Core(#[from] CoreError),
    /// Workspace preparation failed.
    #[error("workspace: {0}")]
    Workspace(#[from] WorkspaceError),
    /// Patch apply failed.
    #[error("patch apply: {0}")]
    Patch(#[from] PatchApplyError),
    /// Container engine failure.
    #[error("container: {0}")]
    Container(#[from] ContainerEngineError),
    /// Filesystem I/O error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Validator received malformed spec or inputs.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

/// Trait implemented by every L2 sub-validator.
pub trait Validator: std::fmt::Debug + Send + Sync {
    /// Stable short name (for example `augmented_unit_tests`).
    fn name(&self) -> &'static str;

    /// Execute the validator on a prepared context.
    fn run(
        &self,
        ctx: &crate::context::ValidationContext<'_>,
    ) -> Result<ValidatorVerdict, ValidatorError>;
}
