//! Differential behaviour validator.
//!
//! Runs each declared observable in two isolated workspaces - one
//! patched with the candidate, one patched with the oracle - and
//! compares the chosen stream(s). The family passes iff every
//! observable produces the same output in both workspaces.
//!
//! # Oracle availability
//!
//! The spec declares an `oracle_patch_ref` but the actual bytes are
//! supplied at runtime via
//! [`crate::extension::L2Extension::with_oracle_patch`]. When no oracle
//! patch is available the family emits
//! [`FailureReason::L2_ORACLE_UNAVAILABLE`] with `verdict =
//! NotApplicable` so the aggregate L2 verdict can distinguish "oracle
//! missing" from "oracle diverged".

use serde_json::json;

use crate::context::ValidationContext;
use crate::exec::{prepare_patched_workspace, run_command};
use crate::spec::{CommandSpec, DifferentialCompare, ObservableSpec};
use crate::validator::{SubCheckResult, SubVerdict, Validator, ValidatorError, ValidatorVerdict};

use eval_ladder_core::FailureReason;
use eval_ladder_runner::ExecOutcome;

/// Differential behaviour validator.
#[derive(Debug, Default, Clone, Copy)]
pub struct DifferentialBehaviorCheck;

/// Stable name.
pub const VALIDATOR_NAME: &str = "differential_behavior";

impl Validator for DifferentialBehaviorCheck {
    fn name(&self) -> &'static str {
        VALIDATOR_NAME
    }

    fn run(&self, ctx: &ValidationContext<'_>) -> Result<ValidatorVerdict, ValidatorError> {
        let Some(diff_spec) = ctx.spec.differential.as_ref() else {
            return Ok(ValidatorVerdict::not_applicable(
                VALIDATOR_NAME,
                "no differential spec",
            ));
        };
        if diff_spec.observables.is_empty() {
            return Ok(ValidatorVerdict::not_applicable(
                VALIDATOR_NAME,
                "no observables in differential spec",
            ));
        }
        let Some(oracle_bytes) = ctx.oracle_patch_bytes else {
            return Ok(ValidatorVerdict {
                validator: VALIDATOR_NAME.to_owned(),
                verdict: SubVerdict::NotApplicable,
                primary_reason: FailureReason::L2_ORACLE_UNAVAILABLE,
                sub_checks: Vec::new(),
                metrics: json!({
                    "reason": "no oracle patch supplied",
                    "oracle_patch_ref": diff_spec.oracle_patch_ref,
                }),
            });
        };

        let (workspace_candidate, _) = prepare_patched_workspace(
            ctx.workspace_template,
            ctx.staging_root,
            "workspace_l2_diff_candidate",
            Some(ctx.patch_bytes),
        )?;
        let (workspace_oracle, _) = prepare_patched_workspace(
            ctx.workspace_template,
            ctx.staging_root,
            "workspace_l2_diff_oracle",
            Some(oracle_bytes),
        )?;

        let mut sub_checks = Vec::with_capacity(diff_spec.observables.len());
        let mut any_diverge = false;

        for obs in &diff_spec.observables {
            let (result, diverged) =
                compare_observable(ctx, &workspace_candidate, &workspace_oracle, obs)?;
            if diverged {
                any_diverge = true;
            }
            sub_checks.push(result);
        }

        let (verdict, primary_reason) = if any_diverge {
            (SubVerdict::Fail, FailureReason::L2_DIFF_BEHAVIOR)
        } else {
            (SubVerdict::Pass, FailureReason::PASS)
        };

        let metrics = json!({
            "total": sub_checks.len(),
            "diverged": sub_checks.iter().filter(|s| matches!(s.verdict, SubVerdict::Fail)).count(),
            "oracle_patch_ref": diff_spec.oracle_patch_ref,
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

fn compare_observable(
    ctx: &ValidationContext<'_>,
    candidate_ws: &std::path::Path,
    oracle_ws: &std::path::Path,
    obs: &ObservableSpec,
) -> Result<(SubCheckResult, bool), ValidatorError> {
    let as_cmd = observable_as_command(obs);

    let cand = run_command(
        ctx.engine,
        ctx.image_ref,
        candidate_ws,
        ctx.env,
        ctx.resource_limits,
        &as_cmd,
    )?;
    let oracle = run_command(
        ctx.engine,
        ctx.image_ref,
        oracle_ws,
        ctx.env,
        ctx.resource_limits,
        &as_cmd,
    )?;

    let (diverged, reason) = diff(&cand, &oracle, obs);
    let verdict = if diverged {
        SubVerdict::Fail
    } else {
        SubVerdict::Pass
    };

    let result = SubCheckResult {
        id: obs.id.clone(),
        verdict,
        exit_code: cand.exit_code,
        timed_out: cand.timed_out || oracle.timed_out,
        detail: if diverged { Some(reason) } else { None },
        metrics: json!({
            "compare": obs.compare,
            "candidate_exit_code": cand.exit_code,
            "oracle_exit_code": oracle.exit_code,
        }),
    };
    Ok((result, diverged))
}

fn observable_as_command(obs: &ObservableSpec) -> CommandSpec {
    CommandSpec {
        id: obs.id.clone(),
        command: obs.command.clone(),
        env: obs.env.clone(),
        workdir: obs.workdir.clone(),
        expected_exit_code: None,
        flaky: false,
    }
}

fn diff(cand: &ExecOutcome, oracle: &ExecOutcome, obs: &ObservableSpec) -> (bool, String) {
    let (a, b) = match obs.compare {
        DifferentialCompare::Stdout => (cand.stdout.as_str(), oracle.stdout.as_str()),
        DifferentialCompare::Stderr => (cand.stderr.as_str(), oracle.stderr.as_str()),
        DifferentialCompare::Full => {
            let ca = format!(
                "exit={:?}\nSTDOUT:\n{}STDERR:\n{}",
                cand.exit_code, cand.stdout, cand.stderr
            );
            let cb = format!(
                "exit={:?}\nSTDOUT:\n{}STDERR:\n{}",
                oracle.exit_code, oracle.stdout, oracle.stderr
            );
            if normalize(&ca, obs) != normalize(&cb, obs) {
                return (
                    true,
                    format!(
                        "full outputs diverged (cand exit={:?}, oracle exit={:?})",
                        cand.exit_code, oracle.exit_code
                    ),
                );
            }
            return (false, String::new());
        }
        DifferentialCompare::ExitCode => {
            let diverged = cand.exit_code != oracle.exit_code;
            return (
                diverged,
                if diverged {
                    format!(
                        "exit codes diverged: candidate={:?}, oracle={:?}",
                        cand.exit_code, oracle.exit_code
                    )
                } else {
                    String::new()
                },
            );
        }
    };
    let na = normalize(a, obs);
    let nb = normalize(b, obs);
    if na == nb {
        return (false, String::new());
    }
    (true, summarize_diff(&na, &nb))
}

fn normalize(s: &str, obs: &ObservableSpec) -> String {
    if obs.normalize_trailing_whitespace {
        s.lines().map(str::trim_end).collect::<Vec<_>>().join("\n")
    } else {
        s.to_owned()
    }
}

fn summarize_diff(a: &str, b: &str) -> String {
    let max = 512;
    let head_a = head(a, max);
    let head_b = head(b, max);
    format!("candidate!=oracle; candidate_head={head_a:?}; oracle_head={head_b:?}")
}

fn head(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }
}
