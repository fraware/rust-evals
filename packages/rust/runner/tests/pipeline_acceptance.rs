//! Milestone C acceptance test.
//!
//! Runs the full L0/L1 pipeline twice on the same fixture task+candidate
//! with a [`FixedClock`] and asserts:
//!
//! 1. Both runs succeed and report L0 PASS + L1 PASS.
//! 2. The `bundle_hash` matches byte-for-byte across the two runs.
//! 3. The `trace.jsonl` matches byte-for-byte across the two runs.
//! 4. Every mandatory bundle file is present and digests verify.
//!
//! This is the contract Milestone C promises to reviewers: given the
//! same inputs, the evaluator produces an identical evidence bundle.

use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use eval_ladder_core::{
    BenchmarkId, BenchmarkLanguage, BenchmarkTask, CandidateId, CandidateResolution, ContextMode,
    EvaluationStatus, EvaluatorVersion, GenerationMetadata, GenerationMode, PatchFormat,
    Sha256Digest, TaskId, EVALUATOR_VERSION,
};
use eval_ladder_evidence::verify_bundle;
use eval_ladder_runner::{
    DeterministicSeed, EvaluationPipeline, FixedClock, LocalProcessEngine, PipelineInputs,
    ResourceLimits, SimpleExitCodeScorer, EVAL_LADDER_NAMESPACE,
};
use tempfile::tempdir;
use uuid::Uuid;

fn fixture_task() -> BenchmarkTask {
    BenchmarkTask::new(
        BenchmarkId::SweBenchVerified,
        TaskId::new("fixture__acceptance-1").unwrap(),
        "fixture/acceptance",
        "1",
        "Acceptance fixture",
        "Trivial task exercised by the Milestone C acceptance suite.",
        "deadbeefcafe0000000000000000000000000000",
        "local:fixture",
        // Portable, deterministic, always-available scorer on any CI
        // machine that builds this crate. A pinned toolchain keeps its
        // stdout bytes stable across back-to-back invocations.
        "cargo --version",
        BenchmarkLanguage::Rust,
        "https://example.test/fixture/acceptance",
        Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
    )
}

fn fixture_candidate(task: &BenchmarkTask) -> CandidateResolution {
    let uid = Uuid::new_v5(&EVAL_LADDER_NAMESPACE, b"acceptance-candidate");
    let mut c = CandidateResolution::new(
        task.benchmark_id,
        task.task_id.clone(),
        "fixture-harness",
        "fixture-model",
        GenerationMode::SingleShot,
        PatchFormat::UnifiedDiff,
        "fixture://noop",
        GenerationMetadata {
            temperature: Some(0.0),
            tool_configuration: serde_json::Value::Object(serde_json::Map::new()),
            context_mode: ContextMode::FileLevel,
            repo_reproduction_used: false,
            random_seed: Some(0),
        },
        Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
    );
    c.candidate_id = CandidateId::from(uid);
    c
}

fn write_template(root: &Path) {
    fs::write(root.join("README.md"), "fixture workspace\n").unwrap();
}

struct RunRecord {
    bundle_hash: Sha256Digest,
    trace_bytes: Vec<u8>,
    l0_pass: bool,
    l1_pass: bool,
}

fn run_once(tag: &str) -> RunRecord {
    let task = fixture_task();
    let candidate = fixture_candidate(&task);

    let template = tempdir().unwrap();
    write_template(template.path());
    let staging = tempdir().unwrap();
    let bundle_root = tempdir().unwrap();
    let bundle_dir = bundle_root.path().join(format!("bundle_{tag}"));

    let engine = LocalProcessEngine;
    let scorer = SimpleExitCodeScorer;
    let clock = FixedClock::deterministic();

    let seed = DeterministicSeed::build(
        candidate.candidate_id,
        task.task_id.clone(),
        <EvaluatorVersion as ToString>::to_string(&EVALUATOR_VERSION),
        "acceptance",
    );

    let pipeline = EvaluationPipeline::new(&engine, &scorer, &clock);
    let outcome = pipeline
        .run(PipelineInputs {
            task: &task,
            candidate: &candidate,
            patch_bytes: b"",
            workspace_template: template.path(),
            staging_root: staging.path(),
            bundle_dir: &bundle_dir,
            identity_seed: &seed,
            resource_limits: ResourceLimits {
                cpu_limit: None,
                memory_limit: None,
                wall_timeout: Some(Duration::from_secs(60)),
            },
            env: &[],
            extensions: &[],
        })
        .expect("pipeline must produce a bundle");

    // Every mandatory file must be in the bundle and digests must verify.
    verify_bundle(&bundle_dir).expect("bundle must verify");

    let trace_bytes = fs::read(bundle_dir.join("trace.jsonl")).unwrap();

    RunRecord {
        bundle_hash: outcome.bundle_hash,
        trace_bytes,
        l0_pass: outcome.l0.status == EvaluationStatus::Pass,
        l1_pass: outcome.l1.status == EvaluationStatus::Pass,
    }
}

#[test]
fn milestone_c_acceptance_rerun_is_deterministic() {
    let a = run_once("a");
    let b = run_once("b");

    assert!(a.l0_pass, "L0 must pass in run 1");
    assert!(a.l1_pass, "L1 must pass in run 1");
    assert!(b.l0_pass, "L0 must pass in run 2");
    assert!(b.l1_pass, "L1 must pass in run 2");

    assert_eq!(
        a.bundle_hash, b.bundle_hash,
        "Milestone C acceptance: bundle_hash must be stable across reruns"
    );
    assert_eq!(
        a.trace_bytes, b.trace_bytes,
        "Milestone C acceptance: trace.jsonl must be byte-identical across reruns"
    );
}
