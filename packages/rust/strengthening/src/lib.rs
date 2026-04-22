//! # eval-ladder-strengthening
//!
//! Level-2 composable validators.
//!
//! Each validator family is a small module that takes a shared
//! [`ValidationContext`] and emits a [`ValidatorVerdict`] with a
//! per-sub-check breakdown. The aggregate L2 verdict is a conjunction:
//! the family pass threshold is `Pass | NotApplicable`, and any `Fail`
//! causes the aggregate to fail with the first failing family's
//! [`eval_ladder_core::FailureReason`] code.
//!
//! The crate ships three real families - augmented unit tests,
//! targeted regression, differential behaviour - and a
//! `NotApplicable`-emitting property-fuzz stub scheduled for Milestone
//! D+.
//!
//! [`L2Extension`] is the entry point: it implements the runner's
//! [`eval_ladder_runner::LevelExtension`] trait so the L2 pipeline
//! plugs into [`eval_ladder_runner::EvaluationPipeline`] as a single
//! configurable step.
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod augmented_tests;
pub mod context;
pub mod differential;
mod exec;
pub mod extension;
pub mod modes;
pub mod property_fuzz;
pub mod regression;
pub mod spec;
pub mod validator;

pub use augmented_tests::AugmentedUnitTests;
pub use context::ValidationContext;
pub use differential::DifferentialBehaviorCheck;
pub use extension::{L2Extension, L2_EXTENSION_NAME, L2_REPORT_FILE, L2_RESULT_FILE};
pub use modes::{StrengtheningMode, StrengtheningModeError};
pub use property_fuzz::PropertyFuzzCheck;
pub use regression::TargetedRegressionCheck;
pub use spec::{
    AugmentedTestSpec, CommandSpec, DifferentialCompare, DifferentialSpec, ObservableSpec,
    PropertyFuzzSpec, RegressionSpec, StrengtheningSpec,
};
pub use validator::{SubCheckResult, SubVerdict, Validator, ValidatorError, ValidatorVerdict};
