//! Benchmark-agnostic scoring abstraction.
//!
//! A [`Scorer`] turns an [`ExecOutcome`] (the raw capture from running the
//! benchmark's official test entrypoint inside a container) into a
//! [`ScorerVerdict`]: a stable pass/fail/invalid classification plus a
//! `primary_reason` code and a free-form `metrics` payload.
//!
//! Each benchmark family (SWE-bench harness JSON, pytest output,
//! `cargo test` JSON, etc.) gets its own `Scorer` implementation. The
//! fixture scorer defined here, [`SimpleExitCodeScorer`], treats a zero
//! exit code as pass and anything else as fail; it is used by the
//! Milestone C rerun-determinism acceptance test and should not be used
//! for real benchmark submissions.

use eval_ladder_core::FailureReason;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::artifact::RunOutcome;
use crate::container::ExecOutcome;

/// Scorer verdict for one evaluator invocation.
///
/// `metrics` is a free-form `serde_json::Value` (which cannot implement
/// `Eq`, since it permits `f64`). `ScorerVerdict` therefore cannot
/// derive `Eq`; the clippy lint is allowed intentionally.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScorerVerdict {
    /// Normalized pass/fail/invalid outcome.
    pub outcome: RunOutcome,
    /// Stable reason code. `"PASS"` if and only if `outcome == Pass`.
    pub primary_reason: String,
    /// Ordered list of secondary reason codes. May be empty.
    #[serde(default)]
    pub secondary_reasons: Vec<String>,
    /// Free-form metrics (test counts, assertion counts, etc).
    pub metrics: Value,
}

impl ScorerVerdict {
    /// Construct a passing verdict with no metrics payload.
    #[must_use]
    pub fn pass() -> Self {
        Self {
            outcome: RunOutcome::Pass,
            primary_reason: FailureReason::PASS.as_str().to_owned(),
            secondary_reasons: Vec::new(),
            metrics: Value::Object(serde_json::Map::new()),
        }
    }

    /// Construct a failing verdict from a stable [`FailureReason`].
    #[must_use]
    pub fn fail(reason: FailureReason, metrics: Value) -> Self {
        Self {
            outcome: RunOutcome::Fail,
            primary_reason: reason.as_str().to_owned(),
            secondary_reasons: Vec::new(),
            metrics,
        }
    }

    /// Construct an invalid verdict (harness error, not a legitimate fail).
    #[must_use]
    pub fn invalid(reason: FailureReason, metrics: Value) -> Self {
        Self {
            outcome: RunOutcome::Invalid,
            primary_reason: reason.as_str().to_owned(),
            secondary_reasons: Vec::new(),
            metrics,
        }
    }
}

/// Turns raw container captures into normalized verdicts.
///
/// Implementations must be pure (no I/O), deterministic, and must never
/// silently coerce a harness error into `Fail`: use
/// [`ScorerVerdict::invalid`] instead.
pub trait Scorer: Send + Sync + std::fmt::Debug {
    /// Score one execution.
    fn score(&self, exec: &ExecOutcome) -> ScorerVerdict;
}

/// Pass iff the captured exit code is exactly zero.
///
/// Maps raw executions to the L0 failure-reason vocabulary:
/// - `exit_code == Some(0)` -> `Pass` / `"PASS"`
/// - `exit_code == Some(n)` with `n != 0` -> `Fail` / `"L0_OFFICIAL_FAIL"`
/// - `timed_out == true` -> `Invalid` / `"L0_OFFICIAL_TIMEOUT"`
/// - otherwise (no exit code, no timeout) -> `Invalid` / `"L0_OFFICIAL_INVALID"`
///
/// The pipeline remaps these codes when producing an L1 result: for L1
/// disagreement the primary reason becomes `L1_RERUN_DISAGREEMENT`, and
/// L1-level errors map to `L1_TIMEOUT` / `L1_HARNESS_ERROR`.
///
/// This scorer exists for fixture and smoke tests; production benchmark
/// scorers must parse structured output.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleExitCodeScorer;

impl Scorer for SimpleExitCodeScorer {
    fn score(&self, exec: &ExecOutcome) -> ScorerVerdict {
        match (exec.exit_code, exec.timed_out) {
            (Some(0), false) => ScorerVerdict {
                outcome: RunOutcome::Pass,
                primary_reason: FailureReason::PASS.as_str().to_owned(),
                secondary_reasons: Vec::new(),
                metrics: json!({ "exit_code": 0 }),
            },
            (Some(n), false) => ScorerVerdict {
                outcome: RunOutcome::Fail,
                primary_reason: FailureReason::L0_OFFICIAL_FAIL.as_str().to_owned(),
                secondary_reasons: Vec::new(),
                metrics: json!({ "exit_code": n }),
            },
            (_, true) => ScorerVerdict {
                outcome: RunOutcome::Invalid,
                primary_reason: FailureReason::L0_OFFICIAL_TIMEOUT.as_str().to_owned(),
                secondary_reasons: Vec::new(),
                metrics: json!({ "timed_out": true }),
            },
            (None, false) => ScorerVerdict {
                outcome: RunOutcome::Invalid,
                primary_reason: FailureReason::L0_OFFICIAL_INVALID.as_str().to_owned(),
                secondary_reasons: Vec::new(),
                metrics: json!({ "exit_code": null }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(exit: Option<i32>, timed_out: bool) -> ExecOutcome {
        ExecOutcome {
            exit_code: exit,
            stdout: String::new(),
            stderr: String::new(),
            wall_time_secs: 0.0,
            timed_out,
        }
    }

    #[test]
    fn zero_exit_is_pass() {
        let v = SimpleExitCodeScorer.score(&outcome(Some(0), false));
        assert_eq!(v.outcome, RunOutcome::Pass);
        assert_eq!(v.primary_reason, "PASS");
    }

    #[test]
    fn nonzero_exit_is_l0_official_fail() {
        let v = SimpleExitCodeScorer.score(&outcome(Some(1), false));
        assert_eq!(v.outcome, RunOutcome::Fail);
        assert_eq!(v.primary_reason, "L0_OFFICIAL_FAIL");
    }

    #[test]
    fn timeout_is_invalid() {
        let v = SimpleExitCodeScorer.score(&outcome(None, true));
        assert_eq!(v.outcome, RunOutcome::Invalid);
        assert_eq!(v.primary_reason, "L0_OFFICIAL_TIMEOUT");
    }

    #[test]
    fn none_exit_not_timeout_is_invalid() {
        let v = SimpleExitCodeScorer.score(&outcome(None, false));
        assert_eq!(v.outcome, RunOutcome::Invalid);
        assert_eq!(v.primary_reason, "L0_OFFICIAL_INVALID");
    }
}
