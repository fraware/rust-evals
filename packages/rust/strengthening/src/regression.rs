//! Targeted regression validator.
//!
//! Runs a spec-declared regression suite in the candidate-patched
//! workspace. Semantically identical to augmented tests, but the
//! separation lets analysis attribute drops to "broke something else"
//! vs "missed an edge case". Both families ship distinct
//! [`FailureReason`] codes.
//!
//! Regression commands should be curated against the files the patch is
//! expected to touch; scoping is up to the spec author.

use serde_json::json;

use crate::context::ValidationContext;
use crate::exec::{
    outcome_matches_expectation, prepare_patched_workspace, run_command, summarize_stderr,
};
use crate::spec::CommandSpec;
use crate::validator::{SubCheckResult, SubVerdict, Validator, ValidatorError, ValidatorVerdict};

use eval_ladder_core::FailureReason;

/// Targeted regression validator.
#[derive(Debug, Default, Clone, Copy)]
pub struct TargetedRegressionCheck;

/// Stable name.
pub const VALIDATOR_NAME: &str = "targeted_regression";

impl Validator for TargetedRegressionCheck {
    fn name(&self) -> &'static str {
        VALIDATOR_NAME
    }

    fn run(&self, ctx: &ValidationContext<'_>) -> Result<ValidatorVerdict, ValidatorError> {
        if ctx.spec.regression.commands.is_empty() {
            return Ok(ValidatorVerdict::not_applicable(
                VALIDATOR_NAME,
                "no regression commands in spec",
            ));
        }

        let (workspace, _patch_outcome) = prepare_patched_workspace(
            ctx.workspace_template,
            ctx.staging_root,
            "workspace_l2_regression",
            Some(ctx.patch_bytes),
        )?;

        let mut sub_checks = Vec::with_capacity(ctx.spec.regression.commands.len());
        let mut any_fail = false;

        for cmd in &ctx.spec.regression.commands {
            let result = run_one(ctx, &workspace, cmd)?;
            if result.verdict.is_fail() {
                any_fail = true;
            }
            sub_checks.push(result);
        }

        let (verdict, primary_reason) = if any_fail {
            (SubVerdict::Fail, FailureReason::L2_REGRESSION_FAIL)
        } else {
            (SubVerdict::Pass, FailureReason::PASS)
        };

        let metrics = json!({
            "total": sub_checks.len(),
            "passed": sub_checks.iter().filter(|s| matches!(s.verdict, SubVerdict::Pass)).count(),
            "failed": sub_checks.iter().filter(|s| matches!(s.verdict, SubVerdict::Fail)).count(),
        });

        Ok(ValidatorVerdict {
            validator: VALIDATOR_NAME.to_owned(),
            verdict,
            primary_reason,
            sub_checks,
            metrics,
        })
    }
}

fn run_one(
    ctx: &ValidationContext<'_>,
    workspace: &std::path::Path,
    cmd: &CommandSpec,
) -> Result<SubCheckResult, ValidatorError> {
    let outcome = run_command(
        ctx.engine,
        ctx.image_ref,
        workspace,
        ctx.env,
        ctx.resource_limits,
        cmd,
    )?;
    let passed = outcome_matches_expectation(&outcome, cmd);
    let verdict = if passed {
        SubVerdict::Pass
    } else {
        SubVerdict::Fail
    };
    let detail = if passed {
        None
    } else {
        summarize_stderr(&outcome, 1024)
    };
    Ok(SubCheckResult {
        id: cmd.id.clone(),
        verdict,
        exit_code: outcome.exit_code,
        timed_out: outcome.timed_out,
        detail,
        metrics: json!({
            "expected_exit_code": cmd.expected_exit_code.unwrap_or(0),
        }),
    })
}
