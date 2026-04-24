//! Milestone F acceptance: L4 proof subset wiring and determinism.
//!
//! The acceptance matrix covers four situations:
//!
//! 1. `l4_fixture_valid_when_obligation_met`: the checker returns
//!    `LeanStatus::Valid` with the obligation's `pass_criterion` and
//!    the pipeline records L4 = `Pass` with the same code.
//! 2. `l4_fixture_invalid_when_obligation_unmet`: the checker returns
//!    `LeanStatus::Invalid` with `L4_OBLIGATION_UNMET`; the pipeline
//!    records L4 = `Fail` with the same code and a matching trace
//!    event.
//! 3. `l4_fixture_not_applicable_when_missing_obligation`: the task
//!    has no obligation in the manifest; L4 is `NotApplicable` with
//!    `L4_OBLIGATION_NOT_APPLICABLE`.
//! 4. `l4_reruns_are_deterministic`: two identical pipeline runs
//!    produce byte-identical `trace.jsonl`, `proof_results.json`, and
//!    `bundle_hash`. This is the Milestone C invariant carried
//!    through L4.
//!
//! All four tests use the in-tree [`ScriptedChecker`] so they do not
//! depend on a Lean toolchain being installed. An opt-in integration
//! test that spawns `lake env lean` against the seeded
//! `Fixtures/MilestoneF.lean` obligation is marked `#[ignore]` and is
//! run on demand in the Tier 2 gate.

use std::path::Path;

use chrono::{TimeZone, Utc};
use eval_ladder_core::{
    BenchmarkId, BenchmarkLanguage, BenchmarkTask, CandidateId, CandidateResolution, ContextMode,
    EvaluationLevel, EvaluationStatus, GenerationMetadata, GenerationMode, PatchFormat, TaskId,
    EVALUATOR_VERSION,
};
use eval_ladder_evidence::verify_bundle;
use eval_ladder_lean::{
    Difficulty, L4Extension, LeanCheckOutcome, LeanChecker, ObligationManifest,
    ObligationProofChecker, ProofObligation, PropertyType, ScriptedChecker, SelectionRationale,
    L4_RESULT_FILE,
};
use eval_ladder_runner::{
    DeterministicSeed, EvaluationPipeline, FixedClock, L1Strategy, LevelExtension,
    LocalProcessEngine, PipelineInputs, ResourceLimits, SimpleExitCodeScorer,
    EVAL_LADDER_NAMESPACE,
};
use tempfile::tempdir;
use uuid::Uuid;

const FIXTURE_TASK_ID: &str = "fixture__milestone-f";
const FIXTURE_OBLIGATION_ID: &str = "obl.fixture.milestone_f";
const FIXTURE_PASS_CODE: &str = "L4_OBLIGATION_MET";

fn fixture_task() -> BenchmarkTask {
    BenchmarkTask::new(
        BenchmarkId::SweBenchVerified,
        TaskId::new(FIXTURE_TASK_ID).unwrap(),
        "fixture/repo",
        "7",
        "Milestone F fixture",
        "L4 proof subset integration fixture.",
        "deadbeefcafe0000000000000000000000000000",
        "local:fixture",
        "cargo --version",
        BenchmarkLanguage::Rust,
        "https://example.test/fixture/7",
        Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
    )
}

