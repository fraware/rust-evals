//! Schema and evaluator versioning.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// Current schema version shipped by this build of `eval-ladder-core`.
///
/// Every persisted artifact carries its own `schema_version` field and the
/// evaluator refuses to load artifacts whose version it does not recognize.
/// Bumping this constant requires a changelog entry and coordinated updates
/// to the JSON schemas under `schemas/`.
pub const SCHEMA_VERSION: SchemaVersion = SchemaVersion(1);

/// Evaluator version string embedded in every `RunManifest` and
/// `EvaluationResult`. Populated at compile time from the workspace `Cargo.toml`.
///
/// This is a `const` because it is shared by every evaluator decision in a
/// given build. The inner representation is `Cow<'static, str>` so that
/// round-tripped deserialized values (which own their string) share a type
/// with the compile-time constant.
pub const EVALUATOR_VERSION: EvaluatorVersion =
    EvaluatorVersion(Cow::Borrowed(env!("CARGO_PKG_VERSION")));

/// A monotonically increasing integer schema version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SchemaVersion(pub u32);

impl SchemaVersion {
    /// Returns the inner integer.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Evaluator version string. A newtype so it cannot be confused with other
/// version identifiers (for example, container image digests).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EvaluatorVersion(pub Cow<'static, str>);

impl EvaluatorVersion {
    /// Returns the inner string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EvaluatorVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
