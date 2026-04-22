//! Score descent and conditional false-success metrics.

use std::collections::BTreeMap;

use eval_ladder_core::{BenchmarkId, EvaluationLevel, EvaluationStatus};
use serde::{Deserialize, Serialize};

use crate::input::AnalysisInput;

/// Aggregation key for score-descent tables.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Stratum {
    /// Benchmark suite. `None` = pooled across benchmarks.
    pub benchmark_id: Option<BenchmarkId>,
    /// Agent identifier. `None` = pooled across agents.
    pub agent_id: Option<String>,
}

/// One row of the score-descent table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreDescentRow {
    /// Stratum.
    pub stratum: Stratum,
    /// Level.
    pub level: EvaluationLevel,
    /// Number of candidates that passed at this level.
    pub passed: u64,
    /// Number of candidates evaluated at this level, excluding
    /// `not_applicable`.
    pub evaluated: u64,
    /// `passed / evaluated`. `None` if `evaluated == 0`.
    pub pass_rate: Option<f64>,
}

/// Pass rate by level, stratified by benchmark and agent.
///
/// The returned rows include a row for the pooled (`benchmark_id = None`,
/// `agent_id = None`) stratum per level, plus per-benchmark, per-agent, and
/// per-(benchmark, agent) rows.
#[must_use]
pub fn score_descent(input: &AnalysisInput) -> Vec<ScoreDescentRow> {
    #[derive(Default, Clone, Copy)]
    struct Counts {
        passed: u64,
        evaluated: u64,
    }

    let mut buckets: BTreeMap<(Stratum, EvaluationLevel), Counts> = BTreeMap::new();

    for row in &input.rows {
        if row.status == EvaluationStatus::NotApplicable {
            continue;
        }
        let is_pass = row.status.is_pass();
        // Emit to every stratum this row participates in.
        let strata = [
            Stratum {
                benchmark_id: None,
                agent_id: None,
            },
            Stratum {
                benchmark_id: Some(row.benchmark_id),
                agent_id: None,
            },
            Stratum {
                benchmark_id: None,
                agent_id: Some(row.agent_id.clone()),
            },
            Stratum {
                benchmark_id: Some(row.benchmark_id),
                agent_id: Some(row.agent_id.clone()),
            },
        ];
        for s in strata {
            let entry = buckets.entry((s, row.level)).or_default();
            entry.evaluated += 1;
            if is_pass {
                entry.passed += 1;
            }
        }
    }

    buckets
        .into_iter()
        .map(|((stratum, level), counts)| {
            let pass_rate = if counts.evaluated == 0 {
                None
            } else {
                #[allow(clippy::cast_precision_loss)]
                let r = counts.passed as f64 / counts.evaluated as f64;
                Some(r)
            };
            ScoreDescentRow {
                stratum,
                level,
                passed: counts.passed,
                evaluated: counts.evaluated,
                pass_rate,
            }
        })
        .collect()
}

/// Row of the conditional false-success table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalFalseSuccessRow {
    /// Lower level.
    pub level_from: EvaluationLevel,
    /// Higher level.
    pub level_to: EvaluationLevel,
    /// Count of candidates that passed `level_from`.
    pub n_passed_from: u64,
    /// Count of those that then failed `level_to`.
    pub n_failed_to: u64,
    /// `n_failed_to / n_passed_from`. `None` if the denominator is zero.
    pub rate: Option<f64>,
}

