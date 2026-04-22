//! Milestone G acceptance: evidence bundles -> paper-ready exports.
//!
//! The acceptance contract has three parts:
//!
//! 1. `milestone_g_bundles_load_into_analysis_input` - a directory of
//!    per-candidate evidence bundles loads into a canonical
//!    [`AnalysisInput`] whose rows are sorted in ladder order per
//!    candidate.
//! 2. `milestone_g_paper_export_is_deterministic` - writing the same
//!    input twice into two empty directories produces byte-identical
//!    CSV/JSON bodies and a byte-identical `manifest.json`. This
//!    carries forward the Milestone C determinism invariant from
//!    evidence bundles into the paper pipeline.
//! 3. `milestone_g_conditional_false_success_sees_l2_drop` - the
//!    headline scientific finding ("L2 strengthening collapses a
//!    fraction of L1 passes") shows up in the shipped paper tables,
//!    end-to-end.
//!
//! No pipeline, no runner, no containers; Milestone G is a pure
//! function over bundle bytes, which is exactly what this test pins.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};
use eval_ladder_analysis::{
    load_bundle_dir,
    paper_export::{write_paper_exports, PaperExportManifest},
    score_descent::conditional_false_success,
    LoadOptions, CANDIDATE_RESOLUTION_FILE,
};
use eval_ladder_core::{
    BenchmarkId, CandidateId, CandidateResolution, ContextMode, EvaluationLevel, EvaluationResult,
    EvaluationStatus, GenerationMetadata, GenerationMode, PatchFormat, TaskId, EVALUATOR_VERSION,
    SCHEMA_VERSION,
};
use tempfile::TempDir;

/// Canonical file names copied from the evidence bundle spec; the test
/// purposefully avoids importing them from each extension crate so it
/// exercises the analysis loader's own contract.
const OFFICIAL: &str = "official_results.json";
const L1: &str = "l1_trusted_rerun_results.json";
const L2: &str = "strengthened_results.json";

fn candidate(task: &str, agent: &str) -> CandidateResolution {
    CandidateResolution {
        schema_version: SCHEMA_VERSION,
        candidate_id: CandidateId::new_v4(),
        benchmark_id: BenchmarkId::SweBenchVerified,
        task_id: TaskId::new(task).unwrap(),
        agent_id: agent.into(),
        model_id: "fixture-model".into(),
        generation_mode: GenerationMode::SingleShot,
        patch_format: PatchFormat::UnifiedDiff,
        patch_ref: "fixture://patch.diff".into(),
        trajectory_ref: None,
        generation_metadata: GenerationMetadata {
            temperature: Some(0.0),
            tool_configuration: serde_json::Value::Object(serde_json::Map::new()),
            context_mode: ContextMode::FileLevel,
            repo_reproduction_used: false,
            random_seed: None,
        },
        submitted_at: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
    }
}

fn result(
    c: &CandidateResolution,
    level: EvaluationLevel,
    status: EvaluationStatus,
    code: &str,
) -> EvaluationResult {
    EvaluationResult {
        schema_version: SCHEMA_VERSION,
        candidate_id: c.candidate_id,
        task_id: c.task_id.clone(),
        level,
        status,
        primary_reason: code.into(),
        secondary_reasons: vec![],
        artifacts: vec![],
        metrics: serde_json::Value::Null,
        started_at: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
        finished_at: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 1).unwrap(),
        evaluator_version: EVALUATOR_VERSION,
    }
}

fn write_json(path: &Path, value: &serde_json::Value) {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap();
}

fn seed_bundle(
    root: &Path,
    bundle_name: &str,
    c: &CandidateResolution,
    per_level: &[(&str, EvaluationResult)],
) -> PathBuf {
    let dir = root.join(bundle_name);
    fs::create_dir_all(&dir).unwrap();
    write_json(
        &dir.join(CANDIDATE_RESOLUTION_FILE),
        &serde_json::to_value(c).unwrap(),
    );
    for (name, r) in per_level {
        write_json(&dir.join(name), &serde_json::to_value(r).unwrap());
    }
    dir
}

