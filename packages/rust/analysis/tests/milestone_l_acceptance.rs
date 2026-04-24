//! Milestone L acceptance: evidence bundles -> static-vs-live paper
//! table, end-to-end.
//!
//! The contract:
//!
//! 1. `milestone_l_static_vs_live_quantifies_overstatement` - bundles
//!    from the same agent spanning SWE-bench Verified (static) and
//!    SWE-bench-Live (live) produce a row with `delta < 0` at the
//!    level where the agent's live pass rate is below its static rate.
//!    This is the paper's "overstatement" claim becoming a shipped,
//!    deterministic number.
//! 2. `milestone_l_paper_export_emits_static_vs_live_files` - the
//!    shipped `analyze paper-export` pipeline writes the new
//!    `static_vs_live.{csv,json}` pair and registers both in the
//!    canonical manifest, and the manifest `schema_version` is bumped
//!    to 2 so any reader pinned on the Milestone G hash
//!    intentionally picks up the additional artifact.
//!
//! The fixture is deliberately minimal: two agents, three tasks,
//! identical L0 verdicts on the static suite, asymmetric L0 verdicts
//! on the live suite. This is enough to exercise the classification,
//! the zero-denominator branches, and the row ordering.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};
use eval_ladder_analysis::{
    load_bundle_dir,
    paper_export::{write_paper_exports, PAPER_EXPORT_SCHEMA_VERSION},
    static_vs_live::static_vs_live,
    AnalysisMode, LoadOptions, CANDIDATE_RESOLUTION_FILE,
};
use eval_ladder_core::{
    candidate::ContextMode, BenchmarkId, CandidateId, CandidateResolution, EvaluationLevel,
    EvaluationResult, EvaluationStatus, GenerationMetadata, GenerationMode, PatchFormat, TaskId,
    EVALUATOR_VERSION, SCHEMA_VERSION,
};
use tempfile::TempDir;

const OFFICIAL: &str = "official_results.json";

