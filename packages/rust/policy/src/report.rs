//! Structured policy reports embedded in evidence bundles.

use eval_ladder_core::{PolicyViolation, SchemaVersion, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};

/// A single policy finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyFinding {
    /// Stable `PV_*` violation code.
    pub code: PolicyViolation,
    /// Human-readable message. Free-form; not a stable contract.
    pub message: String,
    /// Optional evidence pointer (path relative to the bundle root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_path: Option<String>,
}

/// Policy evaluation report. Stored as `policy_results.json` in the bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyReport {
    /// Schema version.
    pub schema_version: SchemaVersion,
    /// Policy name (from the loaded policy spec).
    pub policy_name: String,
    /// All findings, in evaluation order.
    pub findings: Vec<PolicyFinding>,
    /// Convenience flag: `true` iff `findings` is empty.
    pub conformant: bool,
}

impl PolicyReport {
    /// Build a report from findings.
    #[must_use]
    pub fn new(policy_name: impl Into<String>, findings: Vec<PolicyFinding>) -> Self {
        let conformant = findings.is_empty();
        Self {
            schema_version: SCHEMA_VERSION,
            policy_name: policy_name.into(),
            findings,
            conformant,
        }
    }
}
