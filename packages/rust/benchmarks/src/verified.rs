//! SWE-Bench Verified adapter.
//!
//! Ingests the JSONL export of the
//! [`princeton-nlp/SWE-bench_Verified`](https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified)
//! dataset and emits normalized [`BenchmarkTask`] manifests.
//!
//! Verified is static and Python-only. The adapter:
//! - reads every JSONL record from the supplied source (file or directory),
//! - applies the caller's `only_task_ids` and `limit` filters,
//! - normalizes each record through [`record_to_task`],
//! - schema-validates and atomically writes one manifest per task to
//!   `options.output_dir`.
//!
//! The adapter is deterministic: identical input always produces
//! byte-identical output files.

use std::path::Path;

use chrono::Utc;
use eval_ladder_core::{BenchmarkId, BenchmarkLanguage, BenchmarkTask, TaskId};
use tracing::{debug, info};

use crate::adapter::{BenchmarkAdapter, BenchmarkAdapterError, IngestOptions, IngestReport};
use crate::normalize::{
    issue_id_from_instance_id, issue_title_from_problem_statement, parse_created_at,
    VERIFIED_SOURCE_URL,
};
use crate::raw::{apply_filters, read_jsonl, RawSweBenchRecord};
use crate::writer::ManifestWriter;

/// SWE-Bench Verified adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct VerifiedAdapter;

impl VerifiedAdapter {
    /// Construct a fresh adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl BenchmarkAdapter for VerifiedAdapter {
    fn benchmark_id(&self) -> BenchmarkId {
        BenchmarkId::SweBenchVerified
    }

    fn ingest(
        &self,
        source_root: &Path,
        options: &IngestOptions,
    ) -> Result<IngestReport, BenchmarkAdapterError> {
        let out_dir = options.output_dir.clone().ok_or_else(|| {
            BenchmarkAdapterError::IngestFailed("ingest options must specify an output_dir".into())
        })?;
        info!(source = %source_root.display(), out_dir = %out_dir.display(), "verified ingest");

        let records = read_jsonl(source_root)
            .map_err(|e| BenchmarkAdapterError::IngestFailed(e.to_string()))?;
        let selected = apply_filters(records, &options.only_task_ids, options.limit);

        let writer = ManifestWriter::new(out_dir)
            .map_err(|e| BenchmarkAdapterError::IngestFailed(e.to_string()))?;

        let mut ingested = 0_u32;
        let mut skipped = 0_u32;
        let mut diagnostics: Vec<String> = Vec::new();
        let now = Utc::now();

        for origin in selected {
            let task = match record_to_task(&origin.record, now) {
                Ok(t) => t,
                Err(e) => {
                    skipped = skipped.saturating_add(1);
                    diagnostics.push(format!(
                        "{}:{} {}: {e}",
                        origin.source_file.display(),
                        origin.line_no,
                        origin.record.instance_id
                    ));
                    continue;
                }
            };
            writer
                .write(&task)
                .map_err(|e| BenchmarkAdapterError::IngestFailed(e.to_string()))?;
            debug!(task_id = %task.task_id, "verified task written");
            ingested = ingested.saturating_add(1);
        }

        Ok(IngestReport {
            benchmark_id: BenchmarkId::SweBenchVerified,
            tasks_ingested: ingested,
            tasks_skipped: skipped,
            diagnostics,
        })
    }

    fn load_task(&self, manifest_path: &Path) -> Result<BenchmarkTask, BenchmarkAdapterError> {
        let bytes = std::fs::read(manifest_path)?;
        let task: BenchmarkTask = serde_json::from_slice(&bytes)?;
        if task.benchmark_id != BenchmarkId::SweBenchVerified {
            return Err(BenchmarkAdapterError::IngestFailed(format!(
                "expected benchmark_id = swe_bench_verified, got {}",
                task.benchmark_id
            )));
        }
        Ok(task)
    }
}

