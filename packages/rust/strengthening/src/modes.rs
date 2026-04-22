//! Strengthening mode selector.
//!
//! Maps the four named modes (`tests_only`, `tests_plus_diff`,
//! `tests_plus_regression`, `full_l2`) to the set of sub-validators that must
//! run. Each mode is additive so analysis can attribute a score drop to a
//! specific validator family.

use eval_ladder_runner::StrengtheningRules;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Named strengthening modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrengtheningMode {
    /// Augmented unit tests only.
    TestsOnly,
    /// Augmented unit tests plus differential behaviour check.
    TestsPlusDiff,
    /// Augmented unit tests plus targeted regression check.
    TestsPlusRegression,
    /// Full L2 (all validators).
    FullL2,
}

/// Errors returned when parsing or translating modes.
#[derive(Debug, Error)]
pub enum StrengtheningModeError {
    /// Unknown mode name.
    #[error("unknown strengthening mode: {0:?}")]
    Unknown(String),
}

impl StrengtheningMode {
    /// Stable short code used in config files.
    #[must_use]
    pub const fn short_code(self) -> &'static str {
        match self {
            Self::TestsOnly => "tests_only",
            Self::TestsPlusDiff => "tests_plus_diff",
            Self::TestsPlusRegression => "tests_plus_regression",
            Self::FullL2 => "full_l2",
        }
    }

    /// Translate a mode into runner-layer [`StrengtheningRules`].
    #[must_use]
    pub fn rules(self) -> StrengtheningRules {
        match self {
            Self::TestsOnly => StrengtheningRules {
                mode: self.short_code().to_owned(),
                run_augmented_unit_tests: true,
                run_differential_behavior: false,
                run_targeted_regression: false,
                run_property_fuzz: false,
            },
            Self::TestsPlusDiff => StrengtheningRules {
                mode: self.short_code().to_owned(),
                run_augmented_unit_tests: true,
                run_differential_behavior: true,
                run_targeted_regression: false,
                run_property_fuzz: false,
            },
            Self::TestsPlusRegression => StrengtheningRules {
                mode: self.short_code().to_owned(),
                run_augmented_unit_tests: true,
                run_differential_behavior: false,
                run_targeted_regression: true,
                run_property_fuzz: false,
            },
            Self::FullL2 => StrengtheningRules {
                mode: self.short_code().to_owned(),
                run_augmented_unit_tests: true,
                run_differential_behavior: true,
                run_targeted_regression: true,
                run_property_fuzz: true,
            },
        }
    }
}

impl std::str::FromStr for StrengtheningMode {
    type Err = StrengtheningModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tests_only" => Ok(Self::TestsOnly),
            "tests_plus_diff" => Ok(Self::TestsPlusDiff),
            "tests_plus_regression" => Ok(Self::TestsPlusRegression),
            "full_l2" => Ok(Self::FullL2),
            other => Err(StrengtheningModeError::Unknown(other.to_owned())),
        }
    }
}
