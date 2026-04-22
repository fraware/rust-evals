//! Adapter trait and shared types.

use std::path::{Path, PathBuf};

use eval_ladder_core::{BenchmarkId, BenchmarkTask};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Options passed to `ingest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IngestOptions {
    /// Filter to a subset of task ids. Empty means "all".
    #[serde(default)]
    pub only_task_ids: Vec<String>,
    /// Output directory where normalized manifests should be written
    /// (one JSON file per task).
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
    /// Maximum number of tasks to ingest (useful for smoke tests).
    #[serde(default)]
    pub limit: Option<u32>,
}

/// Result of an ingest invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IngestReport {
    /// Benchmark ingested.
    pub benchmark_id: BenchmarkId,
    /// Number of tasks normalized.
    pub tasks_ingested: u32,
    /// Number of tasks skipped (for any reason).
    pub tasks_skipped: u32,
    /// Diagnostic messages (not failures).
    pub diagnostics: Vec<String>,
}

/// Errors produced by an adapter.
#[derive(Debug, Error)]
pub enum BenchmarkAdapterError {
    /// Adapter-specific source ingestion failed.
    #[error("ingest failed: {0}")]
    IngestFailed(String),
    /// A task's environment reference could not be resolved.
    #[error("environment unresolved for task {task_id}: {reason}")]
    EnvironmentUnresolved {
        /// Task id.
        task_id: String,
        /// Human reason.
        reason: String,
    },
    /// Core-layer error.
    #[error("core: {0}")]
    Core(#[from] eval_ladder_core::CoreError),
    /// I/O.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// TOML parse error.
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
    /// The adapter has not yet implemented this step (Milestone B placeholder).
    #[error("adapter step not yet implemented: {0}")]
    NotYetImplemented(&'static str),
}

/// Adapter trait implemented by each benchmark module.
pub trait BenchmarkAdapter: Send + Sync {
    /// Which benchmark this adapter targets.
    fn benchmark_id(&self) -> BenchmarkId;

    /// Ingest benchmark tasks from their public source into normalized
    /// `BenchmarkTask` manifests.
    ///
    /// `source_root` is the directory the adapter should read from (typically
    /// a locally mirrored copy of the public dataset).
    fn ingest(
        &self,
        source_root: &Path,
        options: &IngestOptions,
    ) -> Result<IngestReport, BenchmarkAdapterError>;

    /// Load a normalized task manifest from `manifest_path`.
    fn load_task(&self, manifest_path: &Path) -> Result<BenchmarkTask, BenchmarkAdapterError>;
}
