//! `CandidateResolution`: the canonical evaluation unit.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{BenchmarkId, CandidateId, TaskId};
use crate::version::{SchemaVersion, SCHEMA_VERSION};

/// How the candidate patch was generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationMode {
    /// Multi-step agent loop with tool use.
    AgentLoop,
    /// Single-shot model call with no iterative tool use.
    SingleShot,
    /// Reranking over multiple candidates.
    Rerank,
    /// Human-in-the-loop.
    HumanAssisted,
    /// Any mode not otherwise enumerated (explain in `tool_configuration`).
    Other,
}

/// Patch representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchFormat {
    /// Unified diff.
    UnifiedDiff,
    /// Git-formatted patch (includes commit metadata).
    GitPatch,
    /// Structured JSON edits (for non-text or AST-level edits).
    JsonEdits,
}

/// Metadata the agent must declare about how the candidate was generated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationMetadata {
    /// Sampling temperature, if applicable.
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Tool / harness configuration, free-form.
    pub tool_configuration: Value,
    /// Context-window mode used by the agent.
    pub context_mode: ContextMode,
    /// Whether the agent reproduced the repository locally before editing.
    pub repo_reproduction_used: bool,
    /// Random seed propagated by the agent runtime, if any.
    #[serde(default)]
    pub random_seed: Option<i64>,
}

/// Context-window mode enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextMode {
    /// Whole repository in context.
    FullRepo,
    /// Retrieval-augmented context.
    Retrieval,
    /// File-level context.
    FileLevel,
    /// Fixed window of lines.
    Window,
    /// Any mode not otherwise enumerated.
    Other,
}

/// The canonical evaluation unit. Mirrors
/// `schemas/candidate_resolution.schema.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateResolution {
    /// Schema version.
    pub schema_version: SchemaVersion,
    /// Candidate identifier.
    pub candidate_id: CandidateId,
    /// Benchmark suite discriminator.
    pub benchmark_id: BenchmarkId,
    /// Benchmark-local task identifier.
    pub task_id: TaskId,
    /// Agent system identifier (not the model; the harness).
    pub agent_id: String,
    /// Model identifier used by the agent.
    pub model_id: String,
    /// Generation mode.
    pub generation_mode: GenerationMode,
    /// Patch format.
    pub patch_format: PatchFormat,
    /// POSIX path or content-addressed reference to the patch.
    pub patch_ref: String,
    /// Optional reference to a trajectory artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trajectory_ref: Option<String>,
    /// Generation metadata.
    pub generation_metadata: GenerationMetadata,
    /// Submission timestamp.
    pub submitted_at: DateTime<Utc>,
}

impl CandidateResolution {
    /// Create a fresh candidate record with a new `UUIDv4` identifier.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        benchmark_id: BenchmarkId,
        task_id: TaskId,
        agent_id: impl Into<String>,
        model_id: impl Into<String>,
        generation_mode: GenerationMode,
        patch_format: PatchFormat,
        patch_ref: impl Into<String>,
        generation_metadata: GenerationMetadata,
        submitted_at: DateTime<Utc>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            candidate_id: CandidateId::new_v4(),
            benchmark_id,
            task_id,
            agent_id: agent_id.into(),
            model_id: model_id.into(),
            generation_mode,
            patch_format,
            patch_ref: patch_ref.into(),
            trajectory_ref: None,
            generation_metadata,
            submitted_at,
        }
    }
}
