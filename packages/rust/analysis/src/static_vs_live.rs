//! Static-vs-live comparison table.
//!
//! Milestone L.
//!
//! This module compares an agent's pass rate on the *static* benchmark
//! family (canonically [`BenchmarkId::SweBenchVerified`]) against the
//! *live* benchmark family ([`BenchmarkId::SweBenchLive`]) at every
//! evaluation level that both suites reach. The table is the headline
//! paper artifact for the claim
//!
//! > Official coding-agent benchmark scores overstate semantically
//! > justified issue resolution; a trusted evaluator [...] reveal[s]
//! > the size and structure of that overstatement.
//!
//! Decisions locked in here:
//!
//! - **Which benchmarks count as "static" / "live"**. This module
//!   defines static and live explicitly at the module level so the
//!   paper and the code never drift. `SweBenchVerified` is static;
//!   `SweBenchLive` is live; `RustSweBench` is *neither* and is
//!   deliberately excluded - it is a separate reproduction surface
//!   for the paper, not part of this comparison.
//! - **Aggregation shape**. One row per `(agent_id, level)`. Pooled
//!   (all-agents) rows are not emitted: pooling across agents loses
//!   the per-agent asymmetry we care about; callers that want a
//!   pooled number can derive it trivially from the rows.
//! - **Zero-denominator policy**. If an agent has zero evaluated
//!   candidates on either suite at a given level, the row is still
//!   emitted with explicit `None` rates so the paper table can show
//!   "no data" rather than silently dropping the row.
//! - **Determinism**. Rows are sorted by `(agent_id, level)` before
//!   being returned; every numeric field is derived from integer
//!   counts; no clocks are observed. Re-running the analyzer on the
//!   same [`AnalysisInput`] produces byte-identical output.

use std::collections::BTreeMap;

use eval_ladder_core::{BenchmarkId, EvaluationLevel, EvaluationStatus};
use serde::{Deserialize, Serialize};

use crate::input::AnalysisInput;

/// Benchmarks treated as the "static" arm of the comparison.
pub const STATIC_BENCHMARKS: &[BenchmarkId] = &[BenchmarkId::SweBenchVerified];

/// Benchmarks treated as the "live" arm of the comparison.
pub const LIVE_BENCHMARKS: &[BenchmarkId] = &[BenchmarkId::SweBenchLive];

/// Which arm a given `BenchmarkId` maps into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arm {
    Static,
    Live,
    Excluded,
}

fn classify(benchmark_id: BenchmarkId) -> Arm {
    if STATIC_BENCHMARKS.contains(&benchmark_id) {
        Arm::Static
    } else if LIVE_BENCHMARKS.contains(&benchmark_id) {
        Arm::Live
    } else {
        Arm::Excluded
    }
}

/// One row of the static-vs-live comparison table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticVsLiveRow {
    /// Agent identifier.
    pub agent_id: String,
    /// Evaluation level.
    pub level: EvaluationLevel,
    /// Candidates on the static suite that passed at this level.
    pub static_passed: u64,
    /// Candidates on the static suite evaluated at this level,
    /// excluding `NotApplicable`.
    pub static_evaluated: u64,
    /// `static_passed / static_evaluated`; `None` when the denominator
    /// is zero.
    pub static_pass_rate: Option<f64>,
    /// Candidates on the live suite that passed at this level.
    pub live_passed: u64,
    /// Candidates on the live suite evaluated at this level, excluding
    /// `NotApplicable`.
    pub live_evaluated: u64,
    /// `live_passed / live_evaluated`; `None` when the denominator is
    /// zero.
    pub live_pass_rate: Option<f64>,
    /// `live_pass_rate - static_pass_rate`; `None` whenever either
    /// individual rate is `None`. A negative delta is the paper's
    /// quantitative expression of the "overstatement" claim.
    pub delta: Option<f64>,
    /// `live_pass_rate / static_pass_rate`; `None` when either
    /// individual rate is `None` or the static rate is zero (no
    /// meaningful relative comparison).
    pub ratio: Option<f64>,
}

