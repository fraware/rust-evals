//! Rust-SWE-bench adapter.
//!
//! Rust tasks mirror the SWE-bench record shape but resolve to a
//! Cargo-based test entrypoint and a synthesized environment reference
//! when the upstream release does not ship per-task OCI images. When a
//! `docker_image` is provided we honor it; otherwise we emit a stable
//! content-addressed descriptor derived from the task's
//! `base_commit` so that every evaluator run in the same codebase state
//! ends up with the same environment reference.

use std::path::Path;

use chrono::Utc;
use eval_ladder_core::{BenchmarkId, BenchmarkLanguage, BenchmarkTask, TaskId};
use tracing::{debug, info};

use crate::adapter::{BenchmarkAdapter, BenchmarkAdapterError, IngestOptions, IngestReport};
use crate::normalize::{
    issue_id_from_instance_id, issue_title_from_problem_statement, parse_created_at,
    RUST_SOURCE_URL,
};
use crate::raw::{apply_filters, read_jsonl, RawSweBenchRecord};
use crate::writer::ManifestWriter;

/// Rust-SWE-bench adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct RustNativeAdapter;

impl RustNativeAdapter {
    /// Construct a fresh adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl BenchmarkAdapter for RustNativeAdapter {
    fn benchmark_id(&self) -> BenchmarkId {
        BenchmarkId::RustSweBench
    }

    fn ingest(
        &self,
        source_root: &Path,
        options: &IngestOptions,
    ) -> Result<IngestReport, BenchmarkAdapterError> {
        let out_dir = options.output_dir.clone().ok_or_else(|| {
            BenchmarkAdapterError::IngestFailed("ingest options must specify an output_dir".into())
        })?;
        info!(source = %source_root.display(), out_dir = %out_dir.display(), "rust_native ingest");

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
            debug!(task_id = %task.task_id, "rust task written");
            ingested = ingested.saturating_add(1);
        }

        Ok(IngestReport {
            benchmark_id: BenchmarkId::RustSweBench,
            tasks_ingested: ingested,
            tasks_skipped: skipped,
            diagnostics,
        })
    }

    fn load_task(&self, manifest_path: &Path) -> Result<BenchmarkTask, BenchmarkAdapterError> {
        let bytes = std::fs::read(manifest_path)?;
        let task: BenchmarkTask = serde_json::from_slice(&bytes)?;
        if task.benchmark_id != BenchmarkId::RustSweBench {
            return Err(BenchmarkAdapterError::IngestFailed(format!(
                "expected benchmark_id = rust_swe_bench, got {}",
                task.benchmark_id
            )));
        }
        Ok(task)
    }
}

/// Normalize a raw Rust-SWE-bench record into a [`BenchmarkTask`].
///
/// - When `docker_image` is provided it is taken verbatim.
/// - Otherwise the environment reference is `cargo://<repo>@<short_sha>`,
///   which is intentionally not a docker image; it is a descriptor the
///   runner will resolve against a workspace checkout at L0/L1 time.
pub fn record_to_task(
    raw: &RawSweBenchRecord,
    ingest_now: chrono::DateTime<Utc>,
) -> Result<BenchmarkTask, BenchmarkAdapterError> {
    let task_id = TaskId::new(raw.instance_id.clone())?;
    let issue_id = issue_id_from_instance_id(&raw.instance_id);
    let issue_title = issue_title_from_problem_statement(&raw.problem_statement);
    let created_at = parse_created_at(&raw.instance_id, raw, ingest_now)?;

    let environment_ref = match raw.docker_image.as_deref() {
        Some(img) if !img.is_empty() => img.to_owned(),
        _ => {
            let short = raw.base_commit.chars().take(12).collect::<String>();
            format!("cargo://{}@{}", raw.repo, short)
        }
    };

    // Cargo's deterministic test invocation. `--locked` refuses to edit
    // Cargo.lock; `--workspace` covers repository-level tasks; a nocapture
    // flag is deliberately *not* added here so the official scorer output
    // remains comparable to upstream Rust-SWE-bench numbers.
    let official_test_entrypoint = "cargo test --workspace --locked".to_string();

    let mut labels: Vec<String> = vec!["rust".into(), format!("repo:{}", raw.repo)];
    if let Some(v) = raw.version.as_deref() {
        if !v.is_empty() {
            labels.push(format!("upstream_version:{v}"));
        }
    }
    labels.sort();
    labels.dedup();

    let mut task = BenchmarkTask::new(
        BenchmarkId::RustSweBench,
        task_id,
        raw.repo.clone(),
        issue_id,
        issue_title,
        raw.problem_statement.clone(),
        raw.base_commit.clone(),
        environment_ref,
        official_test_entrypoint,
        BenchmarkLanguage::Rust,
        RUST_SOURCE_URL,
        created_at,
    );
    task.labels = labels;
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_rust() -> RawSweBenchRecord {
        serde_json::from_value(serde_json::json!({
            "instance_id": "tokio-rs__tokio-6789",
            "repo": "tokio-rs/tokio",
            "base_commit": "abcdef0123456789",
            "problem_statement": "Deadlock in select! expansion\n\nBody.",
            "created_at": "2024-05-04T12:00:00Z"
        }))
        .unwrap()
    }

    #[test]
    fn synthesizes_cargo_environment_ref_when_docker_absent() {
        let t = record_to_task(&raw_rust(), Utc::now()).unwrap();
        assert_eq!(t.environment_ref, "cargo://tokio-rs/tokio@abcdef012345");
        assert_eq!(
            t.official_test_entrypoint,
            "cargo test --workspace --locked"
        );
        assert_eq!(t.language, BenchmarkLanguage::Rust);
        assert!(t.labels.contains(&"rust".into()));
        assert!(t.labels.contains(&"repo:tokio-rs/tokio".into()));
    }

    #[test]
    fn honors_explicit_docker_image() {
        let mut r = raw_rust();
        r.docker_image = Some("ghcr.io/fraware/rust-swebench:tokio-6789".into());
        let t = record_to_task(&r, Utc::now()).unwrap();
        assert_eq!(
            t.environment_ref,
            "ghcr.io/fraware/rust-swebench:tokio-6789"
        );
    }
}
