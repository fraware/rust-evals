//! Raw record types for SWE-bench-family JSONL inputs.
//!
//! Upstream benchmark releases (Verified, Live, Rust-SWE-bench) publish
//! per-task records as JSONL with a shared core of fields and
//! benchmark-specific extensions. This module defines one permissive
//! record type that captures the shared core and carries unknown fields
//! in [`RawSweBenchRecord::extra`] for per-adapter inspection.
//!
//! Permissiveness is deliberate: benchmarks gain new columns over time
//! and we do not want ingest to break the moment the upstream team
//! renames a field. Adapters are still strict about the fields they
//! depend on; they simply do not care about the ones they don't.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Shared raw SWE-bench-style task record.
///
/// Field names match the upstream datasets (`princeton-nlp/SWE-bench_Verified`,
/// `microsoft/SWE-bench-Live`, and the Rust-SWE-bench release). The struct
/// is deliberately tolerant of unknown fields so that upstream additions
/// do not break ingest.
#[derive(Debug, Clone, Deserialize)]
pub struct RawSweBenchRecord {
    /// Benchmark-local task identifier (`<owner>__<repo>-<number>` for
    /// SWE-bench-family datasets).
    pub instance_id: String,
    /// `<owner>/<name>`.
    pub repo: String,
    /// Git SHA the task is pinned to.
    pub base_commit: String,
    /// Issue body / problem statement.
    pub problem_statement: String,

    /// Upstream gold patch, if any.
    #[serde(default)]
    pub patch: Option<String>,
    /// Upstream gold test patch, if any.
    #[serde(default)]
    pub test_patch: Option<String>,
    /// Optional hints text.
    #[serde(default)]
    pub hints_text: Option<String>,
    /// Upstream task creation timestamp (RFC 3339 / ISO 8601).
    #[serde(default)]
    pub created_at: Option<String>,
    /// Upstream release / library version tag (for example `"4.3"`).
    #[serde(default)]
    pub version: Option<String>,
    /// Reference commit used for environment setup (SWE-bench harness).
    #[serde(default)]
    pub environment_setup_commit: Option<String>,

    /// Live-specific: per-task OCI image reference.
    #[serde(default)]
    pub docker_image: Option<String>,

    /// JSON-encoded array of test selectors that must flip from fail to pass.
    ///
    /// Upstream stores this as `FAIL_TO_PASS` (uppercase); accept both
    /// spellings so fixtures can use whichever convention they prefer.
    #[serde(default, alias = "FAIL_TO_PASS")]
    pub fail_to_pass: Option<serde_json::Value>,
    /// JSON-encoded array of test selectors that must stay passing.
    ///
    /// Accepts `PASS_TO_PASS` as well.
    #[serde(default, alias = "PASS_TO_PASS")]
    pub pass_to_pass: Option<serde_json::Value>,

    /// Any remaining fields not explicitly captured above.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl RawSweBenchRecord {
    /// Interpret `FAIL_TO_PASS` as a list of strings regardless of whether
    /// the upstream dataset stored it as a JSON array or as a JSON-encoded
    /// string containing a JSON array.
    ///
    /// Returns an empty vector if the field is absent or null.
    #[must_use]
    pub fn fail_to_pass_list(&self) -> Vec<String> {
        decode_string_array(self.fail_to_pass.as_ref())
    }

    /// Same as [`Self::fail_to_pass_list`] for `PASS_TO_PASS`.
    #[must_use]
    pub fn pass_to_pass_list(&self) -> Vec<String> {
        decode_string_array(self.pass_to_pass.as_ref())
    }
}