/// Seed the run directory used by every Milestone G test.
///
/// The scenario mirrors the motivating finding from the paper:
/// two agents ship three tasks, most L1 passes carry through L2, but
/// one L1 pass from each agent collapses at L2 ("L2 strengthening
/// bites"). A further task has L0 pass but no L1/L2 (ladder-incomplete
/// bundle) which the loader must tolerate row-level rather than bail.
fn seed_run_dir() -> TempDir {
    let tmp = TempDir::new().unwrap();

    let a1 = candidate("task-1", "agent-a");
    seed_bundle(
        tmp.path(),
        "a-task-1",
        &a1,
        &[
            (
                OFFICIAL,
                result(
                    &a1,
                    EvaluationLevel::L0Official,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            ),
            (
                L1,
                result(
                    &a1,
                    EvaluationLevel::L1TrustedRerun,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            ),
            (
                L2,
                result(
                    &a1,
                    EvaluationLevel::L2Strengthened,
                    EvaluationStatus::Fail,
                    "L2_DIFF_BEHAVIOR",
                ),
            ),
        ],
    );
    let a2 = candidate("task-2", "agent-a");
    seed_bundle(
        tmp.path(),
        "a-task-2",
        &a2,
        &[
            (
                OFFICIAL,
                result(
                    &a2,
                    EvaluationLevel::L0Official,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            ),
            (
                L1,
                result(
                    &a2,
                    EvaluationLevel::L1TrustedRerun,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            ),
            (
                L2,
                result(
                    &a2,
                    EvaluationLevel::L2Strengthened,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            ),
        ],
    );
    let b1 = candidate("task-1", "agent-b");
    seed_bundle(
        tmp.path(),
        "b-task-1",
        &b1,
        &[
            (
                OFFICIAL,
                result(
                    &b1,
                    EvaluationLevel::L0Official,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            ),
            (
                L1,
                result(
                    &b1,
                    EvaluationLevel::L1TrustedRerun,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            ),
            (
                L2,
                result(
                    &b1,
                    EvaluationLevel::L2Strengthened,
                    EvaluationStatus::Fail,
                    "L2_AUGMENTED_FAIL",
                ),
            ),
        ],
    );
    let b2 = candidate("task-3", "agent-b");
    seed_bundle(
        tmp.path(),
        "b-task-3",
        &b2,
        &[(
            OFFICIAL,
            result(
                &b2,
                EvaluationLevel::L0Official,
                EvaluationStatus::Pass,
                "PASS",
            ),
        )],
    );
    tmp
}

#[test]
fn milestone_g_bundles_load_into_analysis_input() {
    let run = seed_run_dir();
    let input = load_bundle_dir(run.path(), &LoadOptions::default()).unwrap();
    assert_eq!(
        input.rows.len(),
        3 * 3 + 1,
        "expected 3 bundles with 3 levels and 1 bundle with L0 only"
    );
    // Every candidate's rows must be contiguous and sorted by ladder index.
    let mut by_candidate: BTreeMap<CandidateId, Vec<EvaluationLevel>> = BTreeMap::new();
    for row in &input.rows {
        by_candidate
            .entry(row.candidate_id)
            .or_default()
            .push(row.level);
    }
    for (_, levels) in by_candidate {
        let mut sorted = levels.clone();
        sorted.sort();
        assert_eq!(
            levels, sorted,
            "per-candidate level ordering must be stable"
        );
    }
}

#[test]
fn milestone_g_paper_export_is_deterministic() {
    let run = seed_run_dir();
    let out_a = TempDir::new().unwrap();
    let out_b = TempDir::new().unwrap();

    let input = load_bundle_dir(run.path(), &LoadOptions::default()).unwrap();
    let manifest_a: PaperExportManifest = write_paper_exports(&input, out_a.path()).unwrap();
    let manifest_b: PaperExportManifest = write_paper_exports(&input, out_b.path()).unwrap();

    assert_eq!(
        manifest_a, manifest_b,
        "paper-export manifest must be deterministic"
    );
    for f in &manifest_a.files {
        let bytes_a = fs::read(out_a.path().join(&f.path)).unwrap();
        let bytes_b = fs::read(out_b.path().join(&f.path)).unwrap();
        assert_eq!(
            bytes_a, bytes_b,
            "paper-export file {} must be byte-identical across runs",
            f.path
        );
    }

    let manifest_bytes_a = fs::read(out_a.path().join("manifest.json")).unwrap();
    let manifest_bytes_b = fs::read(out_b.path().join("manifest.json")).unwrap();
    assert_eq!(
        manifest_bytes_a, manifest_bytes_b,
        "paper-export manifest.json must be byte-identical across runs"
    );
}

#[test]
fn milestone_g_conditional_false_success_sees_l2_drop() {
    let run = seed_run_dir();
    let input = load_bundle_dir(run.path(), &LoadOptions::default()).unwrap();
    let table = conditional_false_success(&input);
    let l1_l2 = table
        .iter()
        .find(|r| {
            r.level_from == EvaluationLevel::L1TrustedRerun
                && r.level_to == EvaluationLevel::L2Strengthened
        })
        .unwrap();
    assert_eq!(l1_l2.n_passed_from, 3, "three candidates pass L1");
    assert_eq!(l1_l2.n_failed_to, 2, "two collapse at L2");
    assert!(
        (l1_l2.rate.unwrap() - (2.0 / 3.0)).abs() < 1e-12,
        "P(fail L2 | pass L1) must equal 2/3 on the fixture"
    );
}
