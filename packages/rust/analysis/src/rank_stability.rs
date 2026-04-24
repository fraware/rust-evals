//! Rank-stability analysis: Kendall tau-b between agent leaderboards.
//!
//! At each level we rank agents by their pooled pass rate over all
//! candidates evaluated at that level. We then report tau-b between every
//! pair of levels that share agents.

use std::collections::BTreeMap;

use eval_ladder_core::{EvaluationLevel, EvaluationStatus};
use serde::{Deserialize, Serialize};

use crate::input::{project_analysis_mode, AnalysisInput, AnalysisMode};

/// Kendall tau-b between agent leaderboards at two levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankStabilityRow {
    /// First level.
    pub level_a: EvaluationLevel,
    /// Second level.
    pub level_b: EvaluationLevel,
    /// Number of agents shared between the two leaderboards.
    pub n_agents: u32,
    /// Kendall tau-b. `None` if fewer than two agents are shared.
    pub kendall_tau_b: Option<f64>,
}

/// Compute pairwise Kendall tau-b across all levels that appear in `input`.
#[must_use]
pub fn rank_stability(input: &AnalysisInput, mode: AnalysisMode) -> Vec<RankStabilityRow> {
    let input = project_analysis_mode(input, mode);
    // agent -> level -> (passed, evaluated)
    #[derive(Default)]
    struct C {
        passed: u64,
        evaluated: u64,
    }
    let mut agg: BTreeMap<String, BTreeMap<EvaluationLevel, C>> = BTreeMap::new();
    for row in &input.rows {
        if row.status == EvaluationStatus::NotApplicable {
            continue;
        }
        let entry = agg
            .entry(row.agent_id.clone())
            .or_default()
            .entry(row.level)
            .or_default();
        entry.evaluated += 1;
        if row.status.is_pass() {
            entry.passed += 1;
        }
    }

    // Determine levels observed.
    let mut levels: std::collections::BTreeSet<EvaluationLevel> = std::collections::BTreeSet::new();
    for per_level in agg.values() {
        for lvl in per_level.keys() {
            levels.insert(*lvl);
        }
    }
    let levels: Vec<EvaluationLevel> = levels.into_iter().collect();

    let mut out = Vec::new();
    for (i, a) in levels.iter().enumerate() {
        for b in &levels[i + 1..] {
            let mut scores_a: Vec<f64> = Vec::new();
            let mut scores_b: Vec<f64> = Vec::new();
            for per_level in agg.values() {
                let Some(ca) = per_level.get(a) else { continue };
                let Some(cb) = per_level.get(b) else { continue };
                if ca.evaluated == 0 || cb.evaluated == 0 {
                    continue;
                }
                #[allow(clippy::cast_precision_loss)]
                let sa = ca.passed as f64 / ca.evaluated as f64;
                #[allow(clippy::cast_precision_loss)]
                let sb = cb.passed as f64 / cb.evaluated as f64;
                scores_a.push(sa);
                scores_b.push(sb);
            }
            #[allow(clippy::cast_possible_truncation)]
            let n = scores_a.len() as u32;
            let tau = kendall_tau_b(&scores_a, &scores_b);
            out.push(RankStabilityRow {
                level_a: *a,
                level_b: *b,
                n_agents: n,
                kendall_tau_b: tau,
            });
        }
    }
    out
}

