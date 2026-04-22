//! SWE-bench-Live adapter.
//!
//! Live ingest preserves per-task freshness metadata that Verified does
//! not carry:
//!
//! - `docker_image` is **required**; Live ships one OCI image per task
//!   and we refuse to invent one when missing.
//! - the upstream `created_at` becomes the manifest `created_at`, giving
//!   analysis code a stable freshness signal.
//! - a `live` label and a `repo:<owner/repo>` label are emitted so
//!   stratified analysis can slice by repository without re-parsing
//!   `repo_name`.

use std::path::Path;

use chrono::Utc;
use eval_ladder_core::{BenchmarkId, BenchmarkLanguage, BenchmarkTask, TaskId};
use tracing::{debug, info};

use crate::adapter::{BenchmarkAdapter, BenchmarkAdapterError, IngestOptions, IngestReport};
use crate::normalize::{
    issue_id_from_instance_id, issue_title_from_problem_statement, parse_created_at,
    LIVE_SOURCE_URL,
};
use crate::raw::{apply_filters, read_jsonl, RawSweBenchRecord};
use crate::writer::ManifestWriter;

/// SWE-bench-Live adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct LiveAdapter;

impl LiveAdapter {
    /// Construct a fresh adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl BenchmarkAdapter for LiveAdapter {
    fn benchmark_id(&self) -> BenchmarkId {
        BenchmarkId::SweBenchLive
    }

    fn ingest(
        &self,
        source_root: &Path,
        options: &IngestOptions,
    ) -> Result<IngestReport, BenchmarkAdapterError> {
        let out_dir = options.output_dir.clone().ok_or_else(|| {
            BenchmarkAdapterError::IngestFailed("ingest options must specify an output_dir".into())
        })?;
        info!(source = %source_root.display(), out_dir = %out_dir.display(), "live ingest");

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
            debug!(task_id = %task.task_id, "live task written");
            ingested = ingested.saturating_add(1);
        }

        Ok(IngestReport {
            benchmark_id: BenchmarkId::SweBenchLive,
            tasks_ingested: ingested,
            tasks_skipped: skipped,
            diagnostics,
        })
    }

    fn load_task(&self, manifest_path: &Path) -> Result<BenchmarkTask, BenchmarkAdapterError> {
        let bytes = std::fs::read(manifest_path)?;
        let task: BenchmarkTask = serde_json::from_slice(&bytes)?;
        if task.benchmark_id != BenchmarkId::SweBenchLive {
            return Err(BenchmarkAdapterError::IngestFailed(format!(
                "expected benchmark_id = swe_bench_live, got {}",
                task.benchmark_id
            )));
        }
        Ok(task)
    }
}

/// Normalize a raw SWE-bench-Live record into a [`BenchmarkTask`].
///
/// Returns an [`BenchmarkAdapterError::EnvironmentUnresolved`] error when
/// the record does not carry a `docker_image`; Live cannot synthesize a
/// stable per-task image reference and refusing here forces the upstream
/// release to be treated as incomplete.
pub fn record_to_task(
    raw: &RawSweBenchRecord,
    ingest_now: chrono::DateTime<Utc>,
) -> Result<BenchmarkTask, BenchmarkAdapterError> {
    let docker_image = raw
        .docker_image
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BenchmarkAdapterError::EnvironmentUnresolved {
            task_id: raw.instance_id.clone(),
            reason: "Live task is missing required field `docker_image`".into(),
        })?
        .to_owned();

    let task_id = TaskId::new(raw.instance_id.clone())?;
    let issue_id = issue_id_from_instance_id(&raw.instance_id);
    let issue_title = issue_title_from_problem_statement(&raw.problem_statement);
    let created_at = parse_created_at(&raw.instance_id, raw, ingest_now)?;
    let official_test_entrypoint = format!(
        "python -m swebench.harness.run_evaluation --instance_ids {} --dataset_name SWE-bench-Live",
        raw.instance_id
    );

    let mut labels: Vec<String> = vec!["live".into(), format!("repo:{}", raw.repo)];
    if let Some(v) = raw.version.as_deref() {
        if !v.is_empty() {
            labels.push(format!("upstream_version:{v}"));
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
        BenchmarkId::SweBenchLive,
        task_id,
        raw.repo.clone(),
        issue_id,
        issue_title,
        raw.problem_statement.clone(),
        raw.base_commit.clone(),
        docker_image,
        official_test_entrypoint,
        BenchmarkLanguage::Python,
        LIVE_SOURCE_URL,
        created_at,
    );
    task.labels = labels;
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_live() -> RawSweBenchRecord {
        serde_json::from_value(serde_json::json!({
            "instance_id": "apache__airflow-41234",
            "repo": "apache/airflow",
            "base_commit": "1234567abcdef",
            "problem_statement": "Scheduler race condition\n\nDetails...",
            "docker_image": "swebenchlive/apache__airflow-41234:sha-abc",
            "created_at": "2024-08-10T09:00:00Z"
        }))
        .unwrap()
    }

    #[test]
    fn environment_ref_comes_from_docker_image() {
        let t = record_to_task(&raw_live(), Utc::now()).unwrap();
        assert_eq!(
            t.environment_ref,
            "swebenchlive/apache__airflow-41234:sha-abc"
        );
        assert!(t.labels.contains(&"live".into()));
        assert!(t.labels.contains(&"repo:apache/airflow".into()));
    }

    #[test]
    fn missing_docker_image_errors() {
        let mut r = raw_live();
        r.docker_image = None;
        let err = record_to_task(&r, Utc::now()).unwrap_err();
        assert!(matches!(
            err,
            BenchmarkAdapterError::EnvironmentUnresolved { .. }
        ));
    }

    #[test]
    fn empty_docker_image_errors() {
        let mut r = raw_live();
        r.docker_image = Some(String::new());
        let err = record_to_task(&r, Utc::now()).unwrap_err();
        assert!(matches!(
            err,
            BenchmarkAdapterError::EnvironmentUnresolved { .. }
        ));
    }
}
