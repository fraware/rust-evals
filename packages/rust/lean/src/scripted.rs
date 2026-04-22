//! Deterministic test-double [`LeanChecker`].
//!
//! The scripted checker is the primary mechanism for acceptance tests:
//! it lets the Milestone F test matrix cover `Valid`, `Invalid`,
//! `NotApplicable`, and error paths without invoking a real Lean
//! toolchain. Production code paths (`evaluate candidate`,
//! `prove-subset`) should prefer [`crate::checker::ExternalProcessChecker`].

use std::collections::HashMap;
use std::sync::Mutex;

use crate::checker::{LeanCheckContext, LeanCheckError, LeanCheckOutcome, LeanChecker};
use crate::spec::ProofObligation;

/// In-memory checker that returns pre-programmed outcomes keyed by
/// `obligation_id`.
///
/// Cloning the checker preserves the programming; internal state is
/// protected by a single [`Mutex`] so the type is trivially `Send +
/// Sync` without demanding `Sync` from the stored outcomes.
#[derive(Debug, Default)]
pub struct ScriptedChecker {
    inner: Mutex<ScriptedInner>,
}

#[derive(Debug, Default)]
struct ScriptedInner {
    programmed: HashMap<String, Vec<Result<LeanCheckOutcome, ScriptedError>>>,
    default_outcome: Option<LeanCheckOutcome>,
}

/// Serializable error stand-in used by [`ScriptedChecker::program_error`].
#[derive(Debug, Clone)]
pub struct ScriptedError {
    /// Error message surfaced via [`LeanCheckError::Parse`].
    pub message: String,
}

impl ScriptedChecker {
    /// Empty checker. Every `check` call returns
    /// `LeanCheckError::Parse("not programmed")` unless
    /// [`Self::with_default`] or [`Self::program_valid`] etc. are
    /// called first.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the fallback outcome returned when an obligation is not
    /// individually programmed.
    #[must_use]
    pub fn with_default(self, outcome: LeanCheckOutcome) -> Self {
        {
            let mut g = self.inner.lock().expect("ScriptedChecker mutex poisoned");
            g.default_outcome = Some(outcome);
        }
        self
    }

    /// Program a queue of outcomes for `obligation_id`. Each call to
    /// [`LeanChecker::check`] consumes one entry (FIFO). When the
    /// queue is drained, subsequent calls fall back to
    /// [`Self::with_default`].
    pub fn program(&self, obligation_id: impl Into<String>, outcome: LeanCheckOutcome) {
        let mut g = self.inner.lock().expect("ScriptedChecker mutex poisoned");
        g.programmed
            .entry(obligation_id.into())
            .or_default()
            .push(Ok(outcome));
    }

    /// Program a checker error for `obligation_id` (queued like
    /// [`Self::program`]).
    pub fn program_error(&self, obligation_id: impl Into<String>, message: impl Into<String>) {
        let mut g = self.inner.lock().expect("ScriptedChecker mutex poisoned");
        g.programmed
            .entry(obligation_id.into())
            .or_default()
            .push(Err(ScriptedError {
                message: message.into(),
            }));
    }
}

impl LeanChecker for ScriptedChecker {
    fn check(
        &self,
        obligation: &ProofObligation,
        _ctx: &LeanCheckContext<'_>,
    ) -> Result<LeanCheckOutcome, LeanCheckError> {
        let mut guard = self.inner.lock().expect("ScriptedChecker mutex poisoned");
        let queued = guard
            .programmed
            .get_mut(&obligation.obligation_id)
            .and_then(|q| {
                if q.is_empty() {
                    None
                } else {
                    Some(q.remove(0))
                }
            });
        let decision = queued.or_else(|| guard.default_outcome.clone().map(Ok));
        drop(guard);
        match decision {
            Some(Ok(o)) => Ok(o),
            Some(Err(ScriptedError { message })) => Err(LeanCheckError::Parse(message)),
            None => Err(LeanCheckError::Parse(format!(
                "scripted checker has no programmed outcome for obligation_id {:?}",
                obligation.obligation_id
            ))),
        }
    }
}
