//! Augmented unit tests validator.
//!
//! Runs a spec-declared list of commands in a fresh patched workspace.
//! The family passes iff every command exits with its expected status
//! code (default `0`) and does not time out. Each command produces its
//! own [`SubCheckResult`] so the analysis layer can attribute L2 drops
//! to specific augmented tests.
//!
//! # Generation hygiene
//!
//! The spec itself is the unit of version control; generated tests must
//! be committed to the spec file with a frozen prompt/config. Flaky
//! tests must be marked [`CommandSpec::flaky`] so reviewers can audit
//! which parts of the augmented suite are unreliable.

use serde_json::json;

use crate::context::ValidationContext;
use crate::exec::{
    outcome_matches_expectation, prepare_patched_workspace, run_command, summarize_stderr,
};
use crate::spec::CommandSpec;
use crate::validator::{SubCheckResult, SubVerdict, Validator, ValidatorError, ValidatorVerdict};

use eval_ladder_core::FailureReason;

/// Augmented unit tests validator.
#[derive(Debug, Default, Clone, Copy)]
pub struct AugmentedUnitTests;

/// Stable name used in verdicts and trace events.
pub const VALIDATOR_NAME: &str = "augmented_unit_tests";

impl Validator for AugmentedUnitTests {
    fn name(&self) -> &'static str {
        VALIDATOR_NAME
    }

    fn run(&self, ctx: &ValidationContext<'_>) -> Result<ValidatorVerdict, ValidatorError> {
        if ctx.spec.augmented.commands.is_empty() {
            return Ok(ValidatorVerdict::not_applicable(
                VALIDATOR_NAME,
                "no augmented commands in spec",
            ));
        }

        let (workspace, _patch_outcome) = prepare_patched_workspace(
            ctx.workspace_template,
            ctx.staging_root,
            "workspace_l2_augmented",
            Some(ctx.patch_bytes),
        )?;

        let mut sub_checks = Vec::with_capacity(ctx.spec.augmented.commands.len());
        let mut any_fail = false;

        for cmd in &ctx.spec.augmented.commands {
            let result = run_one(ctx, &workspace, cmd)?;
            if result.verdict.is_fail() {
                any_fail = true;
            }
            sub_checks.push(result);
        }

        let (verdict, primary_reason) = if any_fail {
            (SubVerdict::Fail, FailureReason::L2_AUG_TESTS_FAIL)
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
