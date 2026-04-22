//! `ProofObligation` data model.
//!
//! Mirrors `schemas/proof_obligation.schema.json` exactly: unknown keys
//! are rejected, required fields are enforced by the type system, and
//! the wire format is stable across versions (`schema_version = 1`).
//!
//! Governance sits in `docs/proof_subset_policy.md`. That document is
//! the source of truth for *which* tasks may carry an obligation and
//! *what* shape each obligation must take. This module only checks
//! that the JSON loads cleanly.

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable schema version for [`ProofObligation`] JSONL entries.
pub const PROOF_OBLIGATION_SCHEMA_VERSION: u32 = 1;

/// One of the five preferred categories declared in
/// `docs/proof_subset_policy.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    /// Parser/serializer consistency (roundtrip invariants).
    ParserSerializerConsistency,
    /// State-machine safety (no invalid transitions).
    StateMachineSafety,
    /// Preservation invariant on a transformation.
    PreservationInvariant,
    /// Numeric bound or monotonicity.
    NumericBoundOrMonotonicity,
    /// No-panic or no-invalid-state on selected code paths.
    NoPanicOrInvalidState,
}

/// Declared reviewer effort (bounded effort is a rubric requirement).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Difficulty {
    /// Reviewer hours declared at selection time.
    pub reviewer_hours: f64,
}

/// Checklist mirrored from the five selection-rubric items.
///
/// All five must be `true` for the obligation to be eligible per
/// `docs/proof_subset_policy.md`. The loader does not enforce this
/// (humans declare their own eligibility); a CI check can run
/// [`ProofObligation::is_selection_rubric_satisfied`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionRationale {
    /// Property statable in one or two sentences.
    pub one_or_two_sentence_property: bool,
    /// Local scope (no large theorem libraries required).
    pub local_scope: bool,
    /// Property matters to the issue itself.
    pub matters_to_issue: bool,
    /// Property is strictly stronger than the official tests.
    pub strictly_stronger_than_tests: bool,
    /// Formalization effort is bounded.
    pub bounded_effort: bool,
}

/// Fixture / witness input referenced by the obligation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessInput {
    /// Stable short name used by the checker.
    pub name: String,
    /// Path (bundle-relative or repo-relative per the obligation's
    /// conventions) to the witness file.
    pub path: String,
    /// Optional `sha256:<64-hex>` content pin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// The Lean checker invocation declared by the obligation.
///
/// The runner executes `command` with `args`, cwd set by the
/// [`crate::extension::L4Extension`] to the Lean project root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObligationProofChecker {
    /// Checker binary (for example `lake`).
    pub command: String,
    /// Positional arguments passed to `command`.
    pub args: Vec<String>,
}

/// A curated semantic obligation attached to a benchmark task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofObligation {
    /// Schema version. Always `1` for the current layout.
    pub schema_version: u32,
    /// Stable obligation identifier.
    pub obligation_id: String,
    /// Benchmark-local task identifier the obligation attaches to.
    pub task_id: String,
    /// Short human-readable property name.
    pub property_name: String,
    /// One of the preferred categories.
    pub property_type: PropertyType,
    /// Files the obligation constrains.
    pub target_files: Vec<String>,
    /// One- or two-sentence informal statement (<= 600 chars per the
    /// schema).
    pub informal_statement: String,
    /// Path to the formal Lean declaration under
    /// `packages/lean/EvalLadder/Obligations/`.
    pub formal_statement_ref: String,
    /// Checker invocation.
    pub proof_checker: ObligationProofChecker,
    /// Stable uppercase code the checker must return to count as pass.
    pub pass_criterion: String,
    /// Declared difficulty.
    pub difficulty: Difficulty,
    /// Selection-rubric checklist.
    pub selection_rationale: SelectionRationale,
    /// Optional witnesses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub witness_inputs: Vec<WitnessInput>,
    /// Optional symbol-name hints the patch is expected to touch.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_touched_symbols: Vec<String>,
}

impl ProofObligation {
    /// True iff every `SelectionRationale` checkbox is set. Use in
    /// selection-bias audits.
    #[must_use]
    pub const fn is_selection_rubric_satisfied(&self) -> bool {
        let r = &self.selection_rationale;
        r.one_or_two_sentence_property
            && r.local_scope
            && r.matters_to_issue
            && r.strictly_stronger_than_tests
            && r.bounded_effort
    }

