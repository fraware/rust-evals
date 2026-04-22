//! [`LevelExtension`] implementation for L4 proof-carrying obligations.
//!
//! Integrates into [`eval_ladder_runner::EvaluationPipeline`] the same
//! way the L2/L3 extensions do: it is threaded into
//! `PipelineInputs::extensions` and runs after L3 (or L2/L1) under the
//! same trace writer and clock.
//!
//! # Algorithm
//!
//! 1. Look up the obligation for the current task id in the supplied
//!    [`ObligationManifest`].
//! 2. If the task has no obligation: emit a short
//!    `ProofCheckStarted` / `ProofCheckFinished` pair with
//!    `status = "not_applicable"` and return an
//!    `EvaluationStatus::NotApplicable` result keyed with
//!    `L4_OBLIGATION_NOT_APPLICABLE`.
//! 3. Otherwise: emit `ProofCheckStarted`, run the obligation's
//!    checker via the supplied [`LeanChecker`], emit
//!    `ProofCheckFinished` with the verdict, then synthesize the
//!    `EvaluationResult` and write `proof_results.json` to the bundle
//!    directory.
//!
//! # Determinism
//!
//! The extension itself is a deterministic function over
//! `(obligation_manifest, lean_checker, ExtensionContext)`. Production
//! wiring uses [`crate::ExternalProcessChecker`]; real reproducibility
//! of the Lean invocation is the checker's responsibility (pinning
//! `lean-toolchain`, disabling wall-clock randomness etc.). Acceptance
//! tests use [`crate::ScriptedChecker`], which is trivially
//! deterministic, so the Milestone F acceptance test can verify
//! byte-identical bundle hashes across reruns.

use std::path::Path;

use eval_ladder_core::{
    EvaluationLevel, EvaluationResult, EvaluationStatus, FailureReason, TaskId, EVALUATOR_VERSION,
};
use eval_ladder_runner::{ExtensionContext, ExtensionError, LevelExtension};
use eval_ladder_traces::{EventType, TraceWriter};
use serde::Serialize;
use serde_json::json;

use crate::checker::{LeanCheckContext, LeanCheckOutcome, LeanChecker, LeanStatus};
use crate::manifest::ObligationManifest;
use crate::report::{ProofReport, PROOF_REPORT_SCHEMA_VERSION};
use crate::spec::ProofObligation;

/// Stable extension name (trace payloads and logs use it).
pub const L4_EXTENSION_NAME: &str = "l4_lean";

/// Stable filename used in the evidence bundle for the L4 report.
pub const L4_RESULT_FILE: &str = "proof_results.json";

/// L4 extension.
///
/// Holds borrows of the manifest, checker, and Lean project root so
/// callers can stack-allocate it next to the pipeline call and drop
/// it deterministically. The checker is stored as a trait object so
/// downstream crates may plug in custom implementations (for example
/// a remote prover farm) without changing the pipeline wiring.
#[derive(Debug)]
pub struct L4Extension<'a> {
    manifest: &'a ObligationManifest,
    checker: &'a dyn LeanChecker,
    lean_root: &'a Path,
}

impl<'a> L4Extension<'a> {
    /// Build an L4 extension.
    #[must_use]
    pub const fn new(
        manifest: &'a ObligationManifest,
        checker: &'a dyn LeanChecker,
        lean_root: &'a Path,
    ) -> Self {
        Self {
            manifest,
            checker,
            lean_root,
        }
    }

    /// Read-only view of the manifest.
    #[must_use]
    pub const fn manifest(&self) -> &ObligationManifest {
        self.manifest
    }

    /// Read-only view of the Lean project root.
    #[must_use]
    pub const fn lean_root(&self) -> &Path {
        self.lean_root
    }
}

