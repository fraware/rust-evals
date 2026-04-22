//! Task-level strengthening specification.
//!
//! Every L2 validator family is driven by a declarative spec attached to
//! a task. The spec is serializable JSON (or TOML for authoring) so
//! reviewers can read and version-control it without running the code.
//!
//! A single spec drives all four validator families:
//!
//! - [`AugmentedTestSpec`]: a list of commands to run against the
//!   candidate-patched workspace.
//! - [`RegressionSpec`]: a list of commands to run against the
//!   candidate-patched workspace whose failure attributes the drop to
//!   regressions rather than to new behaviour gaps.
//! - [`DifferentialSpec`]: an oracle patch plus a list of observable
//!   commands to run against both the candidate-patched and the
//!   oracle-patched workspace; outputs are canonicalized and compared.
//! - [`PropertyFuzzSpec`]: reserved for Milestone D+. Defined so spec
//!   files can round-trip without loss.
//!
//! Every sub-check inside every family has its own stable `id` so the
//! analysis crate can aggregate "which sub-check caused the L2 drop".

use serde::{Deserialize, Serialize};

use eval_ladder_runner::EnvVar;

/// Full strengthening spec for a single task.
#[allow(clippy::derive_partial_eq_without_eq)] // `env` holds `serde_json::Value` (future-proofing)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrengtheningSpec {
    /// Schema version for forward compatibility.
    #[serde(default = "default_spec_version")]
    pub schema_version: u32,
    /// Augmented unit tests (curated or generated).
    #[serde(default)]
    pub augmented: AugmentedTestSpec,
    /// Targeted regression suite.
    #[serde(default)]
    pub regression: RegressionSpec,
    /// Differential-behaviour spec. `None` means differential is not
    /// applicable for this task.
    #[serde(default)]
    pub differential: Option<DifferentialSpec>,
    /// Property/fuzz spec. `None` means property fuzz is not applicable
    /// for this task.
    #[serde(default)]
    pub property_fuzz: Option<PropertyFuzzSpec>,
}

const fn default_spec_version() -> u32 {
    1
}

/// Augmented unit tests family.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentedTestSpec {
    /// Commands to run in the candidate-patched workspace. All must
    /// succeed for the family to pass.
    #[serde(default)]
    pub commands: Vec<CommandSpec>,
    /// Whether flaky tests (defined per `CommandSpec::flaky`) should be
    /// retried. Default `false`; flaky tests are a failure mode, not a
    /// feature.
    #[serde(default)]
    pub retry_flaky: bool,
}

/// Targeted regression family.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegressionSpec {
    /// Commands to run in the candidate-patched workspace. All must
    /// succeed for the family to pass.
    #[serde(default)]
    pub commands: Vec<CommandSpec>,
}

/// Differential-behaviour family.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DifferentialSpec {
    /// Stable reference to the oracle patch. The actual bytes are
    /// supplied at run time through
    /// [`crate::extension::L2Extension::with_oracle_patch`] so the spec
    /// itself is safe to publish.
    pub oracle_patch_ref: String,
    /// Observables to compare.
    #[serde(default)]
    pub observables: Vec<ObservableSpec>,
}

/// Property-fuzz family. Stub for Milestone D+.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyFuzzSpec {
    /// Property name for reporting.
    pub name: String,
}

/// Single runnable command inside a validator family.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    /// Stable per-spec identifier, e.g. `"aug_edge_cases"`.
    pub id: String,
    /// Program plus arguments. Never passed to a shell; the first entry
    /// is the program name.
    pub command: Vec<String>,
    /// Extra environment variables layered on top of the pipeline's
    /// global env.
    #[serde(default)]
    pub env: Vec<EnvVar>,
    /// Optional subdirectory (relative to the workspace root) to use
    /// as the working directory for this command. Defaults to the
    /// workspace root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    /// Expected exit code. `None` means `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_exit_code: Option<i32>,
    /// Marks the command as known-flaky. Only honoured if the owning
    /// spec allows retries.
    #[serde(default)]
    pub flaky: bool,
}

/// Observable command for differential comparison.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservableSpec {
    /// Stable per-spec identifier, e.g. `"parse_golden_input_1"`.
    pub id: String,
    /// Command to run in *both* the candidate-patched and the
    /// oracle-patched workspace.
    pub command: Vec<String>,
    /// Extra environment variables.
    #[serde(default)]
    pub env: Vec<EnvVar>,
    /// Optional subdirectory (relative to the workspace root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    /// What to compare between the two runs.
    #[serde(default)]
    pub compare: DifferentialCompare,
    /// Whether to trim trailing whitespace before comparison (useful on
    /// Windows CI where CRLF leaks in).
    #[serde(default)]
    pub normalize_trailing_whitespace: bool,
}

/// Which stream(s) the differential observer compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DifferentialCompare {
    /// Compare stdout only (default).
    #[default]
    Stdout,
    /// Compare stderr only.
    Stderr,
    /// Compare both streams plus exit code.
    Full,
    /// Compare exit code only.
    ExitCode,
}

impl StrengtheningSpec {
    /// Parse a spec from JSON bytes.
    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Serialize to canonical JSON for storage in the bundle.
    pub fn to_canonical_json(&self) -> Result<Vec<u8>, eval_ladder_core::CoreError> {
        eval_ladder_core::canonical_json(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_minimal_spec() {
        let s = StrengtheningSpec::default();
        let bytes = s.to_canonical_json().unwrap();
        let back = StrengtheningSpec::from_json(&bytes).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn differential_default_compare_is_stdout() {
        let obs: ObservableSpec =
            serde_json::from_str(r#"{"id":"o1","command":["echo","hi"]}"#).unwrap();
        assert_eq!(obs.compare, DifferentialCompare::Stdout);
        assert!(!obs.normalize_trailing_whitespace);
    }

    #[test]
    fn spec_rejects_unknown_fields() {
        let err = serde_json::from_str::<StrengtheningSpec>(r#"{"schema_version":1,"unknown":1}"#)
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown field"), "got: {msg}");
    }
}