/// Compute the static-vs-live comparison table.
///
/// See the module-level documentation for contract and determinism
/// guarantees.
#[must_use]
pub fn static_vs_live(input: &AnalysisInput) -> Vec<StaticVsLiveRow> {
    #[derive(Default, Clone, Copy)]
    struct Counts {
        static_passed: u64,
        static_evaluated: u64,
        live_passed: u64,
        live_evaluated: u64,
    }

    let mut buckets: BTreeMap<(String, EvaluationLevel), Counts> = BTreeMap::new();

    for row in &input.rows {
        if row.status == EvaluationStatus::NotApplicable {
            continue;
        }
        let arm = classify(row.benchmark_id);
        if arm == Arm::Excluded {
            continue;
        }
        let is_pass = row.status.is_pass();
        let entry = buckets
            .entry((row.agent_id.clone(), row.level))
            .or_default();
        match arm {
            Arm::Static => {
                entry.static_evaluated += 1;
                if is_pass {
                    entry.static_passed += 1;
                }
            }
            Arm::Live => {
                entry.live_evaluated += 1;
                if is_pass {
                    entry.live_passed += 1;
                }
            }
            Arm::Excluded => unreachable!("excluded arm handled above"),
        }
    }

    buckets
        .into_iter()
        .map(|((agent_id, level), counts)| {
            let static_pass_rate = rate(counts.static_passed, counts.static_evaluated);
            let live_pass_rate = rate(counts.live_passed, counts.live_evaluated);
            let delta = match (static_pass_rate, live_pass_rate) {
                (Some(s), Some(l)) => Some(l - s),
                _ => None,
            };
            let ratio = match (static_pass_rate, live_pass_rate) {
                (Some(s), Some(l)) if s > 0.0 => Some(l / s),
                _ => None,
            };
            StaticVsLiveRow {
                agent_id,
                level,
                static_passed: counts.static_passed,
                static_evaluated: counts.static_evaluated,
                static_pass_rate,
                live_passed: counts.live_passed,
                live_evaluated: counts.live_evaluated,
                live_pass_rate,
                delta,
                ratio,
            }
        })
        .collect()
}