impl LevelExtension for L4Extension<'_> {
    fn name(&self) -> &'static str {
        L4_EXTENSION_NAME
    }

    fn level(&self) -> EvaluationLevel {
        EvaluationLevel::L4Semantic
    }

    fn result_file(&self) -> &'static str {
        L4_RESULT_FILE
    }

    fn run(
        &self,
        ctx: &ExtensionContext<'_>,
        trace: &mut TraceWriter,
    ) -> Result<EvaluationResult, ExtensionError> {
        let started_at = ctx.clock.now();
        let task_key = task_key(&ctx.task_id);
        let obligation = self.manifest.get(&task_key).cloned();

        trace.append_at(
            EventType::ProofCheckStarted,
            json!({
                "task_id": task_key,
                "obligation_id": obligation.as_ref().map(|o| o.obligation_id.clone()),
                "applicable": obligation.is_some(),
            }),
            started_at,
        )?;

        let Some(obligation) = obligation else {
            return handle_not_applicable(ctx, trace, started_at, &task_key);
        };

        let outcome = invoke_checker(self.checker, &obligation, ctx, self.lean_root);
        let finished_at = ctx.clock.now();
        let (status, code, message) = interpret(&obligation, &outcome);

        trace.append_at(
            EventType::ProofCheckFinished,
            json!({
                "task_id": task_key,
                "obligation_id": obligation.obligation_id,
                "status": status_str(status),
                "code": code,
                "message_preview": message_preview(&message),
            }),
            finished_at,
        )?;

        let duration_ms = u128::from(
            (finished_at - started_at)
                .num_milliseconds()
                .max(0)
                .unsigned_abs(),
        );

        let report = ProofReport {
            schema_version: PROOF_REPORT_SCHEMA_VERSION,
            evaluator_version: EVALUATOR_VERSION,
            obligation: Some(obligation.clone()),
            outcome: Some(outcome.clone()),
            status,
            code: code.clone(),
            message: message.clone(),
            duration_ms,
            started_at,
            finished_at,
        };
        write_canonical_json(&ctx.bundle_dir.join(L4_RESULT_FILE), &report)?;

        let mut result = EvaluationResult::new(
            ctx.candidate_id,
            ctx.task_id.clone(),
            EvaluationLevel::L4Semantic,
            to_evaluation_status(status),
            code,
            started_at,
            finished_at,
        );
        result.metrics = json!({
            "obligation_id": obligation.obligation_id,
            "property_type": obligation.property_type,
            "pass_criterion": obligation.pass_criterion,
            "duration_ms": duration_ms,
        });
        Ok(result)
    }
}

fn handle_not_applicable(
    ctx: &ExtensionContext<'_>,
    trace: &mut TraceWriter,
    started_at: chrono::DateTime<chrono::Utc>,
    task_key: &str,
) -> Result<EvaluationResult, ExtensionError> {
    let finished_at = ctx.clock.now();
    let code = FailureReason::L4_OBLIGATION_NOT_APPLICABLE
        .as_str()
        .to_owned();
    let message = format!("no obligation registered for task_id {task_key}");

    trace.append_at(
        EventType::ProofCheckFinished,
        json!({
            "task_id": task_key,
            "obligation_id": serde_json::Value::Null,
            "status": status_str(LeanStatus::NotApplicable),
            "code": code,
            "message_preview": message_preview(&message),
        }),
        finished_at,
    )?;

    let report = ProofReport {
        schema_version: PROOF_REPORT_SCHEMA_VERSION,
        evaluator_version: EVALUATOR_VERSION,
        obligation: None,
        outcome: None,
        status: LeanStatus::NotApplicable,
        code: code.clone(),
        message: message.clone(),
        duration_ms: 0,
        started_at,
        finished_at,
    };
    write_canonical_json(&ctx.bundle_dir.join(L4_RESULT_FILE), &report)?;

    let mut result = EvaluationResult::new(
        ctx.candidate_id,
        ctx.task_id.clone(),
        EvaluationLevel::L4Semantic,
        EvaluationStatus::NotApplicable,
        code,
        started_at,
        finished_at,
    );
    result.metrics = json!({
        "obligation_id": serde_json::Value::Null,
        "duration_ms": 0_u64,
    });
    Ok(result)
}

fn invoke_checker(
    checker: &dyn LeanChecker,
    obligation: &ProofObligation,
    ctx: &ExtensionContext<'_>,
    lean_root: &Path,
) -> LeanCheckOutcome {
    let lctx = LeanCheckContext {
        lean_root,
        workspace: ctx.workspace_template,
        patch_bytes: ctx.patch_bytes,
    };
    match checker.check(obligation, &lctx) {
        Ok(o) => o,
        Err(e) => checker_error_to_outcome(&e),
    }
}