fn decode_string_array(v: Option<&serde_json::Value>) -> Vec<String> {
    let Some(v) = v else { return Vec::new() };
    match v {
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|x| x.as_str().map(ToOwned::to_owned))
            .collect(),
        serde_json::Value::String(s) => serde_json::from_str::<Vec<String>>(s).unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Errors produced while reading raw JSONL inputs.
#[derive(Debug, Error)]
pub enum RawReadError {
    /// I/O failure opening or reading a JSONL input.
    #[error("io: {path}: {source}")]
    Io {
        /// Path the error was observed on.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A JSONL line could not be deserialized into [`RawSweBenchRecord`].
    #[error("malformed record at {path}:{line}: {source}")]
    Parse {
        /// Source file.
        path: PathBuf,
        /// 1-based line number.
        line: usize,
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
    },
    /// The supplied source path does not exist.
    #[error("source not found: {0}")]
    NotFound(PathBuf),
}

/// Read every JSONL record from `source`.
///
/// `source` may be either a single `.jsonl` file or a directory. When a
/// directory is passed, every `*.jsonl` file directly under it (no
/// recursion) is read in lexicographic filename order. Each returned
/// record carries its originating file and 1-based line number as
/// diagnostic metadata.
///
/// Empty lines and lines that begin with `#` are skipped so hand-written
/// fixtures can carry comments.
pub fn read_jsonl(source: &Path) -> Result<Vec<RawRecordWithOrigin>, RawReadError> {
    if !source.exists() {
        return Err(RawReadError::NotFound(source.to_path_buf()));
    }
    let mut files: Vec<PathBuf> = Vec::new();
    if source.is_file() {
        files.push(source.to_path_buf());
    } else {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(source)
            .map_err(|source_err| RawReadError::Io {
                path: source.to_path_buf(),
                source: source_err,
            })?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("jsonl"))
            .collect();
        entries.sort();
        files.extend(entries);
    }

    let mut out: Vec<RawRecordWithOrigin> = Vec::new();
    for path in files {
        let file = File::open(&path).map_err(|source_err| RawReadError::Io {
            path: path.clone(),
            source: source_err,
        })?;
        let reader = BufReader::new(file);
        for (idx, line) in reader.lines().enumerate() {
            let line_no = idx + 1;
            let line = line.map_err(|source_err| RawReadError::Io {
                path: path.clone(),
                source: source_err,
            })?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let record: RawSweBenchRecord =
                serde_json::from_str(trimmed).map_err(|source_err| RawReadError::Parse {
                    path: path.clone(),
                    line: line_no,
                    source: source_err,
                })?;
            out.push(RawRecordWithOrigin {
                record,
                source_file: path.clone(),
                line_no,
            });
        }
    }
    Ok(out)
}

/// A parsed raw record paired with its origin for diagnostic purposes.
#[derive(Debug, Clone)]
pub struct RawRecordWithOrigin {
    /// The deserialized record.
    pub record: RawSweBenchRecord,
    /// Absolute path of the JSONL file the record came from.
    pub source_file: PathBuf,
    /// 1-based line number within that file.
    pub line_no: usize,
}

/// Apply the ingest options (filter + limit) to a list of raw records.
///
/// The filter is applied before the limit. Ordering is preserved.
#[must_use]
pub fn apply_filters<I>(records: I, only_ids: &[String], limit: Option<u32>) -> Vec<I::Item>
where
    I: IntoIterator<Item = RawRecordWithOrigin>,
{
    let mut out: Vec<RawRecordWithOrigin> = records
        .into_iter()
        .filter(|r| only_ids.is_empty() || only_ids.iter().any(|id| id == &r.record.instance_id))
        .collect();
    if let Some(n) = limit {
        out.truncate(usize::try_from(n).unwrap_or(usize::MAX));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn reads_single_jsonl_file() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("tasks.jsonl");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(
            f,
            r#"{{"instance_id":"x-1","repo":"a/b","base_commit":"deadbeefcafe","problem_statement":"p"}}"#
        )
        .unwrap();
        writeln!(f).unwrap();
        writeln!(f, "# comment").unwrap();
        writeln!(
            f,
            r#"{{"instance_id":"x-2","repo":"a/b","base_commit":"feedfacecafe","problem_statement":"q"}}"#
        )
        .unwrap();
        let recs = read_jsonl(&p).unwrap();
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].record.instance_id, "x-1");
        assert_eq!(recs[1].line_no, 4);
    }

    #[test]
    fn malformed_line_surfaces_line_number() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("bad.jsonl");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "not json").unwrap();
        let err = read_jsonl(&p).unwrap_err();
        assert!(matches!(err, RawReadError::Parse { line: 1, .. }));
    }

    #[test]
    fn decodes_fail_to_pass_string_form() {
        // `FAIL_TO_PASS` is the upstream uppercase spelling, stored as a
        // JSON-encoded string. Both aspects must round-trip.
        let v: serde_json::Value = serde_json::from_str(
            r#"{"instance_id":"a__b-1","repo":"a/b","base_commit":"deadbeefcafe","problem_statement":"x","FAIL_TO_PASS":"[\"a\",\"b\"]"}"#,
        )
        .unwrap();
        let r: RawSweBenchRecord = serde_json::from_value(v).unwrap();
        assert_eq!(r.fail_to_pass_list(), vec!["a".to_string(), "b".into()]);
    }
}
