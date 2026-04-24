//! `eval-ladder evaluate {candidate,batch}`
//!
//! `evaluate candidate` runs the L0 (official), L1 (trusted rerun)
//! pipeline unconditionally, optionally the L2 strengthening extension
//! when `--strengthening-spec` is supplied, optionally the L3 policy
//! extension when `--policy` is supplied, and optionally the L4
//! proof-subset extension when `--obligations` is supplied.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use eval_ladder_core::{
    BenchmarkId, BenchmarkTask, CandidateResolution, EvaluationLevel, EvaluatorVersion,
    EVALUATOR_VERSION,
};
use eval_ladder_lean::{ExternalProcessChecker, L4Extension, ObligationManifest};
use eval_ladder_policy::{L3Extension, Policy};
use eval_ladder_runner::{
    DeterministicSeed, DockerCliEngine, EvaluationPipeline, FixedClock, L1Strategy, LevelExtension,
    LocalProcessEngine, PipelineInputs, ResourceLimits, SimpleExitCodeScorer, SystemClock,
};
use eval_ladder_strengthening::{L2Extension, StrengtheningMode, StrengtheningSpec};
use tracing::info;

use crate::commands::batch::{self, BatchArgs};
use crate::config::EvaluatorConfig;

/// `evaluate` subcommands.
#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)] // clap subcommand args structs; boxing breaks derive
pub enum EvaluateCmd {
    /// Evaluate a single candidate.
    Candidate(CandidateArgs),
    /// Evaluate a JSONL panel of candidates.
    Batch(BatchArgs),
}

fn parse_levels(input: &str) -> Result<Vec<EvaluationLevel>> {
    let mut out = Vec::new();
    for piece in input.split(',') {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        out.push(piece.parse::<EvaluationLevel>()?);
    }
    if out.is_empty() {
        bail!("--levels must contain at least one level");
    }
    Ok(out)
}

/// Arguments for `evaluate candidate`.
#[derive(Debug, Args)]
pub struct CandidateArgs {
    /// Path to the candidate resolution JSON.
    #[arg(long)]
    pub candidate: PathBuf,
    /// Path to the normalized benchmark task manifest JSON.
    #[arg(long)]
    pub task: PathBuf,
    /// Path to the patch file. An empty file is treated as a noop patch.
    #[arg(long)]
    pub patch: PathBuf,
    /// Path to the workspace template directory (an unpatched checkout
    /// of the task's repo at `base_commit`). The pipeline copies this
    /// directory into a per-level staging dir; it is never mutated.
    #[arg(long)]
    pub workspace_template: PathBuf,
    /// Directory into which the evidence bundle is written. Must be
    /// empty or absent.
    #[arg(long)]
    pub bundle_dir: PathBuf,
    /// Comma-separated level list (for example `L0,L1`).
    #[arg(long, default_value = "L0,L1")]
    pub levels: String,
    /// Path to the evaluator configuration TOML.
    #[arg(long)]
    pub config: PathBuf,
    /// Identity-seed tag appended to the deterministic seed. Changes
    /// the `run_id`/`bundle_id` without changing any other input; use
    /// this to run the same candidate twice and keep the two bundles
    /// separable.
    #[arg(long, default_value = "default")]
    pub seed_tag: String,
    /// Use [`FixedClock`] for deterministic bundle hashes. Production
    /// callers usually leave this off.
    #[arg(long)]
    pub deterministic_clock: bool,
    /// Wall-clock timeout for each container exec, in seconds.
    #[arg(long, default_value_t = 1800)]
    pub timeout_secs: u64,
    /// Optional path to a task-level L2 strengthening spec (JSON).
    /// When supplied together with a `L2` entry in `--levels`, L2 runs
    /// after L1 and emits `strengthened_results.json` plus
    /// `strengthening_report.json` in the bundle.
    #[arg(long)]
    pub strengthening_spec: Option<PathBuf>,
    /// Strengthening mode. Only used when `--strengthening-spec` is
    /// set. One of `tests_only`, `tests_plus_diff`,
    /// `tests_plus_regression`, or `full_l2`.
    #[arg(long, default_value = "full_l2")]
    pub strengthening_mode: String,
    /// Optional oracle patch file. Required only for
    /// `tests_plus_diff` / `full_l2` runs that exercise the
    /// differential-behaviour validator.
    #[arg(long)]
    pub oracle_patch: Option<PathBuf>,
    /// Optional path to an L3 policy TOML document. When supplied
    /// together with an `L3` entry in `--levels`, L3 runs after L2
    /// (or after L1 when L2 was not requested) and emits
    /// `policy_results.json` in the bundle.
    #[arg(long)]
    pub policy: Option<PathBuf>,
    /// Whether the runner observed outbound network activity. Only
    /// meaningful for container engines that expose network telemetry;
    /// defaults to `false`, which is always correct for the local
    /// process engine shipped in-tree.
    #[arg(long, default_value_t = false)]
    pub network_accessed: bool,
    /// Optional path to the proof-subset obligation manifest (JSONL).
    /// When supplied together with an `L4` entry in `--levels`, L4
    /// runs after L3 (or after the lowest available rung when L3 was
    /// not requested) and emits `proof_results.json` in the bundle.
    #[arg(long)]
    pub obligations: Option<PathBuf>,
    /// Optional path to the Lean project root (typically
    /// `packages/lean/EvalLadder`). Required when `--obligations` is
    /// supplied; ignored otherwise.
    #[arg(long)]
    pub lean_root: Option<PathBuf>,
}

