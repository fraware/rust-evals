//! Stable failure, policy-violation, and taxonomy codes.
//!
//! Every pass/fail verdict emitted by the evaluator carries a stable machine
//! code from this module. Adding or renaming a code requires a changelog
//! entry and typically a schema version bump.
//!
//! The code strings match `docs/evaluation_ladder.md` exactly.

use serde::{Deserialize, Serialize};

/// Level-specific failure reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
#[allow(missing_docs)] // Each variant is documented below via `as_str`.
pub enum FailureReason {
    PASS,

    // --- L0 ---
    L0_OFFICIAL_FAIL,
    L0_OFFICIAL_INVALID,
    L0_OFFICIAL_TIMEOUT,
    L0_OFFICIAL_MISSING_ARTIFACT,

    // --- L1 ---
    L1_ENV_FINGERPRINT_MISMATCH,
    L1_PATCH_APPLY_FAILED,
    L1_RERUN_DISAGREEMENT,
    L1_PARSER_AMBIGUOUS,
    L1_TIMEOUT,
    L1_HARNESS_ERROR,

    // --- L2 ---
    L2_AUG_TESTS_FAIL,
    L2_DIFF_BEHAVIOR,
    L2_REGRESSION_FAIL,
    L2_PROPERTY_VIOLATED,
    L2_ORACLE_UNAVAILABLE,

    // --- L4 ---
    L4_OBLIGATION_UNMET,
    L4_PROOF_CHECK_FAILED,
    L4_OBLIGATION_NOT_APPLICABLE,
    L4_EXTRACTION_FAILED,
}

impl FailureReason {
    /// Stable uppercase code used in JSON output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PASS => "PASS",
            Self::L0_OFFICIAL_FAIL => "L0_OFFICIAL_FAIL",
            Self::L0_OFFICIAL_INVALID => "L0_OFFICIAL_INVALID",
            Self::L0_OFFICIAL_TIMEOUT => "L0_OFFICIAL_TIMEOUT",
            Self::L0_OFFICIAL_MISSING_ARTIFACT => "L0_OFFICIAL_MISSING_ARTIFACT",
            Self::L1_ENV_FINGERPRINT_MISMATCH => "L1_ENV_FINGERPRINT_MISMATCH",
            Self::L1_PATCH_APPLY_FAILED => "L1_PATCH_APPLY_FAILED",
            Self::L1_RERUN_DISAGREEMENT => "L1_RERUN_DISAGREEMENT",
            Self::L1_PARSER_AMBIGUOUS => "L1_PARSER_AMBIGUOUS",
            Self::L1_TIMEOUT => "L1_TIMEOUT",
            Self::L1_HARNESS_ERROR => "L1_HARNESS_ERROR",
            Self::L2_AUG_TESTS_FAIL => "L2_AUG_TESTS_FAIL",
            Self::L2_DIFF_BEHAVIOR => "L2_DIFF_BEHAVIOR",
            Self::L2_REGRESSION_FAIL => "L2_REGRESSION_FAIL",
            Self::L2_PROPERTY_VIOLATED => "L2_PROPERTY_VIOLATED",
            Self::L2_ORACLE_UNAVAILABLE => "L2_ORACLE_UNAVAILABLE",
            Self::L4_OBLIGATION_UNMET => "L4_OBLIGATION_UNMET",
            Self::L4_PROOF_CHECK_FAILED => "L4_PROOF_CHECK_FAILED",
            Self::L4_OBLIGATION_NOT_APPLICABLE => "L4_OBLIGATION_NOT_APPLICABLE",
            Self::L4_EXTRACTION_FAILED => "L4_EXTRACTION_FAILED",
        }
    }
}

impl std::fmt::Display for FailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// L3 policy violation codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
#[allow(missing_docs)]
pub enum PolicyViolation {
    PV_NET_ACCESS,
    PV_FORBIDDEN_CMD,
    PV_EDIT_SCOPE,
    PV_FILE_COUNT_EXCEEDED,
    PV_DEPENDENCY_EDIT,
    PV_GENERATED_TEST_DISALLOWED,
    PV_ENV_NONDETERMINISTIC,
    PV_TRACE_INCOMPLETE,
    PV_BINARY_DISALLOWED,
}

impl PolicyViolation {
    /// Stable uppercase code used in JSON output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PV_NET_ACCESS => "PV_NET_ACCESS",
            Self::PV_FORBIDDEN_CMD => "PV_FORBIDDEN_CMD",
            Self::PV_EDIT_SCOPE => "PV_EDIT_SCOPE",
            Self::PV_FILE_COUNT_EXCEEDED => "PV_FILE_COUNT_EXCEEDED",
            Self::PV_DEPENDENCY_EDIT => "PV_DEPENDENCY_EDIT",
            Self::PV_GENERATED_TEST_DISALLOWED => "PV_GENERATED_TEST_DISALLOWED",
            Self::PV_ENV_NONDETERMINISTIC => "PV_ENV_NONDETERMINISTIC",
            Self::PV_TRACE_INCOMPLETE => "PV_TRACE_INCOMPLETE",
            Self::PV_BINARY_DISALLOWED => "PV_BINARY_DISALLOWED",
        }
    }
}

impl std::fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Analysis taxonomy codes aggregated in the false-success taxonomy output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
#[allow(missing_docs)]
pub enum TaxonomyCode {
    TX_RERUN_INSTABILITY,
    TX_WEAK_TEST_EXPOSED,
    TX_DIFF_BEHAVIOR,
    TX_POLICY_INVALID,
    TX_SEMANTIC_OBLIGATION_FAIL,
}

impl TaxonomyCode {
    /// Stable uppercase code used in JSON output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TX_RERUN_INSTABILITY => "TX_RERUN_INSTABILITY",
            Self::TX_WEAK_TEST_EXPOSED => "TX_WEAK_TEST_EXPOSED",
            Self::TX_DIFF_BEHAVIOR => "TX_DIFF_BEHAVIOR",
            Self::TX_POLICY_INVALID => "TX_POLICY_INVALID",
            Self::TX_SEMANTIC_OBLIGATION_FAIL => "TX_SEMANTIC_OBLIGATION_FAIL",
        }
    }
}

impl std::fmt::Display for TaxonomyCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_codes_roundtrip_through_json() {
        let r = FailureReason::L2_DIFF_BEHAVIOR;
        let s = serde_json::to_string(&r).unwrap();
        assert_eq!(s, "\"L2_DIFF_BEHAVIOR\"");
        let back: FailureReason = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn policy_codes_roundtrip_through_json() {
        let v = PolicyViolation::PV_NET_ACCESS;
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "\"PV_NET_ACCESS\"");
        let back: PolicyViolation = serde_json::from_str(&s).unwrap();
        assert_eq!(v, back);
    }
}
