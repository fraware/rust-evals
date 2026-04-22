//! Milestone D acceptance test.
//!
//! Runs the full L0 + L1 + L2 pipeline on a fixture where:
//!
//! - The candidate patch is a no-op (empty patch).
//! - The official scorer is `cargo --version`, which always passes, so
//!   L0 and L1 both PASS.
//! - The strengthening spec declares an augmented unit test that fails
//!   unconditionally on this fixture (runs `cargo nonexistent-command`,
//!   which exits with a non-zero status), so L2 FAILS with
//!   `L2_AUG_TESTS_FAIL`.
//!
//! This is the Milestone D acceptance criterion: a candidate that
//! passes L0 can still fail L2, demonstrating that L2 is strictly
//! stronger than L0/L1 and that the score-descent signal is real.
//!
//! A second assertion pins the determinism invariant: re-running the
//! pipeline with a [`FixedClock`] produces identical `bundle_hash` and
//! byte-identical `trace.jsonl`.

use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use eval_ladder_core::{
    BenchmarkId, BenchmarkLanguage, BenchmarkTask, CandidateId, CandidateResolution, ContextMode,
    EvaluationLevel, EvaluationStatus, EvaluatorVersion, GenerationMetadata, GenerationMode,
    PatchFormat, Sha256Digest, TaskId, EVALUATOR_VERSION,
};
use eval_ladder_evidence::verify_bundle;
use eval_ladder_runner::{
    DeterministicSeed, EvaluationPipeline, FixedClock, LevelExtension, LocalProcessEngine,
    PipelineInputs, ResourceLimits, SimpleExitCodeScorer, EVAL_LADDER_NAMESPACE,
};
use eval_ladder_strengthening::{
    AugmentedTestSpec, CommandSpec, L2Extension, RegressionSpec, StrengtheningMode,
    StrengtheningSpec,
};
use tempfile::tempdir;
use uuid::Uuid;

fn fixture_task() -> BenchmarkTask {
    BenchmarkTask::new(
        BenchmarkId::SweBenchVerified,
        TaskId::new("fixture__milestone-d-1").unwrap(),
        "fixture/milestone-d",
        "1",
        "Milestone D fixture",
        "A no-op candidate that the L0 official scorer accepts but an \
         augmented L2 unit test rejects.",
        "deadbeefcafe0000000000000000000000000000",
        "local:fixture",
        // L0/L1 scorer: always succeeds on any machine with a Rust
        // toolchain, which is a prerequisite to build this crate.
        "cargo --version",
        BenchmarkLanguage::Rust,
        "https://example.test/fixture/milestone-d",
        Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
    )
}

fn fixture_candidate(task: &BenchmarkTask) -> CandidateResolution {
    let uid = Uuid::new_v5(&EVAL_LADDER_NAMESPACE, b"milestone-d-candidate");
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

fn fixture_spec_that_fails_l2() -> StrengtheningSpec {
    StrengtheningSpec {
        schema_version: 1,
        augmented: AugmentedTestSpec {
            commands: vec![CommandSpec {
                id: "aug_synthetic_edge_case".to_owned(),
                // `cargo` exists on every CI machine; this subcommand
                // does not, so the call exits with a non-zero code.
                // That is the "augmented test the official suite
                // missed".
                command: vec!["cargo".to_owned(), "eval-ladder-does-not-exist".to_owned()],
                env: Vec::new(),
                workdir: None,
                expected_exit_code: None,
                flaky: false,
            }],
            retry_flaky: false,
        },
        regression: RegressionSpec::default(),
        differential: None,
        property_fuzz: None,
    }
}

fn write_template(root: &Path) {
    fs::write(root.join("README.md"), "fixture workspace\n").unwrap();
}

struct RunRecord {
    bundle_hash: Sha256Digest,
    trace_bytes: Vec<u8>,
    l0: EvaluationStatus,
    l1: EvaluationStatus,
    l2: Option<EvaluationStatus>,
    l2_primary: Option<String>,
}

fn run_once(tag: &str) -> RunRecord {
    let task = fixture_task();
    let candidate = fixture_candidate(&task);
    let spec = fixture_spec_that_fails_l2();

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
        "milestone-d-acceptance",
    );

    let l2 = L2Extension::new(&spec, StrengtheningMode::TestsOnly);
    let extensions: &[&dyn LevelExtension] = &[&l2];

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
            extensions,
        })
        .expect("pipeline must produce a bundle");

    verify_bundle(&bundle_dir).expect("bundle must verify");

    let l2_result = outcome
        .extensions
        .iter()
        .find(|r| r.level == EvaluationLevel::L2Strengthened);

    let trace_bytes = fs::read(bundle_dir.join("trace.jsonl")).unwrap();

    RunRecord {
        bundle_hash: outcome.bundle_hash,
        trace_bytes,
        l0: outcome.l0.status,
        l1: outcome.l1.status,
        l2: l2_result.map(|r| r.status),
        l2_primary: l2_result.map(|r| r.primary_reason.clone()),
    }
}

#[test]
fn milestone_d_acceptance_l0_pass_l2_fail() {
    let r = run_once("a");

    assert_eq!(r.l0, EvaluationStatus::Pass, "L0 must pass on the fixture");
    assert_eq!(r.l1, EvaluationStatus::Pass, "L1 must pass on the fixture");
    assert_eq!(
        r.l2,
        Some(EvaluationStatus::Fail),
        "Milestone D acceptance: L2 must FAIL where L0/L1 PASS"
    );
    assert_eq!(
        r.l2_primary.as_deref(),
        Some("L2_AUG_TESTS_FAIL"),
        "L2 primary reason must be the augmented-tests code"
    );
}

#[test]
fn milestone_d_l2_reruns_are_deterministic() {
    let a = run_once("a");
    let b = run_once("b");

    assert_eq!(
        a.bundle_hash, b.bundle_hash,
        "Milestone C invariant must hold with L2 enabled: stable bundle_hash"
    );
    assert_eq!(
        a.trace_bytes, b.trace_bytes,
        "Milestone C invariant must hold with L2 enabled: byte-identical trace"
    );
}