/// Dispatch.
pub fn run(cmd: EvaluateCmd) -> Result<()> {
    match cmd {
        EvaluateCmd::Candidate(args) => run_candidate(args),
        EvaluateCmd::Batch(args) => batch::run_batch(args),
    }
}

fn run_candidate(args: CandidateArgs) -> Result<()> {
    let levels = parse_levels(&args.levels)?;

    let wants_l2 = levels.contains(&EvaluationLevel::L2Strengthened);
    let wants_l3 = levels.contains(&EvaluationLevel::L3PolicyConformant);
    let wants_l4 = levels.contains(&EvaluationLevel::L4Semantic);
    let unsupported: Vec<_> = levels
        .iter()
        .filter(|l| {
            !matches!(
                l,
                EvaluationLevel::L0Official
                    | EvaluationLevel::L1TrustedRerun
                    | EvaluationLevel::L2Strengthened
                    | EvaluationLevel::L3PolicyConformant
                    | EvaluationLevel::L4Semantic
            )
        })
        .collect();
    if !unsupported.is_empty() {
        bail!(
            "levels {unsupported:?} are not yet implemented; supported levels are L0,L1,L2,L3,L4"
        );
    }
    if wants_l2 && args.strengthening_spec.is_none() {
        bail!("--levels includes L2 but --strengthening-spec was not supplied");
    }
    if !wants_l2 && args.strengthening_spec.is_some() {
        bail!(
            "--strengthening-spec was supplied but --levels does not include L2; \
             add L2 to --levels or drop --strengthening-spec"
        );
    }
    if wants_l3 && args.policy.is_none() {
        bail!("--levels includes L3 but --policy was not supplied");
    }
    if !wants_l3 && args.policy.is_some() {
        bail!(
            "--policy was supplied but --levels does not include L3; \
             add L3 to --levels or drop --policy"
        );
    }
    if wants_l4 && args.obligations.is_none() {
        bail!("--levels includes L4 but --obligations was not supplied");
    }
    if !wants_l4 && args.obligations.is_some() {
        bail!(
            "--obligations was supplied but --levels does not include L4; \
             add L4 to --levels or drop --obligations"
        );
    }
    if wants_l4 && args.lean_root.is_none() {
        bail!("--levels includes L4 but --lean-root was not supplied");
    }

    let config = EvaluatorConfig::from_path(&args.config)
        .with_context(|| format!("loading evaluator config from {}", args.config.display()))?;

    if !args.task.exists() {
        bail!("task manifest does not exist: {}", args.task.display());
    }
    if !args.candidate.exists() {
        bail!(
            "candidate file does not exist: {}",
            args.candidate.display()
        );
    }
    if !args.workspace_template.exists() {
        bail!(
            "workspace template does not exist: {}",
            args.workspace_template.display()
        );
    }

    let task_bytes = std::fs::read(&args.task)
        .with_context(|| format!("reading task manifest from {}", args.task.display()))?;
    let task: BenchmarkTask = serde_json::from_slice(&task_bytes)
        .with_context(|| format!("parsing task manifest at {}", args.task.display()))?;

    let candidate_bytes = std::fs::read(&args.candidate)
        .with_context(|| format!("reading candidate from {}", args.candidate.display()))?;
    let candidate: CandidateResolution = serde_json::from_slice(&candidate_bytes)
        .with_context(|| format!("parsing candidate at {}", args.candidate.display()))?;

    let patch_bytes = if args.patch.exists() {
        std::fs::read(&args.patch)
            .with_context(|| format!("reading patch from {}", args.patch.display()))?
    } else {
        Vec::new()
    };

    let strengthening_spec: Option<StrengtheningSpec> = match &args.strengthening_spec {
        Some(path) => {
            if !path.exists() {
                bail!("strengthening spec does not exist: {}", path.display());
            }
            let bytes = std::fs::read(path)
                .with_context(|| format!("reading strengthening spec from {}", path.display()))?;
            Some(
                StrengtheningSpec::from_json(&bytes)
                    .with_context(|| format!("parsing strengthening spec at {}", path.display()))?,
            )
        }
        None => None,
    };

    let oracle_patch_bytes: Option<Vec<u8>> = match &args.oracle_patch {
        Some(path) => {
            if !path.exists() {
                bail!("oracle patch does not exist: {}", path.display());
            }
            Some(
                std::fs::read(path)
                    .with_context(|| format!("reading oracle patch from {}", path.display()))?,
            )
        }
        None => None,
    };

    let policy: Option<Policy> = match &args.policy {
        Some(path) => {
            if !path.exists() {
                bail!("policy file does not exist: {}", path.display());
            }
            Some(
                Policy::from_path(path)
                    .with_context(|| format!("loading policy from {}", path.display()))?,
            )
        }
        None => None,
    };

    let obligations: Option<ObligationManifest> = match &args.obligations {
        Some(path) => {
            if !path.exists() {
                bail!("obligations manifest does not exist: {}", path.display());
            }
            Some(
                ObligationManifest::from_path(path)
                    .with_context(|| format!("loading obligations from {}", path.display()))?,
            )
        }
        None => None,
    };
    let lean_checker: Option<ExternalProcessChecker> = args
        .lean_root
        .as_ref()
        .map(|p| ExternalProcessChecker::new(p.clone()));

    let strengthening_mode: Option<StrengtheningMode> = if wants_l2 {
        Some(args.strengthening_mode.parse().with_context(|| {
            format!("parsing --strengthening-mode {:?}", args.strengthening_mode)
        })?)
    } else {
        None
    };

    let staging_root = tempfile::tempdir().context("creating staging tempdir for the pipeline")?;

    let scorer = SimpleExitCodeScorer;
    let seed = DeterministicSeed::build(
        candidate.candidate_id,
        task.task_id.clone(),
        <EvaluatorVersion as ToString>::to_string(&EVALUATOR_VERSION),
        args.seed_tag.clone(),
    );
    let resource_limits = ResourceLimits {
        cpu_limit: None,
        memory_limit: None,
        wall_timeout: Some(Duration::from_secs(args.timeout_secs)),
    };

    info!(
        profile = %config.name,
        task = %task.task_id,
        candidate = %candidate.candidate_id,
        ?levels,
        "evaluate candidate: running pipeline"
    );

    // Build the L2 and L3 extensions up-front so they outlive the two
    // deterministic/system-clock branches below. Borrow lifetimes flow
    // from `strengthening_spec`, `oracle_patch_bytes`, and `policy`,
    // which all live for the rest of this function.
    let mut l2_extension: Option<L2Extension<'_>> = None;
    if let (Some(spec), Some(mode)) = (strengthening_spec.as_ref(), strengthening_mode) {
        let mut ext = L2Extension::new(spec, mode);
        if let Some(bytes) = oracle_patch_bytes.as_deref() {
            ext = ext.with_oracle_patch(bytes);
        }
        l2_extension = Some(ext);
    }
    let l3_extension: Option<L3Extension<'_>> = policy.as_ref().map(|p| {
        L3Extension::new(p).with_observation(
            eval_ladder_policy::L3Observation::default()
                .with_network_accessed(args.network_accessed),
        )
    });
    let l4_extension: Option<L4Extension<'_>> = match (
        obligations.as_ref(),
        lean_checker.as_ref(),
        args.lean_root.as_ref(),
    ) {
        (Some(m), Some(c), Some(root)) => Some(L4Extension::new(m, c, root)),
        _ => None,
    };

    let mut extensions: Vec<&dyn LevelExtension> = Vec::new();
    if let Some(ref ext) = l2_extension {
        extensions.push(ext);
    }
    if let Some(ref ext) = l3_extension {
        extensions.push(ext);
    }
    if let Some(ref ext) = l4_extension {
        extensions.push(ext);
    }

    let use_docker = matches!(
        task.benchmark_id,
        BenchmarkId::SweBenchVerified | BenchmarkId::SweBenchLive
    ) && !task.environment_ref.starts_with("local:");
    let inputs = PipelineInputs {
        task: &task,
        candidate: &candidate,
        patch_bytes: &patch_bytes,
        workspace_template: &args.workspace_template,
        staging_root: staging_root.path(),
        bundle_dir: &args.bundle_dir,
        identity_seed: &seed,
        resource_limits,
        env: &[],
        extensions: &extensions,
        l1_strategy: L1Strategy::StrictRerun,
    };
    let outcome = if use_docker {
        let engine = DockerCliEngine;
        if args.deterministic_clock {
            let clock = FixedClock::deterministic();
            EvaluationPipeline::new(&engine, &scorer, &clock).run(inputs)?
        } else {
            let clock = SystemClock;
            EvaluationPipeline::new(&engine, &scorer, &clock).run(inputs)?
        }
    } else {
        let engine = LocalProcessEngine;
        if args.deterministic_clock {
            let clock = FixedClock::deterministic();
            EvaluationPipeline::new(&engine, &scorer, &clock).run(inputs)?
        } else {
            let clock = SystemClock;
            EvaluationPipeline::new(&engine, &scorer, &clock).run(inputs)?
        }
    };

    let mut report = serde_json::json!({
        "run_id": outcome.identity.run_id,
        "bundle_id": outcome.identity.bundle_id,
        "bundle_hash": outcome.bundle_hash,
        "bundle_dir": args.bundle_dir,
        "l0": {
            "status": outcome.l0.status,
            "primary_reason": outcome.l0.primary_reason,
        },
        "l1": {
            "status": outcome.l1.status,
            "primary_reason": outcome.l1.primary_reason,
        },
    });
    if let Some(obj) = report.as_object_mut() {
        for ext_result in &outcome.extensions {
            obj.insert(
                ext_result.level.short_code().to_ascii_lowercase(),
                serde_json::json!({
                    "status": ext_result.status,
                    "primary_reason": ext_result.primary_reason,
                }),
            );
        }
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