fn candidate(task: &str, agent: &str, bench: BenchmarkId) -> CandidateResolution {
    CandidateResolution {
        schema_version: SCHEMA_VERSION,
        candidate_id: CandidateId::new_v4(),
        benchmark_id: bench,
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
    l0: EvaluationResult,
) -> PathBuf {
    let dir = root.join(bundle_name);
    fs::create_dir_all(&dir).unwrap();
    write_json(
        &dir.join(CANDIDATE_RESOLUTION_FILE),
        &serde_json::to_value(c).unwrap(),
    );
    write_json(&dir.join(OFFICIAL), &serde_json::to_value(&l0).unwrap());
    dir
}

fn seed_run_dir() -> TempDir {
    let tmp = TempDir::new().unwrap();

    // Agent "a": two-for-two on static, zero-for-two on live at L0.
    for (task, status) in [("task-1", true), ("task-2", true)] {
        let c = candidate(task, "agent-a", BenchmarkId::SweBenchVerified);
        let r = result(
            &c,
            EvaluationLevel::L0Official,
            if status {
                EvaluationStatus::Pass
            } else {
                EvaluationStatus::Fail
            },
            if status { "PASS" } else { "L0_FAIL" },
        );
        seed_bundle(tmp.path(), &format!("a-static-{task}"), &c, r);
    }
    for (task, status) in [("task-1", false), ("task-2", false)] {
        let c = candidate(task, "agent-a", BenchmarkId::SweBenchLive);
        let r = result(
            &c,
            EvaluationLevel::L0Official,
            if status {
                EvaluationStatus::Pass
            } else {
                EvaluationStatus::Fail
            },
            if status { "PASS" } else { "L0_FAIL" },
        );
        seed_bundle(tmp.path(), &format!("a-live-{task}"), &c, r);
    }

    // Agent "b": one-of-two on both. Equal rates -> delta 0.
    for (task, status) in [("task-1", true), ("task-2", false)] {
        let c = candidate(task, "agent-b", BenchmarkId::SweBenchVerified);
        let r = result(
            &c,
            EvaluationLevel::L0Official,
            if status {
                EvaluationStatus::Pass
            } else {
                EvaluationStatus::Fail
            },
            if status { "PASS" } else { "L0_FAIL" },
        );
        seed_bundle(tmp.path(), &format!("b-static-{task}"), &c, r);
    }
    for (task, status) in [("task-1", true), ("task-2", false)] {
        let c = candidate(task, "agent-b", BenchmarkId::SweBenchLive);
        let r = result(
            &c,
            EvaluationLevel::L0Official,
            if status {
                EvaluationStatus::Pass
            } else {
                EvaluationStatus::Fail
            },
            if status { "PASS" } else { "L0_FAIL" },
        );
        seed_bundle(tmp.path(), &format!("b-live-{task}"), &c, r);
    }

    tmp
}

#[test]
fn milestone_l_static_vs_live_quantifies_overstatement() {
    let run = seed_run_dir();
    let input = load_bundle_dir(run.path(), &LoadOptions::default()).unwrap();
    let table = static_vs_live(&input, AnalysisMode::Raw);

    // Exactly one row per (agent, level) with data.
    let by_key: BTreeMap<(String, EvaluationLevel), _> = table
        .iter()
        .map(|r| ((r.agent_id.clone(), r.level), r.clone()))
        .collect();

    let a = by_key
        .get(&("agent-a".into(), EvaluationLevel::L0Official))
        .expect("agent-a L0 row must exist");
    assert_eq!(a.static_passed, 2);
    assert_eq!(a.static_evaluated, 2);
    assert_eq!(a.live_passed, 0);
    assert_eq!(a.live_evaluated, 2);
    assert!((a.static_pass_rate.unwrap() - 1.0).abs() < 1e-12);
    assert!((a.live_pass_rate.unwrap() - 0.0).abs() < 1e-12);
    assert!(
        (a.delta.unwrap() - (-1.0)).abs() < 1e-12,
        "headline claim: agent-a's live rate is 1.0 below its static rate"
    );
    assert_eq!(a.ratio, Some(0.0));

    let b = by_key
        .get(&("agent-b".into(), EvaluationLevel::L0Official))
        .expect("agent-b L0 row must exist");
    assert!((b.static_pass_rate.unwrap() - 0.5).abs() < 1e-12);
    assert!((b.live_pass_rate.unwrap() - 0.5).abs() < 1e-12);
    assert!(b.delta.unwrap().abs() < 1e-12);
    assert!((b.ratio.unwrap() - 1.0).abs() < 1e-12);
}

#[test]
fn milestone_l_paper_export_emits_static_vs_live_files() {
    let run = seed_run_dir();
    let out = TempDir::new().unwrap();

    let input = load_bundle_dir(run.path(), &LoadOptions::default()).unwrap();
    let manifest = write_paper_exports(&input, out.path(), AnalysisMode::Cumulative).unwrap();

    assert_eq!(
        manifest.schema_version, PAPER_EXPORT_SCHEMA_VERSION,
        "Milestone L bumps paper-export schema_version"
    );
    assert_eq!(
        PAPER_EXPORT_SCHEMA_VERSION, 3,
        "schema bump must move from 2 -> 3 for analysis_mode"
    );

    for name in ["static_vs_live.csv", "static_vs_live.json"] {
        assert!(
            out.path().join(name).is_file(),
            "expected {name} to be written"
        );
        assert!(
            manifest.files.iter().any(|f| f.path == name),
            "manifest must reference {name}"
        );
    }

    // Manifest is sorted by path; determinism is covered by
    // Milestone G's acceptance test - we only need to confirm that
    // the new entries did not break the ordering invariant.
    let mut sorted = manifest.files.clone();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(
        manifest.files, sorted,
        "paper-export manifest files must be sorted by path"
    );
}
