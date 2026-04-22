//! Property-based or mutation-based fuzz validator.
//!
//! Scheduled for Milestone D+. Ships as a
//! `NotApplicable`-emitting validator so the `full_l2` mode stays
//! representable in the spec and analysis code without a crash or a
//! hidden skip.
//!
//! When the real implementation lands, it will consume
//! [`crate::spec::PropertyFuzzSpec`], seed a PRNG deterministically from
//! the run identity, and run the declared property checker against the
//! candidate-patched workspace.

use serde_json::json;

use crate::context::ValidationContext;
use crate::validator::{SubVerdict, Validator, ValidatorError, ValidatorVerdict};

use eval_ladder_core::FailureReason;

/// Property-based fuzz validator.
#[derive(Debug, Default, Clone, Copy)]
pub struct PropertyFuzzCheck;

/// Stable name.
pub const VALIDATOR_NAME: &str = "property_fuzz";

impl Validator for PropertyFuzzCheck {
    fn name(&self) -> &'static str {
        VALIDATOR_NAME
    }

    fn run(&self, ctx: &ValidationContext<'_>) -> Result<ValidatorVerdict, ValidatorError> {
        let reason = match ctx.spec.property_fuzz.as_ref() {
            None => "no property_fuzz spec",
            Some(_) => "property_fuzz runner scheduled for Milestone D+",
        };
        Ok(ValidatorVerdict {
            validator: VALIDATOR_NAME.to_owned(),
            verdict: SubVerdict::NotApplicable,
            primary_reason: FailureReason::PASS,
            sub_checks: Vec::new(),
            metrics: json!({ "reason": reason }),
        })
    }
}
