//! Milestone E acceptance: a fixture candidate passes L0, L1, and L2
//! but fails L3 under a restrictive policy, and reruns produce
//! bit-identical evidence bundles.
//!
//! This test exercises the full pipeline through the [`L3Extension`],
//! plus the [`L2Extension`] with an augmented-tests-only mode so L2
//! passes. The policy is engineered to fail for a reason that is
//! orthogonal to the candidate patch contents (a forbidden command
//! plus a required reproducible seed), so this test does not rely on
//! `git apply` being able to operate on a non-git workspace.
//!
//! The determinism rerun uses the same [`DeterministicSeed`] and a
//! [`FixedClock`] that is reset between runs. Any divergence in the
//! trace hash chain, the emitted trace events, or the bundle hash
//! fails the test, which is the Milestone C invariant carried
//! forward.

use std::path::Path;

use chrono::{TimeZone, Utc};
use eval_ladder_core::{
    BenchmarkId, BenchmarkLanguage, BenchmarkTask, CandidateId, CandidateResolution, ContextMode,
    EvaluationLevel, EvaluationStatus, GenerationMetadata, GenerationMode, PatchFormat, TaskId,
    EVALUATOR_VERSION,
};
use eval_ladder_evidence::verify_bundle;
use eval_ladder_policy::{L3Extension, NetworkMode, Policy, L3_RESULT_FILE};
use eval_ladder_runner::{
    DeterministicSeed, EvaluationPipeline, FixedClock, LevelExtension, LocalProcessEngine,
    PipelineInputs, ResourceLimits, SimpleExitCodeScorer, EVAL_LADDER_NAMESPACE,
};
use eval_ladder_strengthening::{
    AugmentedTestSpec, CommandSpec, L2Extension, RegressionSpec, StrengtheningMode,
    StrengtheningSpec,
};
use eval_ladder_traces::EventType;
use tempfile::tempdir;
use uuid::Uuid;

fn fixture_task() -> BenchmarkTask {
    BenchmarkTask::new(
        BenchmarkId::SweBenchVerified,
        TaskId::new("fixture__milestone-e").unwrap(),
        "fixture/repo",
        "42",
        "Milestone E fixture",
        "L2 pass / L3 fail demonstration.",
        "deadbeefcafe0000000000000000000000000000",
        "local:fixture",
        "cargo --version",
        BenchmarkLanguage::Rust,
        "https://example.test/fixture/42",
        Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
    )
}

