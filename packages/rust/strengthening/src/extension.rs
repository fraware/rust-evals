//! [`LevelExtension`] implementation for L2 strengthening.
//!
//! Threaded into [`eval_ladder_runner::EvaluationPipeline`] via
//! `PipelineInputs::extensions`. The extension:
//!
//! 1. Emits a `StrengthenedEvalStarted` trace event.
//! 2. Runs each enabled validator family, in declaration order. Each
//!    family produces a [`ValidatorVerdict`] with per-sub-check
//!    breakdown.
//! 3. Aggregates the verdicts into a single
//!    [`eval_ladder_core::EvaluationResult`] keyed at
//!    [`EvaluationLevel::L2Strengthened`].
//! 4. Emits a `StrengthenedEvalFinished` trace event with the same
//!    per-family summary.
//! 5. Writes a full per-validator payload into
//!    `strengthening_report.json` inside the bundle so reviewers can
//!    audit every sub-check.
//!
//! The aggregate L2 verdict is `Pass` iff every enabled family's
//! verdict is `Pass` or `NotApplicable`. The primary reason is taken
//! from the first family to fail (declaration order:
//! augmented -> differential -> regression -> `property_fuzz`).

use std::path::Path;

use eval_ladder_core::{
    EvaluationLevel, EvaluationResult, EvaluationStatus, EvaluatorVersion, FailureReason,
    EVALUATOR_VERSION,
};
use eval_ladder_runner::{ExtensionContext, ExtensionError, LevelExtension, StrengtheningRules};
use eval_ladder_traces::{EventType, TraceWriter};
use serde::Serialize;
use serde_json::json;

use crate::augmented_tests::{AugmentedUnitTests, VALIDATOR_NAME as AUG_NAME};
use crate::context::ValidationContext;
use crate::differential::{DifferentialBehaviorCheck, VALIDATOR_NAME as DIFF_NAME};
use crate::modes::StrengtheningMode;
use crate::property_fuzz::{PropertyFuzzCheck, VALIDATOR_NAME as FUZZ_NAME};
use crate::regression::{TargetedRegressionCheck, VALIDATOR_NAME as REG_NAME};
use crate::spec::StrengtheningSpec;
use crate::validator::{SubVerdict, Validator, ValidatorVerdict};

/// L2 extension plugged into [`eval_ladder_runner::EvaluationPipeline`].
#[derive(Debug)]
pub struct L2Extension<'a> {
    spec: &'a StrengtheningSpec,
    rules: StrengtheningRules,
    oracle_patch_bytes: Option<&'a [u8]>,
}

impl<'a> L2Extension<'a> {
    /// Build an L2 extension with all validator families enabled by
    /// `mode`.
    #[must_use]
    pub fn new(spec: &'a StrengtheningSpec, mode: StrengtheningMode) -> Self {
        Self {
            spec,
            rules: mode.rules(),
            oracle_patch_bytes: None,
        }
    }

    /// Build with explicit [`StrengtheningRules`] (useful when the mode
    /// was decoded from config without going through [`StrengtheningMode`]).
    #[must_use]
    pub fn with_rules(spec: &'a StrengtheningSpec, rules: StrengtheningRules) -> Self {
        Self {
            spec,
            rules,
            oracle_patch_bytes: None,
        }
    }

    /// Attach oracle patch bytes for the differential validator. Returns
    /// `self` for builder chaining.
    #[must_use]
    pub fn with_oracle_patch(mut self, bytes: &'a [u8]) -> Self {
        self.oracle_patch_bytes = Some(bytes);
        self
    }

    /// Read-only view of the rules.
    #[must_use]
    pub fn rules(&self) -> &StrengtheningRules {
        &self.rules
    }
}

/// Stable filename used in the evidence bundle for L2 results.
pub const L2_RESULT_FILE: &str = "strengthened_results.json";

/// Filename for the per-validator report (full sub-check breakdown).
pub const L2_REPORT_FILE: &str = "strengthening_report.json";

/// Stable extension name.
pub const L2_EXTENSION_NAME: &str = "l2_strengthening";

