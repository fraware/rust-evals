//! Schema validator for normalized `BenchmarkTask` manifests.
//!
//! The `benchmark_task.schema.json` document lives at the workspace root and
//! is the authoritative contract for every adapter's output. This module
//! embeds that document at compile time (via [`include_str!`]) so that
//! ingest is runtime-independent of the caller's current working directory
//! and works identically from the CLI, integration tests, and downstream
//! crates.
//!
//! The embedded schema is compiled once into a [`jsonschema::Validator`]
//! (lazily on first construction of [`BenchmarkTaskValidator`]) and then
//! reused for every task during an ingest run.

use jsonschema::Validator;
use serde::Serialize;
use serde_json::Value;

/// The bundled `BenchmarkTask` JSON schema, sourced from
/// `<workspace>/schemas/benchmark_task.schema.json`.
///
/// The path is relative to this source file: four `..` hops escape
/// `packages/rust/benchmarks/src/` back to the workspace root.
pub const BENCHMARK_TASK_SCHEMA: &str =
    include_str!("../../../../schemas/benchmark_task.schema.json");

/// Wraps a compiled JSON Schema validator for `BenchmarkTask` documents.
pub struct BenchmarkTaskValidator {
    validator: Validator,
}

impl BenchmarkTaskValidator {
    /// Compile the bundled schema.
    ///
    /// This is intentionally not a `const fn`: the `jsonschema` crate needs
    /// to parse the embedded JSON and build a validator, and we want errors
    /// (if the bundled schema is somehow malformed) to surface loudly.
    pub fn new() -> Result<Self, SchemaValidatorError> {
        let value: Value = serde_json::from_str(BENCHMARK_TASK_SCHEMA)
            .map_err(SchemaValidatorError::SchemaParse)?;
        let validator =
            jsonschema::validator_for(&value).map_err(|e| SchemaValidatorError::SchemaCompile {
                message: e.to_string(),
            })?;
        Ok(Self { validator })
    }

    /// Validate a serializable value against the schema.
    ///
    /// Returns `Ok(())` if the value is schema-valid, otherwise returns the
    /// collected set of validation errors as human-readable strings (paths
    /// and messages). Callers should surface these diagnostics without
    /// modification; they are designed to be displayed directly.
    pub fn validate<T: Serialize>(&self, value: &T) -> Result<(), SchemaValidatorError> {
        let as_value = serde_json::to_value(value).map_err(SchemaValidatorError::ValueSerialize)?;
        let errors: Vec<String> = self
            .validator
            .iter_errors(&as_value)
            .map(|e| format!("at {} : {}", e.instance_path, e))
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(SchemaValidatorError::Invalid { errors })
        }
    }
}

impl std::fmt::Debug for BenchmarkTaskValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BenchmarkTaskValidator").finish()
    }
}

/// Errors produced by the schema validator.
#[derive(Debug, thiserror::Error)]
pub enum SchemaValidatorError {
    /// The embedded schema document could not be parsed as JSON.
    #[error("bundled benchmark_task.schema.json is not valid JSON: {0}")]
    SchemaParse(#[source] serde_json::Error),
    /// The embedded schema document did not compile to a validator.
    #[error("bundled benchmark_task.schema.json failed to compile: {message}")]
    SchemaCompile {
        /// Compiler message.
        message: String,
    },
    /// Serializing the candidate value to `serde_json::Value` failed.
    #[error("cannot serialize candidate value for validation: {0}")]
    ValueSerialize(#[source] serde_json::Error),
    /// The candidate value did not validate.
    #[error("schema validation failed:\n{}", .errors.join("\n"))]
    Invalid {
        /// One entry per violation.
        errors: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use eval_ladder_core::{BenchmarkId, BenchmarkLanguage, BenchmarkTask, TaskId};

    fn sample_task() -> BenchmarkTask {
        BenchmarkTask::new(
            BenchmarkId::SweBenchVerified,
            TaskId::new("astropy__astropy-12907").unwrap(),
            "astropy/astropy",
            "12907",
            "Modeling separability_matrix does not compute correctly",
            "Full issue text.",
            "d16bfe05a744909de4b27f5875fe0d4ed41ce607",
            "swebench/sweb.eval.x86_64.astropy__astropy-12907:latest",
            "python -m swebench.harness.run_evaluation --instance_ids astropy__astropy-12907",
            BenchmarkLanguage::Python,
            "https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified",
            Utc.with_ymd_and_hms(2022, 3, 2, 15, 14, 54).unwrap(),
        )
    }

    #[test]
    fn compiles_bundled_schema() {
        BenchmarkTaskValidator::new().expect("bundled schema must compile");
    }

    #[test]
    fn accepts_a_well_formed_task() {
        let v = BenchmarkTaskValidator::new().unwrap();
        v.validate(&sample_task()).expect("valid task");
    }

    #[test]
    fn rejects_a_task_with_bad_commit() {
        let v = BenchmarkTaskValidator::new().unwrap();
        let mut task = sample_task();
        task.base_commit = "not-a-commit".into();
        let err = v.validate(&task).unwrap_err();
        matches!(err, SchemaValidatorError::Invalid { .. });
    }
}