fn fixture_candidate(task: &BenchmarkTask) -> CandidateResolution {
    let uid = Uuid::new_v5(&EVAL_LADDER_NAMESPACE, b"milestone-e-candidate-1");
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

/// Strengthening spec that passes: a single augmented test running
/// `cargo --version`. L0/L1 already demonstrate that this command is
/// available and deterministic on the test toolchain.
fn passing_strengthening_spec() -> StrengtheningSpec {
    StrengtheningSpec {
        schema_version: 1,
        augmented: AugmentedTestSpec {
            commands: vec![CommandSpec {
                id: "aug_smoke".into(),
                command: vec!["cargo".into(), "--version".into()],
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

/// Policy that will fail at L3 because it bans `cargo`, which is the
/// exact command L0/L1 executed. Everything else is permissive so
/// only `PV_FORBIDDEN_CMD` fires.
fn failing_policy() -> Policy {
    Policy {
        name: "milestone_e_fixture_policy".into(),
        requires_reproducible_seed: true,
        max_modified_files: None,
        allow_generated_tests: true,
        allow_dependency_lockfile_edits: true,
        network_mode: NetworkMode::Disabled,
        allowed_commands: Vec::new(),
        forbidden_commands: vec!["cargo".into()],
        allowed_edit_globs: Vec::new(),
        forbidden_edit_globs: Vec::new(),
        required_trace_events: vec![
            EventType::RunStarted,
            EventType::OfficialEvalStarted,
            EventType::OfficialEvalFinished,
            EventType::RunFinished,
        ],
    }
}

fn write_template(root: &Path) {
    std::fs::write(root.join("README.md"), "milestone-e fixture\n").unwrap();
}

fn run_full_pipeline(bundle_dir: &Path, seed_tag: &str) -> eval_ladder_runner::PipelineOutcome {
    let task = fixture_task();
    let candidate = fixture_candidate(&task);
    let spec = passing_strengthening_spec();
    let policy = failing_policy();

    let template = tempdir().unwrap();
    write_template(template.path());
    let staging = tempdir().unwrap();

    let engine = LocalProcessEngine;
    let scorer = SimpleExitCodeScorer;
    let clock = FixedClock::deterministic();

    let seed = DeterministicSeed::build(
        candidate.candidate_id,
        task.task_id.clone(),
        EVALUATOR_VERSION.to_string(),
        seed_tag,
    );

    let l2 = L2Extension::new(&spec, StrengtheningMode::TestsOnly);
    let l3 = L3Extension::new(&policy);
    let extensions: Vec<&dyn LevelExtension> = vec![&l2, &l3];

    let pipeline = EvaluationPipeline::new(&engine, &scorer, &clock);
    pipeline
        .run(PipelineInputs {
            task: &task,
            candidate: &candidate,
            patch_bytes: b"",
            workspace_template: template.path(),
            staging_root: staging.path(),
            bundle_dir,
            identity_seed: &seed,
            resource_limits: ResourceLimits {
                cpu_limit: None,
                memory_limit: None,
                wall_timeout: Some(std::time::Duration::from_secs(60)),
            },
            env: &[],
            extensions: &extensions,
        })
        .expect("pipeline must produce a bundle")
}

#[test]
fn milestone_e_acceptance_l2_pass_l3_fail() {
    let root = tempdir().unwrap();
    let bundle_dir = root.path().join("bundle");

    let outcome = run_full_pipeline(&bundle_dir, "e-accept");

    assert_eq!(
        outcome.l0.status,
        EvaluationStatus::Pass,
        "L0 must pass on the fixture"
    );
    assert_eq!(
        outcome.l1.status,
        EvaluationStatus::Pass,
        "L1 must pass on the fixture"
    );
    assert_eq!(
        outcome.extensions.len(),
        2,
        "L2 and L3 both expected to run"
    );

    let l2 = &outcome.extensions[0];
    assert_eq!(l2.level, EvaluationLevel::L2Strengthened);
    assert_eq!(
        l2.status,
        EvaluationStatus::Pass,
        "L2 must pass on the fixture"
    );

    let l3 = &outcome.extensions[1];
    assert_eq!(l3.level, EvaluationLevel::L3PolicyConformant);
    assert_eq!(
        l3.status,
        EvaluationStatus::Fail,
        "Milestone E: L3 must fail while L0/L1/L2 pass"
    );
    assert_eq!(
        l3.primary_reason, "PV_FORBIDDEN_CMD",
        "expected primary reason PV_FORBIDDEN_CMD, got {}",
        l3.primary_reason
    );

    // The bundle must verify and contain the L3 artifact.
    verify_bundle(&bundle_dir).expect("bundle must verify");
    assert!(
        bundle_dir.join(L3_RESULT_FILE).is_file(),
        "bundle must contain {L3_RESULT_FILE}"
    );

    // trace.jsonl must carry a PolicyCheckStarted and at least one
    // PolicyViolationDetected event.
    let trace = std::fs::read_to_string(bundle_dir.join("trace.jsonl")).unwrap();
    assert!(
        trace.contains("\"PolicyCheckStarted\""),
        "trace must contain PolicyCheckStarted"
    );
    assert!(
        trace.contains("\"PolicyViolationDetected\""),
        "trace must contain PolicyViolationDetected"
    );
}

#[test]
fn milestone_e_l3_reruns_are_deterministic() {
    let root_a = tempdir().unwrap();
    let bundle_a = root_a.path().join("bundle_a");
    let outcome_a = run_full_pipeline(&bundle_a, "e-det");

    let root_b = tempdir().unwrap();
    let bundle_b = root_b.path().join("bundle_b");
    let outcome_b = run_full_pipeline(&bundle_b, "e-det");

    assert_eq!(
        outcome_a.bundle_hash, outcome_b.bundle_hash,
        "L3 reruns must produce identical bundle hashes (Milestone C invariant)"
    );

    let trace_a = std::fs::read(bundle_a.join("trace.jsonl")).unwrap();
    let trace_b = std::fs::read(bundle_b.join("trace.jsonl")).unwrap();
    assert_eq!(
        trace_a, trace_b,
        "trace.jsonl bytes must be identical across reruns"
    );

    let results_a = std::fs::read(bundle_a.join(L3_RESULT_FILE)).unwrap();
    let results_b = std::fs::read(bundle_b.join(L3_RESULT_FILE)).unwrap();
    assert_eq!(
        results_a, results_b,
        "policy_results.json must be byte-identical across reruns"
    );
}
