//! Canonical analysis input.
//!
//! The analysis crate operates on a flat, denormalized view: one row per
//! (candidate, level) pair. Callers assemble this view from evidence bundles
//! or directly from in-memory `EvaluationResult` collections.

use eval_ladder_core::{
    BenchmarkId, CandidateId, EvaluationLevel, EvaluationResult, EvaluationStatus, TaskId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Analysis semantics for derived reporting tables.
///
/// `Raw` preserves the evaluator contract exactly: each level is judged
/// independently and reported as-is.
///
/// `Cumulative` derives headline pass semantics where an upper level can
/// only count as pass when all required lower levels also passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMode {
    /// Preserve raw level independence.
    Raw,
    /// Require lower-level pass preconditions for upper-level pass.
    Cumulative,
}

impl Default for AnalysisMode {
    fn default() -> Self {
        Self::Raw
    }
}

/// Stable primary reason used when a cumulative pass is blocked by an unmet
/// lower-level precondition.
pub const CUMULATIVE_PREREQUISITE_NOT_MET: &str = "CUMULATIVE_PREREQUISITE_NOT_MET";

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

/// Project `input` into the requested [`AnalysisMode`].
///
/// `Raw` returns a clone of the original rows.
///
/// `Cumulative` keeps every raw row but rewrites upper-level `Pass` verdicts
/// to `Fail` when required lower-level prerequisites are not satisfied for
/// that candidate. Raw non-pass statuses are preserved.
#[must_use]
pub fn project_analysis_mode(input: &AnalysisInput, mode: AnalysisMode) -> AnalysisInput {
    match mode {
        AnalysisMode::Raw => input.clone(),
        AnalysisMode::Cumulative => cumulative_projection(input),
    }
}

fn cumulative_projection(input: &AnalysisInput) -> AnalysisInput {
    let mut by_candidate: BTreeMap<CandidateId, Vec<usize>> = BTreeMap::new();
    for (idx, row) in input.rows.iter().enumerate() {
        by_candidate.entry(row.candidate_id).or_default().push(idx);
    }

    let mut out = input.clone();
    for indices in by_candidate.values() {
        let mut verdicts: BTreeMap<EvaluationLevel, EvaluationStatus> = BTreeMap::new();
        for &idx in indices {
            let row = &input.rows[idx];
            verdicts.insert(row.level, row.status);
        }

        let l2_requested = verdicts.contains_key(&EvaluationLevel::L2Strengthened);
        let l3_requested = verdicts.contains_key(&EvaluationLevel::L3PolicyConformant);

        for &idx in indices {
            let row = &input.rows[idx];
            if row.status != EvaluationStatus::Pass {
                continue;
            }

            if prerequisites_met(row.level, &verdicts, l2_requested, l3_requested) {
                continue;
            }

            // Derived headline semantics only affect the pass bit.
            out.rows[idx].status = EvaluationStatus::Fail;
            CUMULATIVE_PREREQUISITE_NOT_MET.clone_into(&mut out.rows[idx].primary_reason);
        }
    }

    out
}

fn prerequisites_met(
    level: EvaluationLevel,
    verdicts: &BTreeMap<EvaluationLevel, EvaluationStatus>,
    l2_requested: bool,
    l3_requested: bool,
) -> bool {
    let mut req: Vec<EvaluationLevel> = Vec::new();
    match level {
        EvaluationLevel::L0Official => (),
        EvaluationLevel::L1TrustedRerun => {
            req.push(EvaluationLevel::L0Official);
        }
        EvaluationLevel::L2Strengthened => {
            req.push(EvaluationLevel::L0Official);
            req.push(EvaluationLevel::L1TrustedRerun);
        }
        EvaluationLevel::L3PolicyConformant => {
            req.push(EvaluationLevel::L0Official);
            req.push(EvaluationLevel::L1TrustedRerun);
            if l2_requested {
                req.push(EvaluationLevel::L2Strengthened);
            }
        }
        EvaluationLevel::L4Semantic => {
            req.push(EvaluationLevel::L0Official);
            req.push(EvaluationLevel::L1TrustedRerun);
            if l2_requested {
                req.push(EvaluationLevel::L2Strengthened);
            }
            if l3_requested {
                req.push(EvaluationLevel::L3PolicyConformant);
            }
        }
    }
    req.into_iter()
        .all(|lvl| verdicts.get(&lvl) == Some(&EvaluationStatus::Pass))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_row(level: EvaluationLevel, status: EvaluationStatus, reason: &str) -> AnalysisInputRow {
        AnalysisInputRow {
            candidate_id: CandidateId::new_v4(),
            task_id: TaskId::new("fixture__task-1").unwrap(),
            benchmark_id: BenchmarkId::RustSweBench,
            agent_id: "agent".into(),
            model_id: "model".into(),
            level,
            status,
            primary_reason: reason.into(),
            task_category: None,
        }
    }

    #[test]
    fn cumulative_blocks_upper_pass_when_lower_invalid() {
        let cid = CandidateId::new_v4();
        let mut rows = vec![
            mk_row(
                EvaluationLevel::L0Official,
                EvaluationStatus::Invalid,
                "L0_OFFICIAL_TIMEOUT",
            ),
            mk_row(
                EvaluationLevel::L1TrustedRerun,
                EvaluationStatus::Invalid,
                "L1_HARNESS_ERROR",
            ),
            mk_row(
                EvaluationLevel::L3PolicyConformant,
                EvaluationStatus::Pass,
                "PASS",
            ),
            mk_row(
                EvaluationLevel::L4Semantic,
                EvaluationStatus::Pass,
                "L4_OBLIGATION_MET",
            ),
        ];
        for row in &mut rows {
            row.candidate_id = cid;
        }
        let input = AnalysisInput { rows };
        let projected = project_analysis_mode(&input, AnalysisMode::Cumulative);

        let l3 = projected
            .rows
            .iter()
            .find(|r| r.level == EvaluationLevel::L3PolicyConformant)
            .unwrap();
        let l4 = projected
            .rows
            .iter()
            .find(|r| r.level == EvaluationLevel::L4Semantic)
            .unwrap();
        assert_eq!(l3.status, EvaluationStatus::Fail);
        assert_eq!(l4.status, EvaluationStatus::Fail);
        assert_eq!(l3.primary_reason, CUMULATIVE_PREREQUISITE_NOT_MET);
        assert_eq!(l4.primary_reason, CUMULATIVE_PREREQUISITE_NOT_MET);
    }

    #[test]
    fn cumulative_keeps_raw_non_pass_statuses() {
        let cid = CandidateId::new_v4();
        let mut rows = vec![
            mk_row(EvaluationLevel::L0Official, EvaluationStatus::Pass, "PASS"),
            mk_row(
                EvaluationLevel::L1TrustedRerun,
                EvaluationStatus::Fail,
                "L1_TIMEOUT",
            ),
            mk_row(
                EvaluationLevel::L3PolicyConformant,
                EvaluationStatus::Pass,
                "PASS",
            ),
        ];
        for row in &mut rows {
            row.candidate_id = cid;
        }
        let projected = project_analysis_mode(&AnalysisInput { rows }, AnalysisMode::Cumulative);
        let l1 = projected
            .rows
            .iter()
            .find(|r| r.level == EvaluationLevel::L1TrustedRerun)
            .unwrap();
        assert_eq!(l1.status, EvaluationStatus::Fail);
        assert_eq!(l1.primary_reason, "L1_TIMEOUT");
    }
}
