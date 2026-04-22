//! Core error types.
//!
//! Kept small on purpose: most crates wrap [`CoreError`] inside their own
//! error enum rather than re-exporting it.

use thiserror::Error;

/// Errors produced by `eval-ladder-core`.
#[derive(Debug, Error)]
pub enum CoreError {
    /// The string did not parse as the named identifier kind.
    #[error("invalid {kind}: {value:?}")]
    InvalidId {
        /// Name of the identifier type.
        kind: &'static str,
        /// The rejected input.
        value: String,
    },

    /// The benchmark discriminator was not recognized.
    #[error("unknown benchmark id: {0:?}")]
    UnknownBenchmark(String),

    /// Canonical JSON serialization failed. This is unusual and indicates a
    /// logic bug (for example, a `NaN` float).
    #[error("canonical json error: {0}")]
    CanonicalJson(#[from] serde_json::Error),

    /// A persisted artifact declared a schema version that this build does
    /// not understand.
    #[error("unsupported schema_version {actual}; this build expects {expected}")]
    UnsupportedSchemaVersion {
        /// The version present in the input.
        actual: u32,
        /// The version this build understands.
        expected: u32,
    },
}
