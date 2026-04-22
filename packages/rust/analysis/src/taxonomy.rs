//! False-success taxonomy counts.
//!
//! Aggregates every stable primary-reason code that appears on `fail` rows
//! and emits a count per (benchmark, level, code) triple. The analysis is
//! deliberately simple: the stable codes defined in
//! `eval_ladder_core::FailureReason` and `eval_ladder_core::PolicyViolation`
//! carry the semantics; this module only counts.

use std::collections::BTreeMap;

use eval_ladder_core::{BenchmarkId, EvaluationLevel, EvaluationStatus};
use serde::{Deserialize, Serialize};

use crate::input::AnalysisInput;

/// One row of the taxonomy table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyRow {
    /// Benchmark suite.
    pub benchmark_id: BenchmarkId,
    /// Level.
    pub level: EvaluationLevel,
    /// Stable primary reason code (see `docs/evaluation_ladder.md`).
    pub primary_reason: String,
    /// Count of rows with this combination.
    pub count: u64,
}

/// Count occurrences of every primary reason on `fail` rows, grouped by
/// (benchmark, level, code).
#[must_use]
pub fn taxonomy_counts(input: &AnalysisInput) -> Vec<TaxonomyRow> {
    let mut agg: BTreeMap<(BenchmarkId, EvaluationLevel, String), u64> = BTreeMap::new();
    for row in &input.rows {
        if row.status != EvaluationStatus::Fail {
            continue;
        }
        let key = (row.benchmark_id, row.level, row.primary_reason.clone());
        *agg.entry(key).or_insert(0) += 1;
    }
    agg.into_iter()
        .map(
            |((benchmark_id, level, primary_reason), count)| TaxonomyRow {
                benchmark_id,
                level,
                primary_reason,
                count,
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_ladder_core::{CandidateId, TaskId};

    use crate::input::AnalysisInputRow;

    #[test]
    fn counts_group_by_bench_level_reason() {
        let c1 = CandidateId::new_v4();
        let rows = vec![
            AnalysisInputRow {
                candidate_id: c1,
                task_id: TaskId::new("t").unwrap(),
                benchmark_id: BenchmarkId::SweBenchVerified,
                agent_id: "a".into(),
                model_id: "m".into(),
                level: EvaluationLevel::L2Strengthened,
                status: EvaluationStatus::Fail,
                primary_reason: "L2_DIFF_BEHAVIOR".into(),
                task_category: None,
            },
            AnalysisInputRow {
                candidate_id: CandidateId::new_v4(),
                task_id: TaskId::new("t2").unwrap(),
                benchmark_id: BenchmarkId::SweBenchVerified,
                agent_id: "a".into(),
                model_id: "m".into(),
                level: EvaluationLevel::L2Strengthened,
                status: EvaluationStatus::Fail,
                primary_reason: "L2_DIFF_BEHAVIOR".into(),
                task_category: None,
            },
        ];
        let table = taxonomy_counts(&AnalysisInput { rows });
        assert_eq!(table.len(), 1);
        assert_eq!(table[0].count, 2);
    }
}