/// Kendall tau-b with ties correction. Returns `None` when the denominator is
/// zero (for example, fewer than two distinct values on either axis).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn kendall_tau_b(x: &[f64], y: &[f64]) -> Option<f64> {
    debug_assert_eq!(x.len(), y.len());
    let n = x.len();
    if n < 2 {
        return None;
    }
    let mut concordant: u64 = 0;
    let mut discordant: u64 = 0;
    let mut tied_x: u64 = 0;
    let mut tied_y: u64 = 0;
    for i in 0..n {
        for j in (i + 1)..n {
            let dx = x[j] - x[i];
            let dy = y[j] - y[i];
            match (dx.partial_cmp(&0.0)?, dy.partial_cmp(&0.0)?) {
                (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal) => {
                    tied_x += 1;
                    tied_y += 1;
                }
                (std::cmp::Ordering::Equal, _) => tied_x += 1,
                (_, std::cmp::Ordering::Equal) => tied_y += 1,
                (a, b) if a == b => concordant += 1,
                _ => discordant += 1,
            }
        }
    }
    // tau_b = (C - D) / sqrt((C + D + T_x) * (C + D + T_y))
    let cd = concordant as f64 - discordant as f64;
    let denom_x = (concordant + discordant + tied_x) as f64;
    let denom_y = (concordant + discordant + tied_y) as f64;
    if denom_x == 0.0 || denom_y == 0.0 {
        return None;
    }
    Some(cd / (denom_x * denom_y).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{AnalysisInput, AnalysisInputRow, AnalysisMode};
    use eval_ladder_core::{BenchmarkId, CandidateId, TaskId};

    #[test]
    fn perfect_agreement_gives_tau_b_one() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [0.1, 0.2, 0.3, 0.4];
        let tau = kendall_tau_b(&x, &y).unwrap();
        assert!((tau - 1.0).abs() < 1e-12);
    }

    #[test]
    fn perfect_disagreement_gives_tau_b_minus_one() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [0.4, 0.3, 0.2, 0.1];
        let tau = kendall_tau_b(&x, &y).unwrap();
        assert!((tau + 1.0).abs() < 1e-12);
    }

    #[test]
    fn cumulative_mode_reorders_upper_level_agent_leaderboard() {
        let mut rows = Vec::new();
        let mut push = |agent: &str, task: &str, l0: EvaluationStatus, l3: EvaluationStatus| {
            let cid = CandidateId::new_v4();
            rows.push(AnalysisInputRow {
                candidate_id: cid,
                task_id: TaskId::new(task).unwrap(),
                benchmark_id: BenchmarkId::RustSweBench,
                agent_id: agent.into(),
                model_id: "m".into(),
                level: EvaluationLevel::L0Official,
                status: l0,
                primary_reason: "X".into(),
                task_category: None,
            });
            rows.push(AnalysisInputRow {
                candidate_id: cid,
                task_id: TaskId::new(task).unwrap(),
                benchmark_id: BenchmarkId::RustSweBench,
                agent_id: agent.into(),
                model_id: "m".into(),
                level: EvaluationLevel::L3PolicyConformant,
                status: l3,
                primary_reason: "X".into(),
                task_category: None,
            });
        };
        // agent-a: pathological upper passes despite lower invalids.
        push(
            "agent-a",
            "a1",
            EvaluationStatus::Invalid,
            EvaluationStatus::Pass,
        );
        push(
            "agent-a",
            "a2",
            EvaluationStatus::Invalid,
            EvaluationStatus::Pass,
        );
        // agent-b: clean strong performer.
        push(
            "agent-b",
            "b1",
            EvaluationStatus::Pass,
            EvaluationStatus::Pass,
        );
        push(
            "agent-b",
            "b2",
            EvaluationStatus::Pass,
            EvaluationStatus::Fail,
        );
        // agent-c: middling on L0, weak on L3.
        push(
            "agent-c",
            "c1",
            EvaluationStatus::Pass,
            EvaluationStatus::Fail,
        );
        push(
            "agent-c",
            "c2",
            EvaluationStatus::Fail,
            EvaluationStatus::Fail,
        );
        let input = AnalysisInput { rows };
        let raw = rank_stability(&input, AnalysisMode::Raw);
        let cum = rank_stability(&input, AnalysisMode::Cumulative);

        let raw_l0_l3 = raw
            .iter()
            .find(|r| {
                r.level_a == EvaluationLevel::L0Official
                    && r.level_b == EvaluationLevel::L3PolicyConformant
            })
            .unwrap();
        assert!(
            raw_l0_l3.kendall_tau_b.is_some(),
            "fixture should yield a defined tau in raw mode"
        );
        let cum_l0_l3 = cum
            .iter()
            .find(|r| {
                r.level_a == EvaluationLevel::L0Official
                    && r.level_b == EvaluationLevel::L3PolicyConformant
            })
            .unwrap();
        assert_ne!(raw_l0_l3.kendall_tau_b, cum_l0_l3.kendall_tau_b);
    }
}