impl LevelExtension for L2Extension<'_> {
    fn name(&self) -> &'static str {
        L2_EXTENSION_NAME
    }

    fn level(&self) -> EvaluationLevel {
        EvaluationLevel::L2Strengthened
    }

    fn result_file(&self) -> &'static str {
        L2_RESULT_FILE
    }

    fn run(
        &self,
        ctx: &ExtensionContext<'_>,
        trace: &mut TraceWriter,
    ) -> Result<EvaluationResult, ExtensionError> {
        let started_at = ctx.clock.now();
        trace.append_at(
            EventType::StrengthenedEvalStarted,
            json!({
                "mode": self.rules.mode,
                "run_augmented_unit_tests": self.rules.run_augmented_unit_tests,
                "run_differential_behavior": self.rules.run_differential_behavior,
                "run_targeted_regression": self.rules.run_targeted_regression,
                "run_property_fuzz": self.rules.run_property_fuzz,
            }),
            started_at,
        )?;

        let vctx = ValidationContext::from_extension(ctx, self.spec, self.oracle_patch_bytes);

        let mut verdicts: Vec<ValidatorVerdict> = Vec::new();

        if self.rules.run_augmented_unit_tests {
            verdicts.push(run_one(&AugmentedUnitTests, &vctx).map_err(wrap_err(AUG_NAME))?);
        }
        if self.rules.run_differential_behavior {
            verdicts.push(run_one(&DifferentialBehaviorCheck, &vctx).map_err(wrap_err(DIFF_NAME))?);
        }
        if self.rules.run_targeted_regression {
            verdicts.push(run_one(&TargetedRegressionCheck, &vctx).map_err(wrap_err(REG_NAME))?);
        }
        if self.rules.run_property_fuzz {
            verdicts.push(run_one(&PropertyFuzzCheck, &vctx).map_err(wrap_err(FUZZ_NAME))?);
        }

        let aggregate = aggregate(&verdicts);

        // Serialize the full per-validator breakdown into the bundle so
        // reviewers can inspect every sub-check.
        let report = StrengtheningReport {
            schema_version: 1,
            evaluator_version: EVALUATOR_VERSION,
            mode: self.rules.mode.clone(),
            verdicts: verdicts.clone(),
        };
        write_canonical_json(&ctx.bundle_dir.join(L2_REPORT_FILE), &report)?;

        let finished_at = ctx.clock.now();
        trace.append_at(
            EventType::StrengthenedEvalFinished,
            json!({
                "mode": self.rules.mode,
                "aggregate_verdict": aggregate.status,
                "aggregate_primary_reason": aggregate.primary_reason,
                "per_validator": verdicts.iter().map(|v| json!({
                    "validator": v.validator,
                    "verdict": v.verdict,
                    "primary_reason": v.primary_reason,
                })).collect::<Vec<_>>(),
            }),
            finished_at,
        )?;

        let mut result = EvaluationResult::new(
            ctx.candidate_id,
            ctx.task_id.clone(),
            EvaluationLevel::L2Strengthened,
            aggregate.status,
            aggregate.primary_reason.as_str().to_owned(),
            started_at,
            finished_at,
        );
        result.secondary_reasons = aggregate.secondary_reasons;
        result.metrics = aggregate.metrics;
        Ok(result)
    }
}

#[derive(Serialize)]
struct StrengtheningReport {
    schema_version: u32,
    evaluator_version: EvaluatorVersion,
    mode: String,
    verdicts: Vec<ValidatorVerdict>,
}

fn run_one(
    v: &dyn Validator,
    ctx: &ValidationContext<'_>,
) -> Result<ValidatorVerdict, crate::validator::ValidatorError> {
    v.run(ctx)
}

fn wrap_err(name: &'static str) -> impl Fn(crate::validator::ValidatorError) -> ExtensionError {
    move |e| ExtensionError::inner(name, e)
}

struct Aggregate {
    status: EvaluationStatus,
    primary_reason: FailureReason,
    secondary_reasons: Vec<String>,
    metrics: serde_json::Value,
}

fn aggregate(verdicts: &[ValidatorVerdict]) -> Aggregate {
    let mut any_fail: Option<FailureReason> = None;
    let mut any_invalid = false;
    let mut secondary: Vec<String> = Vec::new();
    let mut ran = 0usize;
    let mut not_applicable = 0usize;

    for v in verdicts {
        match v.verdict {
            SubVerdict::Pass => {
                ran += 1;
            }
            SubVerdict::Fail => {
                ran += 1;
                if any_fail.is_none() {
                    any_fail = Some(v.primary_reason);
                } else {
                    secondary.push(v.primary_reason.as_str().to_owned());
                }
            }
            SubVerdict::Invalid => {
                ran += 1;
                any_invalid = true;
                secondary.push(v.primary_reason.as_str().to_owned());
            }
            SubVerdict::NotApplicable => {
                not_applicable += 1;
            }
        }
    }

    let (status, primary_reason) = match any_fail {
        Some(reason) => (EvaluationStatus::Fail, reason),
        None if any_invalid => (EvaluationStatus::Invalid, FailureReason::L2_AUG_TESTS_FAIL),
        None if ran == 0 => (EvaluationStatus::NotApplicable, FailureReason::PASS),
        None => (EvaluationStatus::Pass, FailureReason::PASS),
    };

    let metrics = json!({
        "validators_total": verdicts.len(),
        "validators_ran": ran,
        "validators_not_applicable": not_applicable,
        "per_validator": verdicts.iter().map(|v| json!({
            "validator": v.validator,
            "verdict": v.verdict,
            "primary_reason": v.primary_reason,
            "metrics": v.metrics,
        })).collect::<Vec<_>>(),
    });

    Aggregate {
        status,
        primary_reason,
        secondary_reasons: secondary,
        metrics,
    }
}

fn write_canonical_json<T: Serialize>(path: &Path, value: &T) -> Result<(), ExtensionError> {
    let mut bytes = eval_ladder_core::canonical_json(value).map_err(|e| {
        ExtensionError::inner(
            L2_EXTENSION_NAME,
            Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()),
        )
    })?;
    bytes.push(b'\n');
    std::fs::write(path, bytes)?;
    Ok(())
}
