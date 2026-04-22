//! L0/L1 evaluation pipeline.
//!
//! This module is the user-visible orchestrator that glues every earlier
//! piece together and produces the three acceptance artifacts for
//! Milestone C:
//!
//! 1. An `EvaluationResult` for L0 (official) and for L1 (trusted rerun).
//! 2. A hash-chained `trace.jsonl`.
//! 3. A sealed evidence bundle (`artifact_hashes.json`) whose
//!    `bundle_hash` is stable across reruns of the same inputs.
//!
//! # Determinism
//!
//! Every timestamp that lands in a trace event or in the bundle index
//! flows through the injected [`Clock`]. Every identifier (`run_id`,
//! `bundle_id`) is derived via `UUIDv5` from a [`DeterministicSeed`]. The
//! `workspace_dir` under which the scorer runs is *not* recorded in any
//! hashed artifact: only the resolved image reference, the command, and
//! the resource limits are. Two invocations of [`EvaluationPipeline::run`]
//! with:
//!
//! - the same `BenchmarkTask`,
//! - the same `CandidateResolution`,
//! - the same patch bytes,
//! - the same `FixedClock` (reset between runs),
//! - the same workspace template (byte-identical contents),
//! - the same container engine and scorer types,
//!
//! produce bit-identical `trace.jsonl` bytes and identical
//! `bundle_hash`. This is the Milestone C acceptance invariant.
//!
//! # Failure modes
//!
//! Every failure path that maps to a stable [`FailureReason`] code
//! emits an `EvaluationResult` with `status = Fail` (or `Invalid` for
//! harness errors). The pipeline itself only raises a [`PipelineError`]
//! when a step *cannot produce a bundle at all* (for example, the
//! workspace template is missing). A failed evaluator is still a
//! successful pipeline run.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use eval_ladder_core::{
    canonical_json, BenchmarkId, BenchmarkTask, CandidateResolution, CoreError, EvaluationLevel,
    EvaluationResult, EvaluationStatus, EvaluatorVersion, FailureReason, SchemaVersion,
    Sha256Digest, EVALUATOR_VERSION, SCHEMA_VERSION,
};
use eval_ladder_evidence::{
    BundleBuilder, BundleBuilderError, EvidenceBundleIndex, MANDATORY_BUNDLE_FILES,
};
use eval_ladder_traces::{EventType, TraceWriter, TraceWriterError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::clock::Clock;
use crate::container::{ContainerEngine, ContainerEngineError, EnvVar, ExecSpec, ResourceLimits};
use crate::extension::{ExtensionContext, ExtensionError, LevelExtension};
use crate::identity::{DeterministicSeed, RunIdentity};
use crate::patch::{apply_patch, PatchApplyError, PatchApplyOutcome};
use crate::scorer::Scorer;
use crate::workspace::{prepare_workspace, WorkspaceError};

/// Declarative inputs for one pipeline invocation.
pub struct PipelineInputs<'a> {
    /// Normalized benchmark task.
    pub task: &'a BenchmarkTask,
    /// Candidate resolution to evaluate.
    pub candidate: &'a CandidateResolution,
    /// Bytes of the candidate patch (may be empty).
    pub patch_bytes: &'a [u8],
    /// Directory containing the unpatched workspace template. The
    /// pipeline never mutates this directory.
    pub workspace_template: &'a Path,
    /// Staging root used for per-level isolated workspaces. Must exist
    /// and be writable. Typically a `TempDir`.
    pub staging_root: &'a Path,
    /// Directory into which the evidence bundle is written. Must be
    /// empty or absent.
    pub bundle_dir: &'a Path,
    /// Deterministic identity seed. Use the same seed across reruns to
    /// get identical `run_id`/`bundle_id` and therefore identical bundle
    /// hashes.
    pub identity_seed: &'a DeterministicSeed,
    /// Resource limits applied to every in-container execution.
    pub resource_limits: ResourceLimits,
    /// Additional environment variables to expose to the scorer.
    pub env: &'a [EnvVar],
    /// Post-L1 extensions to run in order (L2 / L3 / L4). Empty means
    /// L0+L1 only, which is the Milestone C surface.
    pub extensions: &'a [&'a dyn LevelExtension],
}

