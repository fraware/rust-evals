//! `EvaluationResult`: per-level verdict.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hash::Sha256Digest;
use crate::ids::{CandidateId, TaskId};
use crate::level::{EvaluationLevel, EvaluationStatus};
use crate::version::{EvaluatorVersion, SchemaVersion, EVALUATOR_VERSION, SCHEMA_VERSION};

/// Kind of artifact produced by an evaluator level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// Captured standard output.
    Stdout,
    /// Captured standard error.
    Stderr,
    /// Unified diff captured during evaluation.
    Diff,
    /// A human- or machine-readable report.
    Report,
    /// Trace JSONL path.
    Trace,
    /// Lean checker log for L4.
    LeanLog,
    /// Discrepancy trace (differential behaviour).
    Discrepancy,
    /// Raw metrics dump.
    MetricDump,
}

/// File-level artifact produced by an evaluator level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationArtifact {
    /// Artifact kind.
    pub kind: ArtifactKind,
    /// POSIX path inside the evidence bundle.
    pub path: String,
    /// Optional SHA-256 of the artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<Sha256Digest>,
}

/// Per-level verdict for a candidate.
///
/// `metrics` is a free-form `serde_json::Value`, which does not implement
/// `Eq`, so `EvaluationResult` cannot derive `Eq` either.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationResult {
    /// Schema version.
    pub schema_version: SchemaVersion,
    /// Candidate identifier.
    pub candidate_id: CandidateId,
    /// Benchmark-local task identifier.
    pub task_id: TaskId,
    /// Level of this verdict.
    pub level: EvaluationLevel,
    /// Status.
    pub status: EvaluationStatus,
    /// Stable primary failure reason (or `PASS`).
    pub primary_reason: String,
    /// Stable secondary reasons; may be empty.
    pub secondary_reasons: Vec<String>,
    /// Artifacts produced by this level.
    pub artifacts: Vec<EvaluationArtifact>,
    /// Free-form metrics payload.
    pub metrics: Value,
    /// Start timestamp.
    pub started_at: DateTime<Utc>,
    /// Finish timestamp.
    pub finished_at: DateTime<Utc>,
    /// Evaluator version that produced this result.
    pub evaluator_version: EvaluatorVersion,
}

impl EvaluationResult {
    /// Construct a fresh result with the current schema and evaluator versions.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        candidate_id: CandidateId,
        task_id: TaskId,
        level: EvaluationLevel,
        status: EvaluationStatus,
        primary_reason: impl Into<String>,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            candidate_id,
            task_id,
            level,
            status,
            primary_reason: primary_reason.into(),
            secondary_reasons: Vec::new(),
            artifacts: Vec::new(),
            metrics: Value::Object(serde_json::Map::new()),
            started_at,
            finished_at,
            evaluator_version: EVALUATOR_VERSION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::FailureReason;

    #[test]
    fn result_serializes_with_schema_version() {
        let task = TaskId::new("django__django-12345").unwrap();
        let candidate = CandidateId::new_v4();
        let now = Utc::now();
        let r = EvaluationResult::new(
            candidate,
            task,
            EvaluationLevel::L0Official,
            EvaluationStatus::Pass,
            FailureReason::PASS.as_str(),
            now,
            now,
        );
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["level"], "L0");
        assert_eq!(v["status"], "pass");
        assert_eq!(v["primary_reason"], "PASS");
    }
}