/// Conditional false-success rate `P(fail at L_{k+1} | pass at L_k)` for
/// every adjacent pair of levels.
///
/// Pairs are formed from each candidate's per-level verdicts. Levels missing
/// for a candidate are ignored; `NotApplicable` is treated as "no verdict".
#[must_use]
pub fn conditional_false_success(input: &AnalysisInput) -> Vec<ConditionalFalseSuccessRow> {
    // Group by candidate.
    let mut per_candidate: BTreeMap<
        eval_ladder_core::CandidateId,
        BTreeMap<EvaluationLevel, EvaluationStatus>,
    > = BTreeMap::new();
    for row in &input.rows {
        per_candidate
            .entry(row.candidate_id)
            .or_default()
            .insert(row.level, row.status);
    }

    // Adjacent level pairs in ladder order.
    let pairs = [
        (EvaluationLevel::L0Official, EvaluationLevel::L1TrustedRerun),
        (
            EvaluationLevel::L1TrustedRerun,
            EvaluationLevel::L2Strengthened,
        ),
        (
            EvaluationLevel::L2Strengthened,
            EvaluationLevel::L3PolicyConformant,
        ),
        (
            EvaluationLevel::L3PolicyConformant,
            EvaluationLevel::L4Semantic,
        ),
    ];

    pairs
        .into_iter()
        .map(|(lo, hi)| {
            let mut n_passed_from = 0_u64;
            let mut n_failed_to = 0_u64;
            for verdicts in per_candidate.values() {
                let Some(&s_lo) = verdicts.get(&lo) else {
                    continue;
                };
                let Some(&s_hi) = verdicts.get(&hi) else {
                    continue;
                };
                if s_lo == EvaluationStatus::Pass {
                    n_passed_from += 1;
                    if s_hi == EvaluationStatus::Fail {
                        n_failed_to += 1;
                    }
                }
            }
            let rate = if n_passed_from == 0 {
                None
            } else {
                #[allow(clippy::cast_precision_loss)]
                let r = n_failed_to as f64 / n_passed_from as f64;
                Some(r)
            };
            ConditionalFalseSuccessRow {
                level_from: lo,
                level_to: hi,
                n_passed_from,
                n_failed_to,
                rate,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_ladder_core::{CandidateId, TaskId};

    use crate::input::AnalysisInputRow;

    fn row(
        candidate: CandidateId,
        agent: &str,
        bench: BenchmarkId,
        level: EvaluationLevel,
        status: EvaluationStatus,
    ) -> AnalysisInputRow {
        AnalysisInputRow {
            candidate_id: candidate,
            task_id: TaskId::new("t").unwrap(),
            benchmark_id: bench,
            agent_id: agent.to_owned(),
            model_id: "m".into(),
            level,
            status,
            primary_reason: "X".into(),
            task_category: None,
        }
    }

    #[test]
    fn score_descent_computes_pooled_pass_rate() {
        let c1 = CandidateId::new_v4();
        let c2 = CandidateId::new_v4();
        let rows = vec![
            row(
                c1,
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L0Official,
                EvaluationStatus::Pass,
            ),
            row(
                c2,
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L0Official,
                EvaluationStatus::Fail,
            ),
        ];
        let input = AnalysisInput { rows };
        let table = score_descent(&input);
        let pooled = table
            .iter()
            .find(|r| {
                r.stratum.benchmark_id.is_none()
                    && r.stratum.agent_id.is_none()
                    && r.level == EvaluationLevel::L0Official
            })
            .unwrap();
        assert_eq!(pooled.passed, 1);
        assert_eq!(pooled.evaluated, 2);
        assert!((pooled.pass_rate.unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn conditional_false_success_detects_l2_drop() {
        let c1 = CandidateId::new_v4();
        let c2 = CandidateId::new_v4();
        let c3 = CandidateId::new_v4();
        let rows = vec![
            row(
                c1,
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L1TrustedRerun,
                EvaluationStatus::Pass,
            ),
            row(
                c1,
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L2Strengthened,
                EvaluationStatus::Fail,
            ),
            row(
                c2,
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L1TrustedRerun,
                EvaluationStatus::Pass,
            ),
            row(
                c2,
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L2Strengthened,
                EvaluationStatus::Pass,
            ),
            row(
                c3,
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L1TrustedRerun,
                EvaluationStatus::Fail,
            ),
            row(
                c3,
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L2Strengthened,
                EvaluationStatus::Pass,
            ),
        ];
        let input = AnalysisInput { rows };
        let table = conditional_false_success(&input);
        let l1_l2 = table
            .iter()
            .find(|r| {
                r.level_from == EvaluationLevel::L1TrustedRerun
                    && r.level_to == EvaluationLevel::L2Strengthened
            })
            .unwrap();
        assert_eq!(l1_l2.n_passed_from, 2);
        assert_eq!(l1_l2.n_failed_to, 1);
        assert!((l1_l2.rate.unwrap() - 0.5).abs() < 1e-12);
    }
}
