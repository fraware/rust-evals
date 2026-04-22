//! Canonical analysis input.
//!
//! The analysis crate operates on a flat, denormalized view: one row per
//! (candidate, level) pair. Callers assemble this view from evidence bundles
//! or directly from in-memory `EvaluationResult` collections.

use eval_ladder_core::{
    BenchmarkId, CandidateId, EvaluationLevel, EvaluationResult, EvaluationStatus, TaskId,
};
use serde::{Deserialize, Serialize};

/// One row of analysis input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisInputRow {
    /// Candidate identifier.
    pub candidate_id: CandidateId,
    /// Task identifier.
    pub task_id: TaskId,
    /// Benchmark suite.
    pub benchmark_id: BenchmarkId,
    /// Agent harness identifier.
    pub agent_id: String,
    /// Model identifier used by the agent.
    pub model_id: String,
    /// Level this row reports on.
    pub level: EvaluationLevel,
    /// Status.
    pub status: EvaluationStatus,
    /// Stable primary reason code (for example `PASS`, `L2_DIFF_BEHAVIOR`).
    pub primary_reason: String,
    /// Optional task category label (from the task manifest).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_category: Option<String>,
}

impl AnalysisInputRow {
    /// Construct a row by combining a candidate header and an evaluation
    /// result.
    #[must_use]
    pub fn from_candidate_and_result(
        benchmark_id: BenchmarkId,
        agent_id: impl Into<String>,
        model_id: impl Into<String>,
        result: &EvaluationResult,
        task_category: Option<String>,
    ) -> Self {
        Self {
            candidate_id: result.candidate_id,
            task_id: result.task_id.clone(),
            benchmark_id,
            agent_id: agent_id.into(),
            model_id: model_id.into(),
            level: result.level,
            status: result.status,
            primary_reason: result.primary_reason.clone(),
            task_category,
        }
    }
}

/// Collection of rows forming an analysis input.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisInput {
    /// Analysis rows.
    pub rows: Vec<AnalysisInputRow>,
}

impl AnalysisInput {
    /// Number of rows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether there are any rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Iterate over all rows.
    pub fn iter(&self) -> impl Iterator<Item = &AnalysisInputRow> {
        self.rows.iter()
    }
}
