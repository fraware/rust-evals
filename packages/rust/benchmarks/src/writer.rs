//! Deterministic manifest writer.
//!
//! Every adapter funnels its normalized [`BenchmarkTask`] through
//! [`ManifestWriter`] so that:
//!
//! 1. the manifest is validated against `benchmark_task.schema.json`
//!    before it hits disk,
//! 2. the on-disk bytes are canonical (object keys sorted, Unix newlines,
//!    one trailing newline),
//! 3. writes are atomic (tempfile + rename), so a crash during ingest
//!    cannot leave truncated manifests behind, and
//! 4. re-ingesting the same upstream data produces byte-identical files.

use std::io::Write;
use std::path::{Path, PathBuf};

use eval_ladder_core::{canonical_json, BenchmarkTask, CoreError};
use thiserror::Error;

use crate::schema::{BenchmarkTaskValidator, SchemaValidatorError};

/// Errors produced by the manifest writer.
#[derive(Debug, Error)]
pub enum ManifestWriteError {
    /// I/O failure creating the output directory or persisting the manifest.
    #[error("io: {path}: {source}")]
    Io {
        /// Path being operated on.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Canonical JSON serialization failed.
    #[error("canonical serialization failed: {0}")]
    Canonical(#[from] CoreError),
    /// Schema validation failed for the candidate manifest.
    #[error("schema validation failed for task {task_id}: {source}")]
    Schema {
        /// Task id of the failing manifest.
        task_id: String,
        /// Underlying validator error.
        #[source]
        source: SchemaValidatorError,
    },
}

/// Writes normalized `BenchmarkTask` manifests to an output directory.
#[derive(Debug)]
pub struct ManifestWriter {
    out_dir: PathBuf,
    validator: BenchmarkTaskValidator,
}

impl ManifestWriter {
    /// Create a writer that writes into `out_dir`.
    ///
    /// The directory is created (recursively) if it does not already exist.
    pub fn new(out_dir: impl Into<PathBuf>) -> Result<Self, ManifestWriteError> {
        let out_dir = out_dir.into();
        std::fs::create_dir_all(&out_dir).map_err(|source| ManifestWriteError::Io {
            path: out_dir.clone(),
            source,
        })?;
        let validator =
            BenchmarkTaskValidator::new().map_err(|source| ManifestWriteError::Schema {
                task_id: "<validator-init>".to_string(),
                source,
            })?;
        Ok(Self { out_dir, validator })
    }

    /// Output directory.
    #[must_use]
    pub fn out_dir(&self) -> &Path {
        &self.out_dir
    }

    /// Validate and write a single manifest atomically.
    ///
    /// The on-disk filename is `<task_id>.json`, where `task_id` is taken
    /// from [`BenchmarkTask::task_id`] after sanitizing path-unsafe
    /// characters (forward slash and backslash) to `_`.
    ///
    /// Returns the absolute path of the written manifest.
    pub fn write(&self, task: &BenchmarkTask) -> Result<PathBuf, ManifestWriteError> {
        self.validator
            .validate(task)
            .map_err(|source| ManifestWriteError::Schema {
                task_id: task.task_id.to_string(),
                source,
            })?;

        let mut bytes = canonical_json(task)?;
        bytes.push(b'\n');

        let filename = format!("{}.json", sanitize_task_id(task.task_id.as_str()));
        let final_path = self.out_dir.join(&filename);
        let tmp_path = self.out_dir.join(format!(".{filename}.tmp"));

        {
            let mut f =
                std::fs::File::create(&tmp_path).map_err(|source| ManifestWriteError::Io {
                    path: tmp_path.clone(),
                    source,
                })?;
            f.write_all(&bytes)
                .map_err(|source| ManifestWriteError::Io {
                    path: tmp_path.clone(),
                    source,
                })?;
            f.sync_all().map_err(|source| ManifestWriteError::Io {
                path: tmp_path.clone(),
                source,
            })?;
        }
        // On Windows rename onto an existing file fails; remove first if present.
        if final_path.exists() {
            std::fs::remove_file(&final_path).map_err(|source| ManifestWriteError::Io {
                path: final_path.clone(),
                source,
            })?;
        }
        std::fs::rename(&tmp_path, &final_path).map_err(|source| ManifestWriteError::Io {
            path: final_path.clone(),
            source,
        })?;
        Ok(final_path)
    }
}

fn sanitize_task_id(id: &str) -> String {
    id.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use eval_ladder_core::{BenchmarkId, BenchmarkLanguage, TaskId};
    use tempfile::TempDir;

    fn task(id: &str) -> BenchmarkTask {
        BenchmarkTask::new(
            BenchmarkId::SweBenchVerified,
            TaskId::new(id).unwrap(),
            "astropy/astropy",
            "1",
            "title",
            "text",
            "deadbeefcafebabe",
            "img:tag",
            "python -m swebench.harness.run_evaluation",
            BenchmarkLanguage::Python,
            "https://example.com/dataset",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
        )
    }

    #[test]
    fn writes_canonical_json_with_trailing_newline() {
        let tmp = TempDir::new().unwrap();
        let w = ManifestWriter::new(tmp.path()).unwrap();
        let p = w.write(&task("astropy__astropy-1")).unwrap();
        let body = std::fs::read(&p).unwrap();
        assert_eq!(body.last(), Some(&b'\n'));
        let text = std::str::from_utf8(&body).unwrap().trim_end_matches('\n');
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["task_id"].as_str(), Some("astropy__astropy-1"));
    }

    #[test]
    fn same_input_produces_identical_bytes() {
        let tmp = TempDir::new().unwrap();
        let w = ManifestWriter::new(tmp.path()).unwrap();
        let p1 = w.write(&task("x-1")).unwrap();
        let b1 = std::fs::read(&p1).unwrap();
        let p2 = w.write(&task("x-1")).unwrap();
        let b2 = std::fs::read(&p2).unwrap();
        assert_eq!(b1, b2);
        assert_eq!(p1, p2);
    }

    #[test]
    fn rejects_schema_invalid_task() {
        let tmp = TempDir::new().unwrap();
        let w = ManifestWriter::new(tmp.path()).unwrap();
        let mut t = task("x-1");
        t.base_commit = "zzz".into();
        let err = w.write(&t).unwrap_err();
        assert!(matches!(err, ManifestWriteError::Schema { .. }));
    }

    #[test]
    fn sanitizes_unsafe_filename_chars() {
        assert_eq!(sanitize_task_id("a/b:c*d"), "a_b_c_d");
    }
}
