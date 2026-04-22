//! [`LeanChecker`] trait and its default process-spawning
//! implementation.
//!
//! Separating the trait from the implementation lets acceptance tests
//! drive the extension with a deterministic in-process double
//! ([`crate::scripted::ScriptedChecker`]) while production CLIs can
//! spawn `lake env lean` (or any other Lean-adjacent tool) via
//! [`ExternalProcessChecker`].

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::spec::ProofObligation;

/// Three-valued verdict returned by a Lean checker.
///
/// Stable wire format: `valid`, `invalid`, `not_applicable`. Mirrors
/// the verdict vocabulary in `docs/evaluation_ladder.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeanStatus {
    /// The Lean checker accepted the obligation.
    Valid,
    /// The Lean checker rejected the obligation.
    Invalid,
    /// The obligation does not apply to this candidate (for example,
    /// extraction produced no context).
    NotApplicable,
}

/// Structured output returned by a [`LeanChecker`].
///
/// `code` is a stable uppercase code (`^[A-Z][A-Z0-9_]*$`). For
/// `Valid` outcomes the code typically matches the obligation's
/// `pass_criterion`. For `Invalid` / `NotApplicable` outcomes it is
/// one of the `L4_*` codes from `eval_ladder_core::FailureReason`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeanCheckOutcome {
    /// Three-valued status.
    pub status: LeanStatus,
    /// Stable uppercase code.
    pub code: String,
    /// Human-readable message (free form).
    pub message: String,
    /// Structured checker payload. Opaque to this crate; reviewers
    /// consume it via `proof_results.json`. `Value::Null` is allowed
    /// for minimal checkers.
    #[serde(default)]
    pub payload: serde_json::Value,
}

impl LeanCheckOutcome {
    /// Convenience: build a [`LeanStatus::Valid`] outcome.
    #[must_use]
    pub fn valid(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: LeanStatus::Valid,
            code: code.into(),
            message: message.into(),
            payload: serde_json::Value::Null,
        }
    }

    /// Convenience: build a [`LeanStatus::Invalid`] outcome.
    #[must_use]
    pub fn invalid(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: LeanStatus::Invalid,
            code: code.into(),
            message: message.into(),
            payload: serde_json::Value::Null,
        }
    }

    /// Convenience: build a [`LeanStatus::NotApplicable`] outcome.
    #[must_use]
    pub fn not_applicable(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: LeanStatus::NotApplicable,
            code: code.into(),
            message: message.into(),
            payload: serde_json::Value::Null,
        }
    }
}

/// Context passed to [`LeanChecker::check`].
///
/// All paths are absolute; borrow lifetimes are tied to the
/// `LevelExtension::run` call.
pub struct LeanCheckContext<'a> {
    /// Root of the Lean project (for example
    /// `packages/lean/EvalLadder`). `ExternalProcessChecker` sets
    /// `cwd` to this directory when spawning the checker command.
    pub lean_root: &'a Path,
    /// Absolute path to the patched workspace for this task.
    pub workspace: &'a Path,
    /// Candidate patch bytes. Passed through for checkers that want
    /// to inspect the diff (for example, to reject patches that
    /// touch files outside `target_files`).
    pub patch_bytes: &'a [u8],
}

/// Implemented by anything that can judge a [`ProofObligation`].
pub trait LeanChecker: Send + Sync + std::fmt::Debug {
    /// Produce a verdict for `obligation` against the context. Errors
    /// are reserved for harness failures (spawn errors, unparseable
    /// output); normal `Invalid` / `NotApplicable` verdicts are
    /// returned as [`LeanCheckOutcome`].
    fn check(
        &self,
        obligation: &ProofObligation,
        ctx: &LeanCheckContext<'_>,
    ) -> Result<LeanCheckOutcome, LeanCheckError>;
}

/// Errors raised by a [`LeanChecker`].
#[derive(Debug, Error)]
pub enum LeanCheckError {
    /// Failed to spawn the checker process.
    #[error("lean spawn ({command}): {source}")]
    Spawn {
        /// Command that failed to spawn.
        command: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// Checker produced unparseable output.
    #[error("lean output parse: {0}")]
    Parse(String),
    /// Checker exited non-zero and produced no parseable outcome.
    #[error("lean checker exited with {exit:?}; stderr:\n{stderr}")]
    Exited {
        /// Process exit code (`None` means killed by signal).
        exit: Option<i32>,
        /// Captured stderr (may be truncated by the caller).
        stderr: String,
    },
    /// Generic I/O error (for example, writing the extracted context
    /// to disk).
    #[error("lean io: {0}")]
    Io(#[from] std::io::Error),
}

/// Default production checker. Spawns the obligation-declared
/// `proof_checker.command args...` with cwd = `lean_root`.
///
/// The checker parses a single JSON object from stdout. Extra trailing
/// whitespace is tolerated. The JSON must have the shape:
///
/// ```json
/// { "status": "valid|invalid|not_applicable",
///   "code": "L4_OBLIGATION_MET|...",
///   "message": "free form string",
///   "payload": { ... } }
/// ```
///
/// Non-zero exit codes are tolerated as long as stdout contains a
/// parseable JSON outcome; that lets checkers communicate `Invalid`
/// through a canonical exit code without losing structure.
#[derive(Debug, Clone)]
pub struct ExternalProcessChecker {
    lean_root: PathBuf,
    stderr_budget: usize,
}

impl ExternalProcessChecker {
    /// New checker rooted at `lean_root`.
    #[must_use]
    pub fn new(lean_root: impl Into<PathBuf>) -> Self {
        Self {
            lean_root: lean_root.into(),
            stderr_budget: 8 * 1024,
        }
    }

    /// Set the maximum stderr captured on spawn failure.
    #[must_use]
    pub const fn with_stderr_budget(mut self, bytes: usize) -> Self {
        self.stderr_budget = bytes;
        self
    }
}

impl LeanChecker for ExternalProcessChecker {
    fn check(
        &self,
        obligation: &ProofObligation,
        _ctx: &LeanCheckContext<'_>,
    ) -> Result<LeanCheckOutcome, LeanCheckError> {
        let cmd = &obligation.proof_checker.command;
        let output = Command::new(cmd)
            .args(&obligation.proof_checker.args)
            .current_dir(&self.lean_root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|source| LeanCheckError::Spawn {
                command: cmd.clone(),
                source,
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Result<LeanCheckOutcome, _> = serde_json::from_str(stdout.trim());
        match parsed {
            Ok(outcome) => Ok(outcome),
            Err(e) => {
                if output.status.success() {
                    Err(LeanCheckError::Parse(format!(
                        "checker stdout did not contain a LeanCheckOutcome JSON document: {e}"
                    )))
                } else {
                    let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                    if stderr.len() > self.stderr_budget {
                        stderr.truncate(self.stderr_budget);
                        stderr.push_str("...<truncated>");
                    }
                    Err(LeanCheckError::Exited {
                        exit: output.status.code(),
                        stderr,
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lean_outcome_roundtrips() {
        let o = LeanCheckOutcome::valid("L4_OBLIGATION_MET", "ok");
        let s = serde_json::to_string(&o).unwrap();
        let back: LeanCheckOutcome = serde_json::from_str(&s).unwrap();
        assert_eq!(back, o);
    }

    #[test]
    fn lean_outcome_rejects_unknown_status() {
        let err = serde_json::from_str::<LeanCheckOutcome>(
            r#"{"status":"weird","code":"X","message":"","payload":null}"#,
        );
        assert!(err.is_err());
    }
}
