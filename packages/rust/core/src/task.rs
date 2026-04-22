//! `BenchmarkTask`: normalized task manifest emitted by benchmark adapters.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{BenchmarkId, TaskId};
use crate::version::{SchemaVersion, SCHEMA_VERSION};

/// The language the benchmark task is authored in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkLanguage {
    /// Python.
    Python,
    /// Rust.
    Rust,
    /// Tasks that involve more than one language (for example, Python + C
    /// extensions). Kept explicit so language-specific validators can refuse
    /// these.
    Mixed,
}

/// Normalized benchmark task.
///
/// Mirrors `schemas/benchmark_task.schema.json` exactly. The `schema_version`
/// field is initialized to the current version by [`Self::new`] and must be
/// checked by every deserializer path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkTask {
    /// Schema version.
    pub schema_version: SchemaVersion,
    /// Benchmark suite discriminator.
    pub benchmark_id: BenchmarkId,
    /// Benchmark-local task identifier.
    pub task_id: TaskId,
    /// Upstream repository name (for example `django/django`).
    pub repo_name: String,
    /// Upstream issue identifier.
    pub issue_id: String,
    /// Issue title.
    pub issue_title: String,
    /// Issue body / description.
    pub issue_text: String,
    /// Base commit the task is pinned to.
    pub base_commit: String,
    /// Optional reference to a gold patch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gold_patch_ref: Option<String>,
    /// Image or environment descriptor pinned to this task.
    pub environment_ref: String,
    /// Official scorer entrypoint command.
    pub official_test_entrypoint: String,
    /// Language the task is authored in.
    pub language: BenchmarkLanguage,
    /// Free-form labels (benchmark-specific).
    pub labels: Vec<String>,
    /// Ingest time.
    pub created_at: DateTime<Utc>,
    /// Public source URL.
    pub source_url: String,
}

impl BenchmarkTask {
    /// Construct a task manifest, filling in `schema_version` from the current
    /// build.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        benchmark_id: BenchmarkId,
        task_id: TaskId,
        repo_name: impl Into<String>,
        issue_id: impl Into<String>,
        issue_title: impl Into<String>,
        issue_text: impl Into<String>,
        base_commit: impl Into<String>,
        environment_ref: impl Into<String>,
        official_test_entrypoint: impl Into<String>,
        language: BenchmarkLanguage,
        source_url: impl Into<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            benchmark_id,
            task_id,
            repo_name: repo_name.into(),
            issue_id: issue_id.into(),
            issue_title: issue_title.into(),
            issue_text: issue_text.into(),
            base_commit: base_commit.into(),
            gold_patch_ref: None,
            environment_ref: environment_ref.into(),
            official_test_entrypoint: official_test_entrypoint.into(),
            language,
            labels: Vec::new(),
            created_at,
            source_url: source_url.into(),
        }
    }
}