/// Normalize a raw SWE-Bench Verified record into a [`BenchmarkTask`].
///
/// The function is pure and exposed for testing.
pub fn record_to_task(
    raw: &RawSweBenchRecord,
    ingest_now: chrono::DateTime<Utc>,
) -> Result<BenchmarkTask, BenchmarkAdapterError> {
    let task_id = TaskId::new(raw.instance_id.clone())?;
    let issue_id = issue_id_from_instance_id(&raw.instance_id);
    let issue_title = issue_title_from_problem_statement(&raw.problem_statement);
    let created_at = parse_created_at(&raw.instance_id, raw, ingest_now)?;

    let environment_ref = verified_environment_ref(&raw.instance_id);
    let official_test_entrypoint = format!(
        "python -m swebench.harness.run_evaluation --instance_ids {}",
        raw.instance_id
    );

    let mut labels: Vec<String> = Vec::new();
    if let Some(v) = raw.version.as_deref() {
        if !v.is_empty() {
            labels.push(format!("upstream_version:{v}"));
        }
    }
    if let Some(esc) = raw.environment_setup_commit.as_deref() {
        if !esc.is_empty() {
            labels.push(format!("env_setup_commit:{esc}"));
        }
    }
    let ftp = raw.fail_to_pass_list();
    let ptp = raw.pass_to_pass_list();
    if !ftp.is_empty() {
        labels.push(format!("fail_to_pass_count:{}", ftp.len()));
    }
    if !ptp.is_empty() {
        labels.push(format!("pass_to_pass_count:{}", ptp.len()));
    }
    labels.sort();
    labels.dedup();

    let mut task = BenchmarkTask::new(
        BenchmarkId::SweBenchVerified,
        task_id,
        raw.repo.clone(),
        issue_id,
        issue_title,
        raw.problem_statement.clone(),
        raw.base_commit.clone(),
        environment_ref,
        official_test_entrypoint,
        BenchmarkLanguage::Python,
        VERIFIED_SOURCE_URL,
        created_at,
    );
    task.labels = labels;
    Ok(task)
}

/// Standard SWE-bench harness image naming scheme.
///
/// Matches the `swebench/sweb.eval.x86_64.<sanitized_instance_id>:latest`
/// convention used by the official SWE-bench harness. Instance ids contain
/// `__` and `-`, neither of which need to be escaped for an image tag's
/// repository segment, but uppercase letters are lowercased (OCI registries
/// are case-insensitive for the repo name and conventional docker tooling
/// normalizes to lowercase).
fn verified_environment_ref(instance_id: &str) -> String {
    let sanitized = instance_id.to_ascii_lowercase();
    format!("swebench/sweb.eval.x86_64.{sanitized}:latest")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::RawSweBenchRecord;

    fn raw() -> RawSweBenchRecord {
        serde_json::from_value(serde_json::json!({
            "instance_id": "astropy__astropy-12907",
            "repo": "astropy/astropy",
            "base_commit": "d16bfe05a744909de4b27f5875fe0d4ed41ce607",
            "problem_statement": "# separability_matrix broken\nBody here.",
            "version": "4.3",
            "environment_setup_commit": "298ccb478e6bf092953bca67a3d29dc6c35f6752",
            "FAIL_TO_PASS": "[\"a\",\"b\"]",
            "PASS_TO_PASS": "[\"c\"]",
            "created_at": "2022-03-02T15:14:54Z"
        }))
        .unwrap()
    }

    #[test]
    fn maps_record_to_task() {
        let now = Utc::now();
        let t = record_to_task(&raw(), now).unwrap();
        assert_eq!(t.benchmark_id, BenchmarkId::SweBenchVerified);
        assert_eq!(t.task_id.as_str(), "astropy__astropy-12907");
        assert_eq!(t.repo_name, "astropy/astropy");
        assert_eq!(t.issue_id, "12907");
        assert_eq!(t.issue_title, "separability_matrix broken");
        assert_eq!(t.language, BenchmarkLanguage::Python);
        assert_eq!(t.base_commit, "d16bfe05a744909de4b27f5875fe0d4ed41ce607");
        assert_eq!(
            t.environment_ref,
            "swebench/sweb.eval.x86_64.astropy__astropy-12907:latest"
        );
        assert!(t.labels.contains(&"upstream_version:4.3".into()));
        assert!(t.labels.contains(&"fail_to_pass_count:2".into()));
        assert!(t.labels.contains(&"pass_to_pass_count:1".into()));
        assert!(t.labels.windows(2).all(|w| w[0] <= w[1]), "labels sorted");
        assert_eq!(t.created_at.to_rfc3339(), "2022-03-02T15:14:54+00:00");
    }

    #[test]
    fn falls_back_to_ingest_time_when_created_at_missing() {
        let mut r = raw();
        r.created_at = None;
        let fallback = Utc::now();
        let t = record_to_task(&r, fallback).unwrap();
        assert_eq!(t.created_at, fallback);
    }
}