/// Map a [`crate::LeanCheckError`] into an `Invalid` outcome with the
/// stable `L4_PROOF_CHECK_FAILED` code. The checker error is preserved
/// in the payload for reviewers.
fn checker_error_to_outcome(err: &crate::checker::LeanCheckError) -> LeanCheckOutcome {
    let message = err.to_string();
    let payload = json!({
        "error_kind": match err {
            crate::checker::LeanCheckError::Spawn { .. } => "spawn",
            crate::checker::LeanCheckError::Parse(_) => "parse",
            crate::checker::LeanCheckError::Exited { .. } => "exited",
            crate::checker::LeanCheckError::Io(_) => "io",
        },
        "display": message.clone(),
    });
    LeanCheckOutcome {
        status: LeanStatus::Invalid,
        code: FailureReason::L4_PROOF_CHECK_FAILED.as_str().to_owned(),
        message,
        payload,
    }
}

/// Derive the aggregate `(status, code, message)` triple from a
/// checker outcome + obligation. The resulting `code` is:
///
/// - `obligation.pass_criterion` when the checker returned `Valid`
///   and its own `code` matches the expected pass criterion; in that
///   case the aggregate status is `Valid`.
/// - The stable `L4_OBLIGATION_UNMET` when the checker returned
///   `Valid` but with a different code (treated as a disagreement
///   between the obligation and the checker; fails closed).
/// - The checker's own `code` for `Invalid` / `NotApplicable`
///   outcomes.
fn interpret(
    obligation: &ProofObligation,
    outcome: &LeanCheckOutcome,
) -> (LeanStatus, String, String) {
    match outcome.status {
        LeanStatus::Valid if outcome.code == obligation.pass_criterion => (
            LeanStatus::Valid,
            obligation.pass_criterion.clone(),
            outcome.message.clone(),
        ),
        LeanStatus::Valid => (
            LeanStatus::Invalid,
            FailureReason::L4_OBLIGATION_UNMET.as_str().to_owned(),
            format!(
                "checker returned valid with code {:?} but obligation {} expected {:?}",
                outcome.code, obligation.obligation_id, obligation.pass_criterion
            ),
        ),
        LeanStatus::Invalid => (
            LeanStatus::Invalid,
            outcome.code.clone(),
            outcome.message.clone(),
        ),
        LeanStatus::NotApplicable => (
            LeanStatus::NotApplicable,
            if outcome.code.is_empty() {
                FailureReason::L4_OBLIGATION_NOT_APPLICABLE
                    .as_str()
                    .to_owned()
            } else {
                outcome.code.clone()
            },
            outcome.message.clone(),
        ),
    }
}

const fn to_evaluation_status(s: LeanStatus) -> EvaluationStatus {
    match s {
        LeanStatus::Valid => EvaluationStatus::Pass,
        LeanStatus::Invalid => EvaluationStatus::Fail,
        LeanStatus::NotApplicable => EvaluationStatus::NotApplicable,
    }
}

const fn status_str(s: LeanStatus) -> &'static str {
    match s {
        LeanStatus::Valid => "valid",
        LeanStatus::Invalid => "invalid",
        LeanStatus::NotApplicable => "not_applicable",
    }
}

fn task_key(task_id: &TaskId) -> String {
    task_id.as_str().to_owned()
}

fn message_preview(msg: &str) -> String {
    const MAX: usize = 240;
    if msg.len() <= MAX {
        return msg.to_owned();
    }
    let mut end = MAX;
    while end > 0 && !msg.is_char_boundary(end) {
        end -= 1;
    }
    let mut s = msg[..end].to_owned();
    s.push_str("...<truncated>");
    s
}

fn write_canonical_json<T: Serialize>(path: &Path, value: &T) -> Result<(), ExtensionError> {
    let mut bytes = eval_ladder_core::canonical_json(value).map_err(|e| {
        ExtensionError::inner(
            L4_EXTENSION_NAME,
            Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()),
        )
    })?;
    bytes.push(b'\n');
    std::fs::write(path, bytes)?;
    Ok(())
}