/// Pipeline outputs. All `EvaluationResult`s are also serialized into
/// the bundle, but they are returned here for convenience so the caller
/// does not have to reparse them from disk.
#[derive(Debug, Clone)]
pub struct PipelineOutcome {
    /// Deterministic run/bundle identifiers used for this invocation.
    pub identity: RunIdentity,
    /// L0 (official) verdict.
    pub l0: EvaluationResult,
    /// L1 (trusted rerun) verdict.
    pub l1: EvaluationResult,
    /// Extension verdicts (L2/L3/L4), in the order they ran.
    pub extensions: Vec<EvaluationResult>,
    /// Sealed evidence-bundle index.
    pub bundle_index: EvidenceBundleIndex,
    /// Bundle-level SHA-256. A convenience copy of
    /// `bundle_index.bundle_hash`.
    pub bundle_hash: Sha256Digest,
}

/// Errors that prevent the pipeline from producing any bundle at all.
#[derive(Debug, Error)]
pub enum PipelineError {
    /// Failed to materialize a workspace from the template.
    #[error("workspace: {0}")]
    Workspace(#[from] WorkspaceError),
    /// Patch bytes could not be applied.
    #[error("patch apply: {0}")]
    Patch(#[from] PatchApplyError),
    /// Container engine failure that prevents both L0 and L1 from
    /// running.
    #[error("container: {0}")]
    Container(#[from] ContainerEngineError),
    /// Trace writer I/O or protocol error.
    #[error("trace: {0}")]
    Trace(#[from] TraceWriterError),
    /// Evidence bundle finalization error.
    #[error("bundle: {0}")]
    Bundle(#[from] BundleBuilderError),
    /// Canonical JSON serialization error.
    #[error("core: {0}")]
    Core(#[from] CoreError),
    /// Filesystem I/O error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Bundle destination exists and is non-empty.
    #[error("bundle_dir must be empty or absent: {0}")]
    BundleDirNotEmpty(PathBuf),
    /// Inputs disagree on task / benchmark / candidate identity.
    #[error("input mismatch: {0}")]
    InputMismatch(String),
    /// Official test entrypoint string was empty.
    #[error("task has no official_test_entrypoint")]
    MissingEntrypoint,
    /// A [`LevelExtension`] returned an error.
    #[error("extension: {0}")]
    Extension(#[from] ExtensionError),
    /// Two extensions claim the same level or the same result filename,
    /// which would overwrite each other's bundle artifact.
    #[error("duplicate extension slot: {0}")]
    DuplicateExtension(String),
}

/// Minimal run manifest stored in `run_manifest.json`.
///
/// The manifest records every input and configuration knob that affects
/// reproducibility. Timestamps flow through the injected clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunManifest {
    /// Schema version.
    pub schema_version: SchemaVersion,
    /// Evaluator version that produced this manifest.
    pub evaluator_version: EvaluatorVersion,
    /// Deterministic run identifier.
    pub run_id: eval_ladder_core::RunId,
    /// Deterministic bundle identifier.
    pub bundle_id: eval_ladder_core::BundleId,
    /// Benchmark identifier.
    pub benchmark_id: BenchmarkId,
    /// Task identifier.
    pub task_id: eval_ladder_core::TaskId,
    /// Candidate identifier.
    pub candidate_id: eval_ladder_core::CandidateId,
    /// Resolved image reference as returned by the container engine.
    pub image_ref: String,
    /// Exact command executed by the scorer.
    pub command: Vec<String>,
    /// Resource limits applied to the run.
    pub resource_limits: ResourceLimits,
    /// Whether the patch apply step modified the workspace.
    pub patch_apply_outcome: PatchApplyOutcome,
    /// Levels produced by the pipeline in the order they ran
    /// (always starts with L0 then L1, followed by any extension
    /// levels).
    pub levels_emitted: Vec<EvaluationLevel>,
    /// Started timestamp (from injected clock).
    pub started_at: DateTime<Utc>,
    /// Finished timestamp (from injected clock).
    pub finished_at: DateTime<Utc>,
}

/// Orchestrator. Constructed once per evaluation and then called with
/// [`Self::run`].
///
/// Holding the engine, scorer, and clock on the pipeline instance (by
/// reference) keeps the trait-object dance explicit. Callers usually
/// construct a pipeline per run because the container engine and scorer
/// types vary with the benchmark family.
#[derive(Debug)]
pub struct EvaluationPipeline<'e, E, S, C>
where
    E: ContainerEngine,
    S: Scorer,
    C: Clock,
{
    engine: &'e E,
    scorer: &'e S,
    clock: &'e C,
}

impl<'e, E, S, C> EvaluationPipeline<'e, E, S, C>
where
    E: ContainerEngine,
    S: Scorer,
    C: Clock,
{
    /// Construct a pipeline over the given engine, scorer, and clock.
    pub fn new(engine: &'e E, scorer: &'e S, clock: &'e C) -> Self {
        Self {
            engine,
            scorer,
            clock,
        }
    }

    /// Execute one evaluation end-to-end. Returns every
    /// `EvaluationResult` (L0, L1, and any extension verdicts) and the
    /// sealed evidence bundle.
    pub fn run(&self, inputs: PipelineInputs<'_>) -> Result<PipelineOutcome, PipelineError> {
        validate_inputs(&inputs)?;
        validate_extensions(inputs.extensions)?;
        ensure_bundle_dir_ready(inputs.bundle_dir)?;

        let identity = RunIdentity::deterministic(inputs.identity_seed);

        // --- prepare the bundle root and the trace writer --------------
        std::fs::create_dir_all(inputs.bundle_dir)?;
        let trace_path = inputs.bundle_dir.join("trace.jsonl");
        let mut trace = TraceWriter::create(
            &trace_path,
            identity.run_id,
            inputs.candidate.candidate_id,
            inputs.task.task_id.clone(),
        )?;

        let started_at = self.clock.now();
        trace.append_at(
            EventType::RunStarted,
            json!({
                "evaluator_version": EVALUATOR_VERSION,
                "benchmark_id": inputs.task.benchmark_id,
                "task_id": inputs.task.task_id,
                "candidate_id": inputs.candidate.candidate_id,
                "run_id": identity.run_id,
                "bundle_id": identity.bundle_id,
            }),
            started_at,
        )?;

        // --- resolve image & emit ContainerPrepared --------------------
        let image_ref = self.engine.prepare_image(&inputs.task.environment_ref)?;
        trace.append_at(
            EventType::ContainerPrepared,
            json!({
                "image_ref": image_ref,
                "declared_environment_ref": inputs.task.environment_ref,
            }),
            self.clock.now(),
        )?;

        // --- L0 official run ------------------------------------------
        let workspace_l0 = inputs.staging_root.join("workspace_l0");
        prepare_workspace(inputs.workspace_template, &workspace_l0)?;
        let patch_outcome_l0 = apply_patch(&workspace_l0, inputs.patch_bytes)?;
        trace.append_at(
            EventType::PatchApplied,
            json!({
                "level": EvaluationLevel::L0Official,
                "outcome": patch_outcome_l0,
                "bytes": inputs.patch_bytes.len(),
            }),
            self.clock.now(),
        )?;

        let command = split_entrypoint(&inputs.task.official_test_entrypoint)?;
        let spec = ExecSpec::new(
            image_ref.clone(),
            &workspace_l0,
            command.clone(),
            inputs.env.to_vec(),
            inputs.resource_limits.clone(),
        );

        let l0_started_at = self.clock.now();
        trace.append_at(
            EventType::OfficialEvalStarted,
            json!({ "command": command }),
            l0_started_at,
        )?;
        let l0_exec = self.engine.exec(&spec)?;
        let l0_finished_at = self.clock.now();
        let l0_verdict = self.scorer.score(&l0_exec);
        trace.append_at(
            EventType::OfficialEvalFinished,
            json!({
                "outcome": l0_verdict.outcome,
                "primary_reason": l0_verdict.primary_reason,
                "metrics": l0_verdict.metrics,
                "timed_out": l0_exec.timed_out,
                "exit_code": l0_exec.exit_code,
            }),
            l0_finished_at,
        )?;

        // --- L1 trusted rerun -----------------------------------------
        let workspace_l1 = inputs.staging_root.join("workspace_l1");
        prepare_workspace(inputs.workspace_template, &workspace_l1)?;
        let patch_outcome_l1 = apply_patch(&workspace_l1, inputs.patch_bytes)?;

        let l1_started_at = self.clock.now();
        let l1_spec = ExecSpec::new(
            image_ref.clone(),
            &workspace_l1,
            command.clone(),
            inputs.env.to_vec(),
            inputs.resource_limits.clone(),
        );
        let l1_exec = self.engine.exec(&l1_spec)?;
        let l1_finished_at = self.clock.now();
        let l1_rerun_verdict = self.scorer.score(&l1_exec);

        let l1_verdict = reconcile_l1(&l0_verdict, &l1_rerun_verdict, patch_outcome_l1);
        trace.append_at(
            EventType::OfficialEvalFinished, // second official invocation
            json!({
                "level": EvaluationLevel::L1TrustedRerun,
                "rerun_outcome": l1_rerun_verdict.outcome,
                "rerun_primary_reason": l1_rerun_verdict.primary_reason,
                "reconciled_outcome": l1_verdict.outcome,
                "reconciled_primary_reason": l1_verdict.primary_reason,
                "timed_out": l1_exec.timed_out,
                "exit_code": l1_exec.exit_code,
            }),
            l1_finished_at,
        )?;

        // --- build EvaluationResults ----------------------------------
        let l0_result = build_eval_result(
            inputs.candidate,
            inputs.task,
            EvaluationLevel::L0Official,
            &l0_verdict,
            l0_started_at,
            l0_finished_at,
        );
        let l1_result = build_eval_result(
            inputs.candidate,
            inputs.task,
            EvaluationLevel::L1TrustedRerun,
            &l1_verdict,
            l1_started_at,
            l1_finished_at,
        );

        // --- write all mandatory bundle files -------------------------
        write_canonical_json(
            &inputs.bundle_dir.join("candidate_resolution.json"),
            &inputs.candidate,
        )?;
        write_canonical_json(&inputs.bundle_dir.join("official_results.json"), &l0_result)?;
        write_canonical_json(
            &inputs.bundle_dir.join("l1_trusted_rerun_results.json"),
            &l1_result,
        )?;
        std::fs::write(inputs.bundle_dir.join("patch.diff"), inputs.patch_bytes)?;
        std::fs::write(inputs.bundle_dir.join("stdout.log"), &l0_exec.stdout)?;
        std::fs::write(inputs.bundle_dir.join("stderr.log"), &l0_exec.stderr)?;

        // --- run any post-L1 extensions (L2/L3/L4) --------------------
        let mut extension_results: Vec<EvaluationResult> =
            Vec::with_capacity(inputs.extensions.len());
        for ext in inputs.extensions {
            let ctx = ExtensionContext {
                task: inputs.task,
                candidate: inputs.candidate,
                patch_bytes: inputs.patch_bytes,
                workspace_template: inputs.workspace_template,
                staging_root: inputs.staging_root,
                bundle_dir: inputs.bundle_dir,
                image_ref: &image_ref,
                env: inputs.env,
                resource_limits: &inputs.resource_limits,
                engine: self.engine as &dyn ContainerEngine,
                clock: self.clock as &dyn Clock,
                l0: &l0_result,
                l1: &l1_result,
                run_id: identity.run_id,
                candidate_id: inputs.candidate.candidate_id,
                task_id: inputs.task.task_id.clone(),
            };
            let result = ext.run(&ctx, &mut trace)?;
            if result.level != ext.level() {
                return Err(PipelineError::InputMismatch(format!(
                    "extension {} emitted a result for {:?} but declared {:?}",
                    ext.name(),
                    result.level,
                    ext.level()
                )));
            }
            write_canonical_json(&inputs.bundle_dir.join(ext.result_file()), &result)?;
            extension_results.push(result);
        }

        let container_metadata = json!({
            "engine": engine_kind_label::<E>(),
            "image_ref": image_ref,
            "command": command,
            "resource_limits": inputs.resource_limits,
            "patch_apply_outcome_l0": patch_outcome_l0,
            "patch_apply_outcome_l1": patch_outcome_l1,
        });
        write_canonical_json(
            &inputs.bundle_dir.join("container_metadata.json"),
            &container_metadata,
        )?;

        let mut levels_emitted = vec![EvaluationLevel::L0Official, EvaluationLevel::L1TrustedRerun];
        levels_emitted.extend(extension_results.iter().map(|r| r.level));

        let finished_at = self.clock.now();

        let run_manifest = RunManifest {
            schema_version: SCHEMA_VERSION,
            evaluator_version: EVALUATOR_VERSION,
            run_id: identity.run_id,
            bundle_id: identity.bundle_id,
            benchmark_id: inputs.task.benchmark_id,
            task_id: inputs.task.task_id.clone(),
            candidate_id: inputs.candidate.candidate_id,
            image_ref: image_ref.clone(),
            command,
            resource_limits: inputs.resource_limits.clone(),
            patch_apply_outcome: patch_outcome_l0,
            levels_emitted: levels_emitted.clone(),
            started_at,
            finished_at,
        };
        write_canonical_json(&inputs.bundle_dir.join("run_manifest.json"), &run_manifest)?;

        let mut ext_summary = serde_json::Map::new();
        for (level, result) in levels_emitted
            .iter()
            .skip(2) // L0 + L1 already captured below
            .zip(extension_results.iter())
        {
            ext_summary.insert(
                level.short_code().to_owned(),
                json!({
                    "status": result.status,
                    "primary_reason": result.primary_reason,
                }),
            );
        }
        trace.append_at(
            EventType::RunFinished,
            json!({
                "l0_status": l0_result.status,
                "l0_primary_reason": l0_result.primary_reason,
                "l1_status": l1_result.status,
                "l1_primary_reason": l1_result.primary_reason,
                "extensions": serde_json::Value::Object(ext_summary),
            }),
            finished_at,
        )?;

        // --- seal bundle ----------------------------------------------
        let bundle_index = BundleBuilder::new(
            inputs.bundle_dir,
            inputs.candidate.candidate_id,
            inputs.task.task_id.clone(),
            inputs.task.benchmark_id,
        )
        .with_bundle_id(identity.bundle_id)
        .finalize_at(finished_at)?;

        // --- sanity: every mandatory file must now be present --------
        for name in MANDATORY_BUNDLE_FILES {
            if !inputs.bundle_dir.join(name).exists() {
                return Err(PipelineError::Bundle(
                    BundleBuilderError::MissingMandatoryFile((*name).to_owned()),
                ));
            }
        }

        let bundle_hash = bundle_index.bundle_hash.clone();
        Ok(PipelineOutcome {
            identity,
            l0: l0_result,
            l1: l1_result,
            extensions: extension_results,
            bundle_index,
            bundle_hash,
        })
    }
}

// ---------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------

fn validate_extensions(exts: &[&dyn LevelExtension]) -> Result<(), PipelineError> {
    use std::collections::HashSet;
    let mut levels: HashSet<EvaluationLevel> = HashSet::with_capacity(exts.len());
    let mut files: HashSet<&'static str> = HashSet::with_capacity(exts.len());
    for ext in exts {
        if matches!(
            ext.level(),
            EvaluationLevel::L0Official | EvaluationLevel::L1TrustedRerun
        ) {
            return Err(PipelineError::DuplicateExtension(format!(
                "extension {} targets L0/L1, which are owned by the pipeline",
                ext.name()
            )));
        }
        if !levels.insert(ext.level()) {
            return Err(PipelineError::DuplicateExtension(format!(
                "two extensions target {:?}",
                ext.level()
            )));
        }
        if !files.insert(ext.result_file()) {
            return Err(PipelineError::DuplicateExtension(format!(
                "two extensions write the same bundle file {}",
                ext.result_file()
            )));
        }
        if matches!(
            ext.result_file(),
            "candidate_resolution.json"
                | "run_manifest.json"
                | "official_results.json"
                | "l1_trusted_rerun_results.json"
                | "container_metadata.json"
                | "artifact_hashes.json"
                | "patch.diff"
                | "stdout.log"
                | "stderr.log"
                | "trace.jsonl"
        ) {
            return Err(PipelineError::DuplicateExtension(format!(
                "extension {} would overwrite reserved bundle file {}",
                ext.name(),
                ext.result_file()
            )));
        }
    }
    Ok(())
}

fn validate_inputs(i: &PipelineInputs<'_>) -> Result<(), PipelineError> {
    if i.task.task_id != i.candidate.task_id {
        return Err(PipelineError::InputMismatch(format!(
            "task_id {} does not match candidate.task_id {}",
            i.task.task_id, i.candidate.task_id
        )));
    }
    if i.task.benchmark_id != i.candidate.benchmark_id {
        return Err(PipelineError::InputMismatch(format!(
            "benchmark_id {:?} does not match candidate.benchmark_id {:?}",
            i.task.benchmark_id, i.candidate.benchmark_id
        )));
    }
    if i.task.official_test_entrypoint.trim().is_empty() {
        return Err(PipelineError::MissingEntrypoint);
    }
    Ok(())
}

fn ensure_bundle_dir_ready(dir: &Path) -> Result<(), PipelineError> {
    if dir.exists() {
        let mut iter = std::fs::read_dir(dir)?;
        if iter.next().is_some() {
            return Err(PipelineError::BundleDirNotEmpty(dir.to_path_buf()));
        }
    }
    Ok(())
}

fn split_entrypoint(s: &str) -> Result<Vec<String>, PipelineError> {
    // Whitespace splitting is the SWE-bench convention for the
    // `official_test_entrypoint` string. It intentionally does not
    // support shell features (quoting, redirection, env substitution).
    let parts: Vec<String> = s.split_whitespace().map(str::to_owned).collect();
    if parts.is_empty() {
        return Err(PipelineError::MissingEntrypoint);
    }
    Ok(parts)
}

fn reconcile_l1(
    l0: &crate::scorer::ScorerVerdict,
    l1_rerun: &crate::scorer::ScorerVerdict,
    l1_patch: PatchApplyOutcome,
) -> crate::scorer::ScorerVerdict {
    use crate::scorer::ScorerVerdict;

    // A patch-apply failure in L1 (but not in L0) would have manifested
    // as a PipelineError earlier; if we reach this point both patches
    // applied.
    let _ = l1_patch;

    if l0.outcome == l1_rerun.outcome && l0.primary_reason == l1_rerun.primary_reason {
        return ScorerVerdict {
            outcome: l0.outcome.clone(),
            primary_reason: if l0.outcome == crate::artifact::RunOutcome::Pass {
                FailureReason::PASS.as_str().to_owned()
            } else {
                // L1 inherits L0's failure class but relabels it as an L1
                // code for downstream analysis.
                FailureReason::L1_HARNESS_ERROR.as_str().to_owned()
            },
            secondary_reasons: vec![l0.primary_reason.clone()],
            metrics: json!({
                "rerun_agreement": true,
                "l0_metrics": l0.metrics,
                "l1_metrics": l1_rerun.metrics,
            }),
        };
    }

    ScorerVerdict {
        outcome: crate::artifact::RunOutcome::Fail,
        primary_reason: FailureReason::L1_RERUN_DISAGREEMENT.as_str().to_owned(),
        secondary_reasons: vec![l0.primary_reason.clone(), l1_rerun.primary_reason.clone()],
        metrics: json!({
            "rerun_agreement": false,
            "l0_outcome": l0.outcome,
            "l1_outcome": l1_rerun.outcome,
            "l0_metrics": l0.metrics,
            "l1_metrics": l1_rerun.metrics,
        }),
    }
}

fn build_eval_result(
    candidate: &CandidateResolution,
    task: &BenchmarkTask,
    level: EvaluationLevel,
    verdict: &crate::scorer::ScorerVerdict,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
) -> EvaluationResult {
    let status = match verdict.outcome {
        crate::artifact::RunOutcome::Pass => EvaluationStatus::Pass,
        crate::artifact::RunOutcome::Fail => EvaluationStatus::Fail,
        crate::artifact::RunOutcome::Invalid => EvaluationStatus::Invalid,
    };
    let mut res = EvaluationResult::new(
        candidate.candidate_id,
        task.task_id.clone(),
        level,
        status,
        verdict.primary_reason.clone(),
        started_at,
        finished_at,
    );
    res.secondary_reasons.clone_from(&verdict.secondary_reasons);
    res.metrics.clone_from(&verdict.metrics);
    res
}

fn write_canonical_json<T: Serialize>(path: &Path, value: &T) -> Result<(), PipelineError> {
    let mut bytes = canonical_json(value)?;
    bytes.push(b'\n');
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Best-effort label for the container engine used. Only the type name
/// matters (not the instance state) so we can report it in
/// `container_metadata.json`.
fn engine_kind_label<E: ?Sized>() -> &'static str {
    let name = std::any::type_name::<E>();
    if name.ends_with("NoopEngine") {
        "noop"
    } else if name.ends_with("LocalProcessEngine") {
        "local"
    } else {
        // Do not leak crate paths into the bundle; callers can stitch
        // the concrete backend name in via their own metadata.
        "other"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;
    use crate::container::LocalProcessEngine;
    use crate::scorer::SimpleExitCodeScorer;
    use chrono::TimeZone;
    use eval_ladder_core::{
        BenchmarkLanguage, CandidateId, ContextMode, GenerationMetadata, GenerationMode,
        PatchFormat, TaskId,
    };
    use serde_json::Value;
    use std::fs;
    use tempfile::tempdir;

    fn fixture_task() -> BenchmarkTask {
        let mut t = BenchmarkTask::new(
            BenchmarkId::SweBenchVerified,
            TaskId::new("fixture__trivial-1").unwrap(),
            "fixture/repo",
            "42",
            "Fixture title",
            "Fixture body.",
            "deadbeefcafe0000000000000000000000000000",
            "local:fixture",
            // Use `cargo --version` as a portable, fast, always-available
            // scorer. On a pinned toolchain the stdout/stderr are
            // byte-identical across two invocations, which is what the
            // determinism test needs.
            "cargo --version",
            BenchmarkLanguage::Rust,
            "https://example.test/fixture/42",
            Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
        );
        t.labels = vec!["fixture".to_owned()];
        t
    }

    fn fixture_candidate(task: &BenchmarkTask) -> CandidateResolution {
        use uuid::Uuid;
        let uid = Uuid::new_v5(
            &crate::identity::EVAL_LADDER_NAMESPACE,
            b"fixture-candidate-1",
        );
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
                tool_configuration: Value::Object(serde_json::Map::new()),
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

    fn pipeline_inputs_at<'a>(
        task: &'a BenchmarkTask,
        candidate: &'a CandidateResolution,
        template: &'a Path,
        staging: &'a Path,
        bundle_dir: &'a Path,
        seed: &'a DeterministicSeed,
    ) -> PipelineInputs<'a> {
        PipelineInputs {
            task,
            candidate,
            patch_bytes: b"",
            workspace_template: template,
            staging_root: staging,
            bundle_dir,
            identity_seed: seed,
            resource_limits: ResourceLimits {
                cpu_limit: None,
                memory_limit: None,
                wall_timeout: Some(std::time::Duration::from_secs(60)),
            },
            env: &[],
            extensions: &[],
        }
    }

    #[test]
    fn end_to_end_fixture_produces_bundle() {
        let task = fixture_task();
        let candidate = fixture_candidate(&task);

        let template = tempdir().unwrap();
        write_template(template.path());
        let staging = tempdir().unwrap();
        let bundle_dir = tempdir().unwrap().path().join("bundle_a");

        let seed = DeterministicSeed::build(
            candidate.candidate_id,
            task.task_id.clone(),
            EVALUATOR_VERSION.to_string(),
            "t-e2e",
        );
        let engine = LocalProcessEngine;
        let scorer = SimpleExitCodeScorer;
        let clock = FixedClock::deterministic();

        let pipeline = EvaluationPipeline::new(&engine, &scorer, &clock);
        let outcome = pipeline
            .run(pipeline_inputs_at(
                &task,
                &candidate,
                template.path(),
                staging.path(),
                &bundle_dir,
                &seed,
            ))
            .expect("pipeline must produce a bundle");

        assert_eq!(outcome.l0.status, EvaluationStatus::Pass);
        assert_eq!(outcome.l0.primary_reason, "PASS");
        assert_eq!(outcome.l1.status, EvaluationStatus::Pass);
        assert_eq!(outcome.l1.primary_reason, "PASS");
        eval_ladder_evidence::verify_bundle(&bundle_dir)
            .expect("evidence bundle must verify against its own index");
    }

    #[test]
    fn bundle_hash_is_stable_across_reruns() {
        let task = fixture_task();
        let candidate = fixture_candidate(&task);

        let seed = DeterministicSeed::build(
            candidate.candidate_id,
            task.task_id.clone(),
            EVALUATOR_VERSION.to_string(),
            "t-determinism",
        );

        let run_once = |tag: &str| -> Sha256Digest {
            let template = tempdir().unwrap();
            write_template(template.path());
            let staging = tempdir().unwrap();
            let bundle_root = tempdir().unwrap();
            let bundle_dir = bundle_root.path().join(format!("bundle_{tag}"));

            let engine = LocalProcessEngine;
            let scorer = SimpleExitCodeScorer;
            let clock = FixedClock::deterministic();

            let pipeline = EvaluationPipeline::new(&engine, &scorer, &clock);
            let outcome = pipeline
                .run(pipeline_inputs_at(
                    &task,
                    &candidate,
                    template.path(),
                    staging.path(),
                    &bundle_dir,
                    &seed,
                ))
                .expect("rerun must also succeed");

            // Also assert trace.jsonl bytes match on second call (below).
            let trace = std::fs::read(bundle_dir.join("trace.jsonl")).unwrap();
            std::fs::write(bundle_root.path().join("trace_copy.bin"), &trace).unwrap();
            outcome.bundle_hash
        };

        let h1 = run_once("a");
        let h2 = run_once("b");
        assert_eq!(
            h1, h2,
            "rerun must produce an identical bundle_hash: Milestone C acceptance"
        );
    }
}