#[inline]
fn rate(passed: u64, evaluated: u64) -> Option<f64> {
    if evaluated == 0 {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        let r = passed as f64 / evaluated as f64;
        Some(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::AnalysisInputRow;
    use eval_ladder_core::{BenchmarkId, CandidateId, EvaluationLevel, EvaluationStatus, TaskId};

    fn row(
        agent: &str,
        bench: BenchmarkId,
        level: EvaluationLevel,
        status: EvaluationStatus,
    ) -> AnalysisInputRow {
        AnalysisInputRow {
            candidate_id: CandidateId::new_v4(),
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
    fn static_vs_live_classifies_benchmarks_and_computes_rates() {
        let rows = vec![
            // Agent a: static 2/3 pass @ L0, live 1/4 pass @ L0.
            row(
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L0Official,
                EvaluationStatus::Pass,
            ),
            row(
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L0Official,
                EvaluationStatus::Pass,
            ),
            row(
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L0Official,
                EvaluationStatus::Fail,
            ),
            row(
                "a",
                BenchmarkId::SweBenchLive,
                EvaluationLevel::L0Official,
                EvaluationStatus::Pass,
            ),
            row(
                "a",
                BenchmarkId::SweBenchLive,
                EvaluationLevel::L0Official,
                EvaluationStatus::Fail,
            ),
            row(
                "a",
                BenchmarkId::SweBenchLive,
                EvaluationLevel::L0Official,
                EvaluationStatus::Fail,
            ),
            row(
                "a",
                BenchmarkId::SweBenchLive,
                EvaluationLevel::L0Official,
                EvaluationStatus::Fail,
            ),
            // RustSweBench is neither static nor live: must be excluded.
            row(
                "a",
                BenchmarkId::RustSweBench,
                EvaluationLevel::L0Official,
                EvaluationStatus::Pass,
            ),
            // NotApplicable must be excluded.
            row(
                "a",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L2Strengthened,
                EvaluationStatus::NotApplicable,
            ),
        ];
        let input = AnalysisInput { rows };
        let table = static_vs_live(&input);
        assert_eq!(table.len(), 1, "one row per (agent, level) with data");
        let r = &table[0];
        assert_eq!(r.agent_id, "a");
        assert_eq!(r.level, EvaluationLevel::L0Official);
        assert_eq!(r.static_passed, 2);
        assert_eq!(r.static_evaluated, 3);
        assert_eq!(r.live_passed, 1);
        assert_eq!(r.live_evaluated, 4);
        let static_rate = r.static_pass_rate.unwrap();
        let live_rate = r.live_pass_rate.unwrap();
        assert!((static_rate - 2.0 / 3.0).abs() < 1e-12);
        assert!((live_rate - 0.25).abs() < 1e-12);
        assert!((r.delta.unwrap() - (live_rate - static_rate)).abs() < 1e-12);
        assert!((r.ratio.unwrap() - (live_rate / static_rate)).abs() < 1e-12);
        assert!(
            r.delta.unwrap() < 0.0,
            "paper claim: live should be below static"
        );
    }

    #[test]
    fn static_vs_live_handles_missing_arm_without_panic() {
        // Agent b has data on static only at L1.
        let rows = vec![row(
            "b",
            BenchmarkId::SweBenchVerified,
            EvaluationLevel::L1TrustedRerun,
            EvaluationStatus::Pass,
        )];
        let input = AnalysisInput { rows };
        let table = static_vs_live(&input);
        assert_eq!(table.len(), 1);
        let r = &table[0];
        assert_eq!(r.static_evaluated, 1);
        assert_eq!(r.live_evaluated, 0);
        assert!(r.static_pass_rate.is_some());
        assert!(r.live_pass_rate.is_none());
        assert!(r.delta.is_none());
        assert!(r.ratio.is_none());
    }

    #[test]
    fn static_vs_live_ratio_none_when_static_rate_is_zero() {
        let rows = vec![
            row(
                "c",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L0Official,
                EvaluationStatus::Fail,
            ),
            row(
                "c",
                BenchmarkId::SweBenchLive,
                EvaluationLevel::L0Official,
                EvaluationStatus::Pass,
            ),
        ];
        let input = AnalysisInput { rows };
        let table = static_vs_live(&input);
        let r = &table[0];
        assert_eq!(r.static_pass_rate, Some(0.0));
        assert_eq!(r.live_pass_rate, Some(1.0));
        assert_eq!(r.delta, Some(1.0));
        assert!(
            r.ratio.is_none(),
            "ratio is undefined when static rate is zero"
        );
    }

    #[test]
    fn static_vs_live_sorts_rows_by_agent_then_level() {
        let rows = vec![
            row(
                "zeta",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L1TrustedRerun,
                EvaluationStatus::Pass,
            ),
            row(
                "alpha",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L2Strengthened,
                EvaluationStatus::Pass,
            ),
            row(
                "alpha",
                BenchmarkId::SweBenchVerified,
                EvaluationLevel::L0Official,
                EvaluationStatus::Pass,
            ),
        ];
        let input = AnalysisInput { rows };
        let table = static_vs_live(&input);
        let keys: Vec<(String, EvaluationLevel)> = table
            .iter()
            .map(|r| (r.agent_id.clone(), r.level))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("alpha".into(), EvaluationLevel::L0Official),
                ("alpha".into(), EvaluationLevel::L2Strengthened),
                ("zeta".into(), EvaluationLevel::L1TrustedRerun),
            ],
            "rows must be sorted by (agent_id, level)"
        );
    }
}