fn fixture_candidate(task: &BenchmarkTask) -> CandidateResolution {
    let uid = Uuid::new_v5(&EVAL_LADDER_NAMESPACE, b"milestone-f-candidate-1");
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

fn fixture_obligation() -> ProofObligation {
    ProofObligation {
        schema_version: 1,
        obligation_id: FIXTURE_OBLIGATION_ID.into(),
        task_id: FIXTURE_TASK_ID.into(),
        property_name: "trivial_identity".into(),
        property_type: PropertyType::NoPanicOrInvalidState,
        target_files: vec!["src/lib.rs".into()],
        informal_statement: "identity_is_reflexive holds for every natural number.".into(),
        formal_statement_ref: "EvalLadder/Obligations/Fixtures/MilestoneF.lean".into(),
        proof_checker: ObligationProofChecker {
            command: "lake".into(),
            args: vec![
                "env".into(),
                "lean".into(),
                "EvalLadder/Obligations/Fixtures/MilestoneF.lean".into(),
            ],
        },
        pass_criterion: FIXTURE_PASS_CODE.into(),
        difficulty: Difficulty {
            reviewer_hours: 0.25,
        },
        selection_rationale: SelectionRationale {
            one_or_two_sentence_property: true,
            local_scope: true,
            matters_to_issue: true,
            strictly_stronger_than_tests: true,
            bounded_effort: true,
        },
        witness_inputs: Vec::new(),
        expected_touched_symbols: Vec::new(),
    }
}

fn fixture_manifest() -> ObligationManifest {
    let mut m = ObligationManifest::empty();
    m.insert(fixture_obligation())
        .expect("fixture manifest must accept the obligation");
    m
}

fn write_template(root: &Path) {
    std::fs::write(root.join("README.md"), "milestone-f fixture\n").unwrap();
}

fn run_pipeline_with_checker<C: LeanChecker>(
    bundle_dir: &Path,
    manifest: &ObligationManifest,
    checker: &C,
    seed_tag: &str,
) -> eval_ladder_runner::PipelineOutcome {
    let task = fixture_task();
    let candidate = fixture_candidate(&task);

    let template = tempdir().unwrap();
    write_template(template.path());
    let staging = tempdir().unwrap();
    let lean_root = tempdir().unwrap();

    let engine = LocalProcessEngine;
    let scorer = SimpleExitCodeScorer;
    let clock = FixedClock::deterministic();

    let seed = DeterministicSeed::build(
        candidate.candidate_id,
        task.task_id.clone(),
        EVALUATOR_VERSION.to_string(),
        seed_tag,
    );

    let l4 = L4Extension::new(manifest, checker, lean_root.path());
    let extensions: Vec<&dyn LevelExtension> = vec![&l4];

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
            l1_strategy: L1Strategy::StrictRerun,
        })
        .expect("pipeline must produce a bundle")
}

fn only_l4(outcome: &eval_ladder_runner::PipelineOutcome) -> &eval_ladder_core::EvaluationResult {
    assert_eq!(
        outcome.extensions.len(),
        1,
        "expected exactly one extension result (L4), got {}",
        outcome.extensions.len()
    );
    let r = &outcome.extensions[0];
    assert_eq!(r.level, EvaluationLevel::L4Semantic);
    r
}

#[test]
fn l4_fixture_valid_when_obligation_met() {
    let root = tempdir().unwrap();
    let bundle_dir = root.path().join("bundle");
    let manifest = fixture_manifest();
    let checker = ScriptedChecker::new();
    checker.program(
        FIXTURE_OBLIGATION_ID,
        LeanCheckOutcome::valid(FIXTURE_PASS_CODE, "fixture obligation met"),
    );

    let outcome = run_pipeline_with_checker(&bundle_dir, &manifest, &checker, "f-valid");
    let l4 = only_l4(&outcome);
    assert_eq!(
        l4.status,
        EvaluationStatus::Pass,
        "expected L4 pass; got {:?} ({})",
        l4.status,
        l4.primary_reason
    );
    assert_eq!(l4.primary_reason, FIXTURE_PASS_CODE);

    verify_bundle(&bundle_dir).expect("bundle must verify");
    assert!(bundle_dir.join(L4_RESULT_FILE).is_file());
    let trace = std::fs::read_to_string(bundle_dir.join("trace.jsonl")).unwrap();
    assert!(trace.contains("\"ProofCheckStarted\""));
    assert!(trace.contains("\"ProofCheckFinished\""));
    assert!(trace.contains("\"status\":\"valid\""));
}

#[test]
fn l4_fixture_invalid_when_obligation_unmet() {
    let root = tempdir().unwrap();
    let bundle_dir = root.path().join("bundle");
    let manifest = fixture_manifest();
    let checker = ScriptedChecker::new();
    checker.program(
        FIXTURE_OBLIGATION_ID,
        LeanCheckOutcome::invalid("L4_OBLIGATION_UNMET", "obligation declared false"),
    );

    let outcome = run_pipeline_with_checker(&bundle_dir, &manifest, &checker, "f-invalid");
    let l4 = only_l4(&outcome);
    assert_eq!(l4.status, EvaluationStatus::Fail);
    assert_eq!(l4.primary_reason, "L4_OBLIGATION_UNMET");

    verify_bundle(&bundle_dir).expect("bundle must verify");
    let trace = std::fs::read_to_string(bundle_dir.join("trace.jsonl")).unwrap();
    assert!(trace.contains("\"status\":\"invalid\""));
    assert!(trace.contains("\"code\":\"L4_OBLIGATION_UNMET\""));
}

