//! Evaluation level and status enums.
//!
//! The ladder has five levels; see `docs/evaluation_ladder.md` for their
//! full semantics. Every evaluator produces one `EvaluationResult` per
//! level it was asked to evaluate, with an explicit [`EvaluationStatus`].

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Evaluation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EvaluationLevel {
    /// Passes official benchmark validation.
    #[serde(rename = "L0")]
    L0Official,
    /// Passes deterministic rerun in our harness.
    #[serde(rename = "L1")]
    L1TrustedRerun,
    /// Passes strengthened tests / differential / regression / fuzz.
    #[serde(rename = "L2")]
    L2Strengthened,
    /// Success achieved through a valid process (command/edit/network policy).
    #[serde(rename = "L3")]
    L3PolicyConformant,
    /// Satisfies a machine-checkable semantic obligation on the curated subset.
    #[serde(rename = "L4")]
    L4Semantic,
}

impl EvaluationLevel {
    /// All levels, in ladder order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::L0Official,
            Self::L1TrustedRerun,
            Self::L2Strengthened,
            Self::L3PolicyConformant,
            Self::L4Semantic,
        ]
    }

    /// Stable short code (`"L0"`, ..., `"L4"`) used in schemas and paths.
    #[must_use]
    pub const fn short_code(self) -> &'static str {
        match self {
            Self::L0Official => "L0",
            Self::L1TrustedRerun => "L1",
            Self::L2Strengthened => "L2",
            Self::L3PolicyConformant => "L3",
            Self::L4Semantic => "L4",
        }
    }

    /// Human-readable long name.
    #[must_use]
    pub const fn long_name(self) -> &'static str {
        match self {
            Self::L0Official => "Official",
            Self::L1TrustedRerun => "Trusted rerun",
            Self::L2Strengthened => "Strengthened",
            Self::L3PolicyConformant => "Policy-conformant",
            Self::L4Semantic => "Semantic",
        }
    }
}

impl fmt::Display for EvaluationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.short_code())
    }
}

impl std::str::FromStr for EvaluationLevel {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Case-insensitive for ergonomic CLI use; the canonical form is
        // still upper-case ASCII.
        match s.trim().to_ascii_uppercase().as_str() {
            "L0" => Ok(Self::L0Official),
            "L1" => Ok(Self::L1TrustedRerun),
            "L2" => Ok(Self::L2Strengthened),
            "L3" => Ok(Self::L3PolicyConformant),
            "L4" => Ok(Self::L4Semantic),
            _ => Err(CoreError::InvalidId {
                kind: "EvaluationLevel",
                value: s.to_owned(),
            }),
        }
    }
}

/// Outcome status for a single level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationStatus {
    /// Level checks succeeded.
    Pass,
    /// Level checks failed with a failure reason.
    Fail,
    /// The evaluator could not produce a verdict (for example, the harness
    /// itself errored). Never silently coerced to `Fail`.
    Invalid,
    /// The level does not apply to this task (for example, L4 on a task
    /// outside the curated proof subset).
    NotApplicable,
}

impl EvaluationStatus {
    /// Returns `true` if the status is a definitive pass.
    #[must_use]
    pub const fn is_pass(self) -> bool {
        matches!(self, Self::Pass)
    }

    /// Returns `true` if the status is a definitive fail.
    #[must_use]
    pub const fn is_fail(self) -> bool {
        matches!(self, Self::Fail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_levels_have_unique_short_codes() {
        let codes: std::collections::HashSet<_> = EvaluationLevel::all()
            .iter()
            .map(|l| l.short_code())
            .collect();
        assert_eq!(codes.len(), EvaluationLevel::all().len());
    }

    #[test]
    fn level_parses_case_insensitively() {
        assert_eq!(
            "l0".parse::<EvaluationLevel>().unwrap(),
            EvaluationLevel::L0Official
        );
        assert_eq!(
            "L3".parse::<EvaluationLevel>().unwrap(),
            EvaluationLevel::L3PolicyConformant
        );
        assert!("L9".parse::<EvaluationLevel>().is_err());
    }
}