    /// Load a single obligation from a JSON file.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ProofObligationLoadError> {
        let bytes =
            std::fs::read(path.as_ref()).map_err(|source| ProofObligationLoadError::Io {
                path: path.as_ref().display().to_string(),
                source,
            })?;
        Self::from_slice(&bytes)
    }

    /// Parse a single obligation from bytes.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ProofObligationLoadError> {
        let obl: Self = serde_json::from_slice(bytes).map_err(ProofObligationLoadError::Parse)?;
        obl.validate()?;
        Ok(obl)
    }

    /// Minimal structural validation (schema version + pass code
    /// shape). The JSON schema does most of the heavy lifting; this
    /// is a belt-and-braces check in case a handwritten loader skips
    /// the schema pass.
    pub fn validate(&self) -> Result<(), ProofObligationLoadError> {
        if self.schema_version != PROOF_OBLIGATION_SCHEMA_VERSION {
            return Err(ProofObligationLoadError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.proof_checker.command.trim().is_empty() {
            return Err(ProofObligationLoadError::Invalid(
                "proof_checker.command must not be empty".to_owned(),
            ));
        }
        if !is_stable_code(&self.pass_criterion) {
            return Err(ProofObligationLoadError::Invalid(format!(
                "pass_criterion {:?} must match ^[A-Z][A-Z0-9_]*$",
                self.pass_criterion
            )));
        }
        if self.target_files.is_empty() {
            return Err(ProofObligationLoadError::Invalid(
                "target_files must be non-empty".to_owned(),
            ));
        }
        Ok(())
    }
}

fn is_stable_code(s: &str) -> bool {
    let mut chars = s.chars();
    let first = chars.next();
    let Some(c) = first else { return false };
    if !c.is_ascii_uppercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Errors produced by the obligation loader.
#[derive(Debug, Error)]
pub enum ProofObligationLoadError {
    /// File system error while reading the obligation file.
    #[error("obligation io ({path}): {source}")]
    Io {
        /// Path the loader tried to read.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// JSON parse error.
    #[error("obligation parse: {0}")]
    Parse(#[from] serde_json::Error),
    /// Schema version the loader does not recognize.
    #[error("obligation unsupported schema_version: {0}")]
    UnsupportedSchemaVersion(u32),
    /// Structural validation failure.
    #[error("obligation invalid: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ProofObligation {
        ProofObligation {
            schema_version: 1,
            obligation_id: "obl.fixture".into(),
            task_id: "fixture__milestone-f".into(),
            property_name: "trivial".into(),
            property_type: PropertyType::NoPanicOrInvalidState,
            target_files: vec!["src/lib.rs".into()],
            informal_statement: "The fixture is always true.".into(),
            formal_statement_ref: "EvalLadder/Obligations/Fixtures/MilestoneF.lean".into(),
            proof_checker: ObligationProofChecker {
                command: "lake".into(),
                args: vec!["env".into(), "lean".into()],
            },
            pass_criterion: "L4_OBLIGATION_MET".into(),
            difficulty: Difficulty {
                reviewer_hours: 0.5,
            },
            selection_rationale: SelectionRationale {
                one_or_two_sentence_property: true,
                local_scope: true,
                matters_to_issue: true,
                strictly_stronger_than_tests: true,
                bounded_effort: true,
            },
            witness_inputs: Vec::new(),
            expected_touched_symbols: Vec::new(),
        }
    }

    #[test]
    fn round_trip_minimal_obligation() {
        let bytes = serde_json::to_vec(&sample()).unwrap();
        let back = ProofObligation::from_slice(&bytes).unwrap();
        assert_eq!(back, sample());
    }

    #[test]
    fn rejects_unknown_fields() {
        let json = r#"{
            "schema_version": 1,
            "obligation_id": "x",
            "task_id": "t",
            "property_name": "p",
            "property_type": "no_panic_or_invalid_state",
            "target_files": ["a"],
            "informal_statement": "s",
            "formal_statement_ref": "r",
            "proof_checker": {"command": "lake", "args": []},
            "pass_criterion": "L4_OK",
            "difficulty": {"reviewer_hours": 1.0},
            "selection_rationale": {
                "one_or_two_sentence_property": true,
                "local_scope": true,
                "matters_to_issue": true,
                "strictly_stronger_than_tests": true,
                "bounded_effort": true
            },
            "unknown": 42
        }"#;
        assert!(matches!(
            ProofObligation::from_slice(json.as_bytes()),
            Err(ProofObligationLoadError::Parse(_))
        ));
    }

    #[test]
    fn rejects_lowercase_pass_code() {
        let mut o = sample();
        o.pass_criterion = "l4_not_upper".into();
        let err = o.validate().unwrap_err();
        assert!(matches!(err, ProofObligationLoadError::Invalid(_)));
    }

    #[test]
    fn rejects_empty_command() {
        let mut o = sample();
        o.proof_checker.command = String::new();
        assert!(matches!(
            o.validate(),
            Err(ProofObligationLoadError::Invalid(_))
        ));
    }

    #[test]
    fn selection_rubric_flag_helper() {
        let mut o = sample();
        assert!(o.is_selection_rubric_satisfied());
        o.selection_rationale.local_scope = false;
        assert!(!o.is_selection_rubric_satisfied());
    }
}