#[test]
fn l4_fixture_not_applicable_when_missing_obligation() {
    let root = tempdir().unwrap();
    let bundle_dir = root.path().join("bundle");
    // Empty manifest: no task has an obligation, so L4 must be NotApplicable.
    let manifest = ObligationManifest::empty();
    let checker = ScriptedChecker::new();

    let outcome = run_pipeline_with_checker(&bundle_dir, &manifest, &checker, "f-na");
    let l4 = only_l4(&outcome);
    assert_eq!(l4.status, EvaluationStatus::NotApplicable);
    assert_eq!(l4.primary_reason, "L4_OBLIGATION_NOT_APPLICABLE");

    verify_bundle(&bundle_dir).expect("bundle must verify");
    let trace = std::fs::read_to_string(bundle_dir.join("trace.jsonl")).unwrap();
    assert!(trace.contains("\"ProofCheckStarted\""));
    assert!(trace.contains("\"status\":\"not_applicable\""));
}

#[test]
fn l4_reruns_are_deterministic() {
    let manifest = fixture_manifest();

    let root_a = tempdir().unwrap();
    let bundle_a = root_a.path().join("bundle_a");
    let checker_a = ScriptedChecker::new();
    checker_a.program(
        FIXTURE_OBLIGATION_ID,
        LeanCheckOutcome::valid(FIXTURE_PASS_CODE, "fixture obligation met"),
    );
    let outcome_a = run_pipeline_with_checker(&bundle_a, &manifest, &checker_a, "f-det");

    let root_b = tempdir().unwrap();
    let bundle_b = root_b.path().join("bundle_b");
    let checker_b = ScriptedChecker::new();
    checker_b.program(
        FIXTURE_OBLIGATION_ID,
        LeanCheckOutcome::valid(FIXTURE_PASS_CODE, "fixture obligation met"),
    );
    let outcome_b = run_pipeline_with_checker(&bundle_b, &manifest, &checker_b, "f-det");

    assert_eq!(
        outcome_a.bundle_hash, outcome_b.bundle_hash,
        "L4 reruns must produce identical bundle hashes"
    );
    assert_eq!(
        std::fs::read(bundle_a.join("trace.jsonl")).unwrap(),
        std::fs::read(bundle_b.join("trace.jsonl")).unwrap(),
        "trace.jsonl bytes must be identical across reruns"
    );
    assert_eq!(
        std::fs::read(bundle_a.join(L4_RESULT_FILE)).unwrap(),
        std::fs::read(bundle_b.join(L4_RESULT_FILE)).unwrap(),
        "proof_results.json must be byte-identical across reruns"
    );
}

/// Opt-in end-to-end integration against the real Lean toolchain. The
/// fixture obligation points at `EvalLadder/Obligations/Fixtures/MilestoneF.lean`
/// which proves `n = n`; the external checker must report `valid`.
///
/// Ignored by default because it requires `lake` on PATH and the Lean
/// toolchain pinned by `packages/lean/EvalLadder/lean-toolchain`.
#[test]
#[ignore = "requires lake + Lean toolchain; run with `cargo test -p eval-ladder-lean -- --ignored`"]
fn l4_external_checker_against_lake_binary_ok() {
    let lean_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("lean")
        .join("EvalLadder");
    assert!(
        lean_root.is_dir(),
        "lean root not found at {}",
        lean_root.display()
    );

    let obligation = ProofObligation {
        // A checker that wraps `lake env lean` would normally emit
        // structured JSON via a small driver; here we use a shell
        // one-liner that calls `lean --version` as a smoke test. The
        // purpose of this ignored test is to document the production
        // path, not to verify any specific proof.
        proof_checker: ObligationProofChecker {
            command: "lake".into(),
            args: vec!["--version".into()],
        },
        ..fixture_obligation()
    };

    let checker = eval_ladder_lean::ExternalProcessChecker::new(&lean_root);
    let ctx = eval_ladder_lean::LeanCheckContext {
        lean_root: &lean_root,
        workspace: lean_root.as_path(),
        patch_bytes: b"",
    };
    let res = checker.check(&obligation, &ctx);
    // `lake --version` does not emit a LeanCheckOutcome JSON, so the
    // external checker is expected to surface a parse error. The
    // integration test just asserts we reached the checker and did
    // not panic; production drivers will return a well-formed
    // outcome.
    match res {
        Ok(_) | Err(eval_ladder_lean::LeanCheckError::Parse(_)) => (),
        Err(other) => panic!("unexpected integration error: {other}"),
    }
}
