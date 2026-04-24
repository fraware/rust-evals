//! Batch evaluation (Milestone H).
//!
//! `eval-ladder evaluate batch` iterates a JSONL panel of
//! [`PanelEntry`]s, drives the full L0 - L4 pipeline for each entry,
//! and writes:
//!
//! - One full sealed evidence bundle per entry under
//!   `<out-dir>/<bundle_name>/` (the `bundle_name` is taken from the
//!   entry, defaulting to the stringified `candidate_id`).
//! - A deterministic [`BatchSummary`] at `<out-dir>/batch_summary.json`
//!   listing every entry's verdict plus the batch-wide configuration
//!   fingerprint.
//!
//! The batch is intentionally single-threaded. Bundles are a pure
//! function of their inputs, so parallelism would only bring wall-time
//! savings; it is *not* required to match the Milestone C determinism
//! invariant, and it is trivially layered on top later.
//!
//! Resilience: one failing entry never aborts the batch. If any step
//! between parsing the panel line and invoking the pipeline fails, the
//! entry gets a [`BatchEntryStatus::Invalid`] row with a stable error
//! code and the loop continues. The exit code of the CLI is non-zero
//! only if *every* entry failed or the panel file itself was
//! unreadable.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{collections::HashMap, thread};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use eval_ladder_core::{
    canonical_json, BenchmarkId, BenchmarkTask, CandidateResolution, EvaluationLevel,
    EvaluationStatus, EvaluatorVersion, Sha256Digest, EVALUATOR_VERSION,
};
use eval_ladder_lean::{ExternalProcessChecker, L4Extension, ObligationManifest};
use eval_ladder_policy::{L3Extension, Policy};
use eval_ladder_runner::{
    DeterministicSeed, DockerCliEngine, EnvVar, EvaluationPipeline, FixedClock, L1Strategy,
    LevelExtension, LocalProcessEngine, PipelineInputs, PipelineOutcome, ResourceLimits,
    RunManifest, SimpleExitCodeScorer, SystemClock,
};
use eval_ladder_strengthening::{L2Extension, StrengtheningMode, StrengtheningSpec};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info, warn};

/// Current schema version for the persisted [`BatchSummary`].
pub const BATCH_SUMMARY_SCHEMA_VERSION: u32 = 1;

/// Canonical file name for the summary at the root of the output directory.
pub const BATCH_SUMMARY_FILE: &str = "batch_summary.json";

/// Arguments accepted by `eval-ladder evaluate batch`.
#[derive(Debug, Clone, clap::Args)]
pub struct BatchArgs {
    /// Path to the JSONL panel. One JSON object per line; `#` and blank
    /// lines are ignored. See [`PanelEntry`] for the per-line schema.
    #[arg(long)]
    pub input: PathBuf,

    /// Comma-separated level list. Each level in this list must be
    /// supported by the entries loaded from `--input` (otherwise the
    /// run yields `EvaluationStatus::Invalid` for the affected entry).
    #[arg(long, default_value = "L0,L1")]
    pub levels: String,

    /// Path to the evaluator configuration TOML.
    #[arg(long)]
    pub config: PathBuf,

    /// Output directory. One subdirectory per entry is created here,
    /// plus a single `batch_summary.json`. The directory is created if
    /// missing.
    #[arg(long)]
    pub out: PathBuf,

    /// Use [`FixedClock`] for every entry. Required for the
    /// Milestone H determinism acceptance test; production callers
    /// typically leave this off.
    #[arg(long)]
    pub deterministic_clock: bool,

    /// Batch-wide wall-clock timeout per container exec, in seconds.
    #[arg(long, default_value_t = 1800)]
    pub timeout_secs: u64,
    /// Optional short timeout used by adaptive mode.
    #[arg(long)]
    pub short_timeout_secs: Option<u64>,
    /// Adapt per-entry timeout using prior batch_summary reasons in `--out`.
    #[arg(long, default_value_t = false)]
    pub adaptive_timeouts: bool,
    /// Reuse existing successful bundles in `--out` when possible.
    #[arg(long, default_value_t = false)]
    pub resume: bool,
    /// Parallel jobs (entries are parallelized; each entry remains level-serial).
    #[arg(long, default_value_t = 1)]
    pub jobs: usize,
    /// Fast/Heavy workflow preset for level list.
    #[arg(long)]
    pub track: Option<String>,
    /// L1 strategy (`strict` or `smart_rust_reuse`).
    #[arg(long, default_value = "smart_rust_reuse")]
    pub l1_strategy: String,
    /// Optional root for shared rust target cache (CARGO_TARGET_DIR).
    #[arg(long)]
    pub rust_target_cache_root: Option<PathBuf>,
    /// Reuse duplicate workloads (same repo/base_commit/patch) by copying first bundle.
    #[arg(long, default_value_t = true)]
    pub dedupe_workloads: bool,

    /// Identity-seed tag appended to every deterministic seed. Changes
    /// the `run_id`/`bundle_id` without changing any other input.
    #[arg(long, default_value = "batch")]
    pub seed_tag: String,

    /// Optional path to a batch-wide L2 strengthening spec (JSON). If
    /// set, L2 must also be included in `--levels`.
    #[arg(long)]
    pub strengthening_spec: Option<PathBuf>,

    /// Strengthening mode; only used when `--strengthening-spec` is set.
    #[arg(long, default_value = "full_l2")]
    pub strengthening_mode: String,

    /// Optional oracle-patch file used by the differential validator in
    /// strengthening mode `tests_plus_diff` / `full_l2`.
    #[arg(long)]
    pub oracle_patch: Option<PathBuf>,

    /// Optional path to a batch-wide L3 policy TOML.
    #[arg(long)]
    pub policy: Option<PathBuf>,

    /// Whether the runner observed outbound network activity. Only
    /// meaningful for container engines that expose network telemetry;
    /// defaults to `false`.
    #[arg(long, default_value_t = false)]
    pub network_accessed: bool,

    /// Optional path to the proof-subset obligation manifest (JSONL).
    #[arg(long)]
    pub obligations: Option<PathBuf>,

    /// Optional path to the Lean project root. Required when
    /// `--obligations` is supplied.
    #[arg(long)]
    pub lean_root: Option<PathBuf>,
}

/// One entry in a panel JSONL. Paths are resolved relative to the
/// directory that contains the panel file unless they are absolute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelEntry {
    /// Path to the normalized benchmark task manifest JSON.
    pub task: PathBuf,
    /// Path to the candidate-resolution JSON.
    pub candidate: PathBuf,
    /// Path to the patch file. An empty file is treated as a noop patch.
    pub patch: PathBuf,
    /// Path to the workspace-template directory.
    pub workspace_template: PathBuf,
    /// Optional override for the per-bundle subdirectory name.
    /// Defaults to the candidate's stringified UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_name: Option<String>,
    /// Optional free-form entry id used in the summary; defaults to
    /// `bundle_name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_id: Option<String>,
}

/// Errors that preclude launching the batch at all (panel unreadable,
/// invalid level list, &c.). Per-entry failures are captured as
/// [`BatchEntryStatus::Invalid`] rows instead.
#[derive(Debug, Error)]
pub enum BatchError {
    /// Panel file missing or unreadable.
    #[error("batch panel io: {0}")]
    Io(#[from] std::io::Error),
    /// A panel line failed to parse.
    #[error("batch panel line {line}: {source}")]
    Json {
        /// 1-indexed line number.
        line: usize,
        /// Underlying JSON error.
        source: serde_json::Error,
    },
    /// Panel was empty.
    #[error("batch panel {0} has no entries")]
    EmptyPanel(PathBuf),
    /// Level list parse error.
    #[error("batch levels: {0}")]
    Levels(String),
    /// Batch-wide extension flag combination was inconsistent.
    #[error("batch extension flags: {0}")]
    Extensions(String),
    /// The canonical summary could not be serialized.
    #[error("batch summary canonicalize: {0}")]
    Canonicalize(#[from] eval_ladder_core::CoreError),
}

/// Deterministic per-entry outcome summary row.
// `levels` is a `serde_json::Value` to preserve verbatim level output,
// which blocks `Eq`. `PartialEq` is still derived so tests can compare
// rows directly.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchEntryRow {
    /// Entry id from the panel (defaults to `bundle_name`).
    pub entry_id: String,
    /// Bundle directory name relative to the batch output directory.
    pub bundle_name: String,
    /// Panel-declared task manifest path (display form, for audit).
    pub task_path: String,
    /// Panel-declared candidate path (display form, for audit).
    pub candidate_path: String,
    /// Overall per-entry status (`ok` / `invalid`).
    pub status: BatchEntryStatus,
    /// Stable primary reason code for each level the entry produced.
    /// Keys are `l0`..`l4`; values are `{status, primary_reason}`.
    pub levels: serde_json::Value,
    /// Bundle hash, if the pipeline succeeded far enough to seal a
    /// bundle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_hash: Option<Sha256Digest>,
    /// Error message if the entry was `Invalid`. Stable code prefix
    /// (`BATCH_LOAD_*`, `BATCH_PIPELINE_*`) + human-readable detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Per-entry status in the summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchEntryStatus {
    /// The pipeline produced a sealed bundle (levels may still have
    /// failed individually).
    Ok,
    /// Panel-line load or pipeline dispatch failed before a bundle was
    /// sealed.
    Invalid,
}

/// Deterministic batch-run artifact written to `batch_summary.json`.
// Inherits the `Value`-blocked Eq from `BatchEntryRow`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchSummary {
    /// Schema version.
    pub schema_version: u32,
    /// Evaluator version that ran the batch.
    pub evaluator_version: EvaluatorVersion,
    /// Canonicalized level list applied to the batch.
    pub levels: Vec<EvaluationLevel>,
    /// Total entries loaded from the panel.
    pub total_entries: u64,
    /// Number of entries whose bundle sealed successfully.
    pub ok_entries: u64,
    /// Number of `BatchEntryStatus::Invalid` entries.
    pub invalid_entries: u64,
    /// Per-entry rows, sorted by `bundle_name` for stable diffs.
    pub entries: Vec<BatchEntryRow>,
    /// Wall-clock batch start (omitted when `deterministic_clock`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// Wall-clock batch end (omitted when `deterministic_clock`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
}

/// Load the panel file into a `Vec<PanelEntry>`. Paths inside each
/// entry are resolved against `panel_path.parent()`.
pub fn load_panel(panel_path: &Path) -> Result<Vec<PanelEntry>, BatchError> {
    let file = fs::File::open(panel_path)?;
    let reader = BufReader::new(file);
    let mut out: Vec<PanelEntry> = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line_no = i + 1;
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut entry: PanelEntry =
            serde_json::from_str(trimmed).map_err(|e| BatchError::Json {
                line: line_no,
                source: e,
            })?;
        // Resolve relative paths against the panel directory.
        let base = panel_path.parent().unwrap_or_else(|| Path::new("."));
        resolve_in_place(&mut entry.task, base);
        resolve_in_place(&mut entry.candidate, base);
        resolve_in_place(&mut entry.patch, base);
        resolve_in_place(&mut entry.workspace_template, base);
        out.push(entry);
    }
    if out.is_empty() {
        return Err(BatchError::EmptyPanel(panel_path.to_path_buf()));
    }
    Ok(out)
}

fn resolve_in_place(p: &mut PathBuf, base: &Path) {
    if p.is_relative() {
        *p = base.join(&*p);
    }
}

/// Parse `--levels` the same way `run_candidate` does.
fn parse_levels(input: &str) -> Result<Vec<EvaluationLevel>, BatchError> {
    let mut out = Vec::new();
    for piece in input.split(',') {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        let parsed: EvaluationLevel = piece
            .parse()
            .map_err(|e: eval_ladder_core::CoreError| BatchError::Levels(e.to_string()))?;
        out.push(parsed);
    }
    if out.is_empty() {
        return Err(BatchError::Levels(
            "--levels must contain at least one level".into(),
        ));
    }
    Ok(out)
}

fn track_levels(track: &str) -> Result<Vec<EvaluationLevel>, BatchError> {
    match track {
        "fast" => parse_levels("L3,L4"),
        "heavy" => parse_levels("L0,L1"),
        "full" => parse_levels("L0,L1,L2,L3,L4"),
        other => Err(BatchError::Levels(format!(
            "--track must be one of fast|heavy|full, got {other}"
        ))),
    }
}

/// Batch-wide resources that are loaded once and shared across every
/// entry.
struct BatchExtensions<'a> {
    l2: Option<L2Extension<'a>>,
    l3: Option<L3Extension<'a>>,
    l4: Option<L4Extension<'a>>,
}

impl<'a> BatchExtensions<'a> {
    fn as_level_extensions(&'a self) -> Vec<&'a dyn LevelExtension> {
        let mut v: Vec<&dyn LevelExtension> = Vec::new();
        if let Some(ref e) = self.l2 {
            v.push(e);
        }
        if let Some(ref e) = self.l3 {
            v.push(e);
        }
        if let Some(ref e) = self.l4 {
            v.push(e);
        }
        v
    }
}

/// Container for the long-lived, borrowed resources behind
/// `BatchExtensions`.
struct BatchBackingResources {
    strengthening_spec: Option<StrengtheningSpec>,
    strengthening_mode: Option<StrengtheningMode>,
    oracle_patch_bytes: Option<Vec<u8>>,
    policy: Option<Policy>,
    obligations: Option<ObligationManifest>,
    lean_root: Option<PathBuf>,
    lean_checker: Option<ExternalProcessChecker>,
}

fn load_backing_resources(args: &BatchArgs) -> Result<BatchBackingResources> {
    let strengthening_spec = match &args.strengthening_spec {
        Some(path) => {
            let bytes = fs::read(path)
                .with_context(|| format!("reading strengthening spec from {}", path.display()))?;
            Some(
                StrengtheningSpec::from_json(&bytes)
                    .with_context(|| format!("parsing strengthening spec at {}", path.display()))?,
            )
        }
        None => None,
    };
    let strengthening_mode: Option<StrengtheningMode> = match args.strengthening_spec {
        Some(_) => Some(args.strengthening_mode.parse().with_context(|| {
            format!("parsing --strengthening-mode {:?}", args.strengthening_mode)
        })?),
        None => None,
    };
    let oracle_patch_bytes = match &args.oracle_patch {
        Some(path) => Some(
            fs::read(path)
                .with_context(|| format!("reading oracle patch from {}", path.display()))?,
        ),
        None => None,
    };
    let policy = match &args.policy {
        Some(path) => Some(
            Policy::from_path(path)
                .with_context(|| format!("loading policy from {}", path.display()))?,
        ),
        None => None,
    };
    let obligations = match &args.obligations {
        Some(path) => Some(
            ObligationManifest::from_path(path)
                .with_context(|| format!("loading obligations from {}", path.display()))?,
        ),
        None => None,
    };
    let lean_checker = args
        .lean_root
        .as_ref()
        .map(|p| ExternalProcessChecker::new(p.clone()));
    Ok(BatchBackingResources {
        strengthening_spec,
        strengthening_mode,
        oracle_patch_bytes,
        policy,
        obligations,
        lean_root: args.lean_root.clone(),
        lean_checker,
    })
}

fn build_extensions(res: &BatchBackingResources, network_accessed: bool) -> BatchExtensions<'_> {
    let l2 = match (res.strengthening_spec.as_ref(), res.strengthening_mode) {
        (Some(spec), Some(mode)) => {
            let mut ext = L2Extension::new(spec, mode);
            if let Some(bytes) = res.oracle_patch_bytes.as_deref() {
                ext = ext.with_oracle_patch(bytes);
            }
            Some(ext)
        }
        _ => None,
    };
    let l3 = res.policy.as_ref().map(|p| {
        L3Extension::new(p).with_observation(
            eval_ladder_policy::L3Observation::default().with_network_accessed(network_accessed),
        )
    });
    let l4 = match (
        res.obligations.as_ref(),
        res.lean_checker.as_ref(),
        res.lean_root.as_ref(),
    ) {
        (Some(m), Some(c), Some(root)) => Some(L4Extension::new(m, c, root)),
        _ => None,
    };
    BatchExtensions { l2, l3, l4 }
}

fn validate_level_flag_combination(args: &BatchArgs) -> Result<Vec<EvaluationLevel>, BatchError> {
    let levels = match args.track.as_deref() {
        Some(t) => track_levels(t)?,
        None => parse_levels(&args.levels)?,
    };
    let wants_l2 = levels.contains(&EvaluationLevel::L2Strengthened);
    let wants_l3 = levels.contains(&EvaluationLevel::L3PolicyConformant);
    let wants_l4 = levels.contains(&EvaluationLevel::L4Semantic);
    let unsupported: Vec<&EvaluationLevel> = levels
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
        return Err(BatchError::Levels(format!(
            "unsupported levels {unsupported:?}; supported: L0,L1,L2,L3,L4"
        )));
    }
    let paired = [
        (
            wants_l2,
            args.strengthening_spec.is_some(),
            "L2",
            "--strengthening-spec",
        ),
        (wants_l3, args.policy.is_some(), "L3", "--policy"),
        (wants_l4, args.obligations.is_some(), "L4", "--obligations"),
    ];
    for (wanted, supplied, level, flag) in paired {
        if wanted && !supplied {
            return Err(BatchError::Extensions(format!(
                "--levels includes {level} but {flag} was not supplied"
            )));
        }
        if !wanted && supplied {
            return Err(BatchError::Extensions(format!(
                "{flag} was supplied but --levels does not include {level}"
            )));
        }
    }
    if wants_l4 && args.lean_root.is_none() {
        return Err(BatchError::Extensions(
            "--levels includes L4 but --lean-root was not supplied".into(),
        ));
    }
    Ok(levels)
}

/// Run one panel entry end-to-end, returning a summary row.
///
/// Catches every recoverable error and maps it to a `BatchEntryRow`
/// with `status = Invalid`. This is the resilience contract: one bad
/// entry does not abort the batch.
#[allow(clippy::too_many_arguments)]
fn run_entry(
    entry: &PanelEntry,
    out_dir: &Path,
    levels: &[EvaluationLevel],
    extensions: &[&dyn LevelExtension],
    args: &BatchArgs,
) -> BatchEntryRow {
    // 1. Load task, candidate, patch from disk.
    let load_result = (|| -> anyhow::Result<(BenchmarkTask, CandidateResolution, Vec<u8>)> {
        let task_bytes = fs::read(&entry.task)
            .with_context(|| format!("reading task from {}", entry.task.display()))?;
        let task: BenchmarkTask = serde_json::from_slice(&task_bytes)
            .with_context(|| format!("parsing task at {}", entry.task.display()))?;
        let candidate_bytes = fs::read(&entry.candidate)
            .with_context(|| format!("reading candidate from {}", entry.candidate.display()))?;
        let candidate: CandidateResolution = serde_json::from_slice(&candidate_bytes)
            .with_context(|| format!("parsing candidate at {}", entry.candidate.display()))?;
        let patch_bytes = if entry.patch.exists() {
            fs::read(&entry.patch)
                .with_context(|| format!("reading patch from {}", entry.patch.display()))?
        } else {
            Vec::new()
        };
        Ok((task, candidate, patch_bytes))
    })();
    let (task, candidate, patch_bytes) = match load_result {
        Ok(t) => t,
        Err(e) => {
            return invalid_row(
                entry,
                None,
                serde_json::json!({
                    "l0": {"status": EvaluationStatus::Invalid, "primary_reason": "BATCH_ENTRY_INVALID"},
                }),
                format!("BATCH_LOAD_FAILED: {e:#}"),
            );
        }
    };

    let bundle_name = entry
        .bundle_name
        .clone()
        .unwrap_or_else(|| candidate.candidate_id.to_string());
    let bundle_dir = out_dir.join(&bundle_name);

    // Invoke the pipeline. Per the runner contract, `bundle_dir` must
    // be empty or absent.
    let pipeline_result = run_pipeline(
        &task,
        &candidate,
        &patch_bytes,
        &bundle_dir,
        entry,
        extensions,
        args,
        args.timeout_secs,
    );

    match pipeline_result {
        Ok(outcome) => ok_row(entry, &bundle_name, &outcome, levels),
        Err(e) => {
            let detail = format!("{e:#}");
            invalid_row(
                entry,
                Some(bundle_name),
                classify_invalid_levels(levels, &detail),
                format!("BATCH_PIPELINE_FAILED: {detail}"),
            )
        }
    }
}

fn run_entry_isolated(
    entry: &PanelEntry,
    out_dir: &Path,
    levels: &[EvaluationLevel],
    args: &BatchArgs,
    timeout_secs: u64,
) -> BatchEntryRow {
    let resources = match load_backing_resources(args) {
        Ok(r) => r,
        Err(e) => {
            return invalid_row(
                entry,
                entry.bundle_name.clone(),
                serde_json::json!({
                    "l0": {"status": EvaluationStatus::Invalid, "primary_reason": "BATCH_ENTRY_INVALID"},
                }),
                format!("BATCH_RESOURCE_FAILED: {e:#}"),
            );
        }
    };
    let extensions = build_extensions(&resources, args.network_accessed);
    let ext_refs = extensions.as_level_extensions();
    let mut args_override = args.clone();
    args_override.timeout_secs = timeout_secs;
    run_entry(entry, out_dir, levels, &ext_refs, &args_override)
}

fn run_pipeline(
    task: &BenchmarkTask,
    candidate: &CandidateResolution,
    patch_bytes: &[u8],
    bundle_dir: &Path,
    entry: &PanelEntry,
    extensions: &[&dyn LevelExtension],
    args: &BatchArgs,
    timeout_secs: u64,
) -> Result<PipelineOutcome> {
    if !entry.workspace_template.exists() {
        bail!(
            "workspace template does not exist: {}",
            entry.workspace_template.display()
        );
    }
    let staging_root =
        tempfile::tempdir().context("creating staging tempdir for the batch pipeline")?;
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
        wall_timeout: Some(Duration::from_secs(timeout_secs)),
    };
    let l1_strategy = parse_l1_strategy(&args.l1_strategy)?;
    let mut env: Vec<EnvVar> = Vec::new();
    if task.benchmark_id == BenchmarkId::RustSweBench {
        let root = args
            .rust_target_cache_root
            .clone()
            .unwrap_or_else(|| args.out.join(".cargo_target_cache"));
        let cache = root
            .join(task.repo_name.replace('/', "__"))
            .join(task.base_commit.as_str());
        fs::create_dir_all(&cache)
            .with_context(|| format!("creating rust target cache dir {}", cache.display()))?;
        env.push(EnvVar {
            name: "CARGO_TARGET_DIR".to_owned(),
            value: cache.display().to_string(),
        });
    }

    let inputs = PipelineInputs {
        task,
        candidate,
        patch_bytes,
        workspace_template: &entry.workspace_template,
        staging_root: staging_root.path(),
        bundle_dir,
        identity_seed: &seed,
        resource_limits,
        env: &env,
        extensions,
        l1_strategy,
    };

    let use_docker = matches!(
        task.benchmark_id,
        BenchmarkId::SweBenchVerified | BenchmarkId::SweBenchLive
    ) && !task.environment_ref.starts_with("local:");
    let outcome = if use_docker {
        let engine = DockerCliEngine;
        if args.deterministic_clock {
            let clock = FixedClock::deterministic();
            EvaluationPipeline::new(&engine, &scorer, &clock).run(inputs)
        } else {
            let clock = SystemClock;
            EvaluationPipeline::new(&engine, &scorer, &clock).run(inputs)
        }
    } else {
        let engine = LocalProcessEngine;
        if args.deterministic_clock {
            let clock = FixedClock::deterministic();
            EvaluationPipeline::new(&engine, &scorer, &clock).run(inputs)
        } else {
            let clock = SystemClock;
            EvaluationPipeline::new(&engine, &scorer, &clock).run(inputs)
        }
    }
    .map_err(anyhow::Error::new)?;
    Ok(outcome)
}

fn ok_row(
    entry: &PanelEntry,
    bundle_name: &str,
    outcome: &PipelineOutcome,
    _levels: &[EvaluationLevel],
) -> BatchEntryRow {
    let mut levels_obj = serde_json::Map::new();
    levels_obj.insert(
        "l0".into(),
        serde_json::json!({
            "status": outcome.l0.status,
            "primary_reason": outcome.l0.primary_reason,
        }),
    );
    levels_obj.insert(
        "l1".into(),
        serde_json::json!({
            "status": outcome.l1.status,
            "primary_reason": outcome.l1.primary_reason,
        }),
    );
    for ext in &outcome.extensions {
        levels_obj.insert(
            ext.level.short_code().to_ascii_lowercase(),
            serde_json::json!({
                "status": ext.status,
                "primary_reason": ext.primary_reason,
            }),
        );
    }
    BatchEntryRow {
        entry_id: entry
            .entry_id
            .clone()
            .unwrap_or_else(|| bundle_name.to_owned()),
        bundle_name: bundle_name.to_owned(),
        task_path: entry.task.display().to_string(),
        candidate_path: entry.candidate.display().to_string(),
        status: BatchEntryStatus::Ok,
        levels: serde_json::Value::Object(levels_obj),
        bundle_hash: Some(outcome.bundle_hash.clone()),
        error: None,
    }
}

fn invalid_row(
    entry: &PanelEntry,
    bundle_name: Option<String>,
    levels_obj: serde_json::Value,
    error_message: String,
) -> BatchEntryRow {
    let bundle_name = bundle_name
        .or_else(|| entry.bundle_name.clone())
        .or_else(|| entry.entry_id.clone())
        .unwrap_or_else(|| "unknown".into());
    let entry_id = entry
        .entry_id
        .clone()
        .unwrap_or_else(|| bundle_name.clone());
    BatchEntryRow {
        entry_id,
        bundle_name,
        task_path: entry.task.display().to_string(),
        candidate_path: entry.candidate.display().to_string(),
        status: BatchEntryStatus::Invalid,
        levels: levels_obj,
        bundle_hash: None,
        error: Some(error_message),
    }
}

fn classify_invalid_levels(levels: &[EvaluationLevel], detail: &str) -> serde_json::Value {
    let lower = detail.to_ascii_lowercase();
    let image_or_backend_unavailable =
        lower.contains("image not found") || lower.contains("no container backend is available");

    if image_or_backend_unavailable {
        let mut map = serde_json::Map::new();
        for lvl in levels {
            let key = lvl.short_code().to_ascii_lowercase();
            let primary = match lvl {
                EvaluationLevel::L0Official => "L0_OFFICIAL_INVALID",
                EvaluationLevel::L1TrustedRerun => "L1_HARNESS_ERROR",
                EvaluationLevel::L2Strengthened => "L2_ORACLE_UNAVAILABLE",
                EvaluationLevel::L3PolicyConformant => "PV_TRACE_INCOMPLETE",
                EvaluationLevel::L4Semantic => "L4_EXTRACTION_FAILED",
            };
            map.insert(
                key,
                serde_json::json!({"status": EvaluationStatus::Invalid, "primary_reason": primary}),
            );
        }
        return serde_json::Value::Object(map);
    }

    serde_json::json!({
        "l0": {"status": EvaluationStatus::Invalid, "primary_reason": "BATCH_ENTRY_INVALID"},
    })
}

fn parse_l1_strategy(s: &str) -> Result<L1Strategy, BatchError> {
    match s {
        "strict" => Ok(L1Strategy::StrictRerun),
        "smart_rust_reuse" => Ok(L1Strategy::SmartRustReuse),
        other => Err(BatchError::Extensions(format!(
            "--l1-strategy must be strict|smart_rust_reuse, got {other}"
        ))),
    }
}

fn has_level_result(row: &BatchEntryRow, level: EvaluationLevel) -> bool {
    let key = level.short_code().to_ascii_lowercase();
    row.levels
        .as_object()
        .is_some_and(|m| m.get(&key).is_some())
}

fn parse_existing_summary(out_dir: &Path) -> Option<BatchSummary> {
    let p = out_dir.join(BATCH_SUMMARY_FILE);
    let bytes = fs::read(p).ok()?;
    serde_json::from_slice::<BatchSummary>(&bytes).ok()
}

fn result_file_for_level(level: EvaluationLevel) -> &'static str {
    match level {
        EvaluationLevel::L0Official => "official_results.json",
        EvaluationLevel::L1TrustedRerun => "l1_trusted_rerun_results.json",
        EvaluationLevel::L2Strengthened => "strengthened_results.json",
        EvaluationLevel::L3PolicyConformant => "policy_results.json",
        EvaluationLevel::L4Semantic => "proof_results.json",
    }
}

fn try_row_from_existing_bundle(
    entry: &PanelEntry,
    bundle_name: &str,
    bundle_dir: &Path,
    levels: &[EvaluationLevel],
) -> Option<BatchEntryRow> {
    let run_manifest_path = bundle_dir.join("run_manifest.json");
    let run_manifest_bytes = fs::read(&run_manifest_path).ok()?;
    let _manifest: RunManifest = serde_json::from_slice(&run_manifest_bytes).ok()?;

    let mut levels_obj = serde_json::Map::new();
    for level in levels {
        let result_path = bundle_dir.join(result_file_for_level(*level));
        let bytes = fs::read(&result_path).ok()?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        let status = value.get("status")?.clone();
        let primary_reason = value.get("primary_reason")?.clone();
        levels_obj.insert(
            level.short_code().to_ascii_lowercase(),
            serde_json::json!({
                "status": status,
                "primary_reason": primary_reason,
            }),
        );
    }

    let bundle_hash = fs::read(bundle_dir.join("artifact_hashes.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|v| v.get("bundle_hash").cloned())
        .and_then(|v| serde_json::from_value::<Sha256Digest>(v).ok());

    Some(BatchEntryRow {
        entry_id: entry
            .entry_id
            .clone()
            .unwrap_or_else(|| bundle_name.to_owned()),
        bundle_name: bundle_name.to_owned(),
        task_path: entry.task.display().to_string(),
        candidate_path: entry.candidate.display().to_string(),
        status: BatchEntryStatus::Ok,
        levels: serde_json::Value::Object(levels_obj),
        bundle_hash,
        error: None,
    })
}

fn workload_key_for_entry(entry: &PanelEntry) -> Option<String> {
    let task_bytes = fs::read(&entry.task).ok()?;
    let task: BenchmarkTask = serde_json::from_slice(&task_bytes).ok()?;
    let patch = if entry.patch.exists() {
        fs::read(&entry.patch).ok()?
    } else {
        Vec::new()
    };
    let patch_hash = eval_ladder_core::digest(&patch);
    Some(format!(
        "{}|{}|{}",
        task.repo_name, task.base_commit, patch_hash
    ))
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst).with_context(|| format!("removing existing {}", dst.display()))?;
    }
    fs::create_dir_all(dst).with_context(|| format!("creating {}", dst.display()))?;
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&out)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &out)?;
        }
    }
    Ok(())
}

/// Top-level entrypoint: `eval-ladder evaluate batch`.
pub fn run_batch(args: BatchArgs) -> Result<()> {
    fs::create_dir_all(&args.out)
        .with_context(|| format!("creating batch out dir {}", args.out.display()))?;

    let levels = validate_level_flag_combination(&args)
        .map_err(|e| anyhow::anyhow!("batch flag validation: {e}"))?;
    let panel = load_panel(&args.input)
        .with_context(|| format!("loading panel from {}", args.input.display()))?;

    info!(
        panel = %args.input.display(),
        out = %args.out.display(),
        total = panel.len(),
        ?levels,
        "evaluate batch: running pipeline"
    );

    let existing_summary = if args.resume || args.adaptive_timeouts {
        parse_existing_summary(&args.out)
    } else {
        None
    };
    let mut existing_rows: HashMap<String, BatchEntryRow> = HashMap::new();
    if let Some(s) = &existing_summary {
        for row in &s.entries {
            existing_rows.insert(row.bundle_name.clone(), row.clone());
        }
    }

    // Batch-wide resources and extensions (loaded once) for single-job path.
    let resources = load_backing_resources(&args)?;
    let extensions = build_extensions(&resources, args.network_accessed);
    let ext_refs = extensions.as_level_extensions();

    // If the deterministic clock is set we do not embed wall-clock
    // timestamps in the summary, so the rerun invariant holds for the
    // summary file as well as the individual bundles.
    let started_at = if args.deterministic_clock {
        None
    } else {
        Some(Utc::now())
    };

    let default_short_timeout = args
        .short_timeout_secs
        .unwrap_or((args.timeout_secs / 6).max(60));
    let mut rows: Vec<BatchEntryRow> = Vec::with_capacity(panel.len());
    let mut seen_workloads: HashMap<String, String> = HashMap::new();
    let jobs = args.jobs.max(1);
    for (chunk_idx, chunk) in panel.chunks(jobs).enumerate() {
        let mut handles = Vec::with_capacity(chunk.len());
        for (offset, entry) in chunk.iter().enumerate() {
            let i = chunk_idx * jobs + offset;
            info!(idx = i + 1, total = panel.len(), "batch entry");
            let bundle_name = entry.bundle_name.clone().unwrap_or_else(|| {
                entry
                    .entry_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_owned())
            });
            let bundle_dir = args.out.join(&bundle_name);

            if args.resume && bundle_dir.join("run_manifest.json").exists() {
                if let Some(row) = existing_rows.get(&bundle_name).filter(|r| {
                    r.status == BatchEntryStatus::Ok && levels.iter().all(|lvl| has_level_result(r, *lvl))
                }) {
                    rows.push(row.clone());
                    continue;
                }
                if let Some(row) = try_row_from_existing_bundle(entry, &bundle_name, &bundle_dir, &levels)
                {
                    rows.push(row);
                    continue;
                }
            }

            if args.dedupe_workloads {
                if let Some(key) = workload_key_for_entry(entry) {
                    if let Some(source_bundle) = seen_workloads.get(&key) {
                        let source_dir = args.out.join(source_bundle);
                        if source_dir.join("run_manifest.json").exists() {
                            let _ = copy_dir_recursive(&source_dir, &bundle_dir);
                            if let Some(src_row) = existing_rows
                                .get(source_bundle)
                                .or_else(|| rows.iter().find(|r| &r.bundle_name == source_bundle))
                            {
                                let mut cloned = src_row.clone();
                                cloned.bundle_name = bundle_name.clone();
                                cloned.entry_id = entry
                                    .entry_id
                                    .clone()
                                    .unwrap_or_else(|| bundle_name.clone());
                                rows.push(cloned);
                                continue;
                            }
                        }
                    } else {
                        seen_workloads.insert(key, bundle_name.clone());
                    }
                }
            }

            let mut timeout_secs = args.timeout_secs;
            if args.adaptive_timeouts {
                if let Some(prev) = existing_rows.get(&bundle_name) {
                    let l0_reason = prev
                        .levels
                        .get("l0")
                        .and_then(|v| v.get("primary_reason"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    if matches!(
                        l0_reason,
                        "L0_OFFICIAL_FAIL" | "L1_HARNESS_ERROR" | "PV_EDIT_SCOPE"
                    ) {
                        timeout_secs = default_short_timeout;
                    }
                }
            }
            if jobs == 1 {
                let mut args_override = args.clone();
                args_override.timeout_secs = timeout_secs;
                let row = run_entry(entry, &args.out, &levels, &ext_refs, &args_override);
                if row.status == BatchEntryStatus::Invalid {
                    warn!(
                        entry_id = %row.entry_id,
                        error = row.error.as_deref().unwrap_or("?"),
                        "batch entry invalid; continuing"
                    );
                }
                rows.push(row);
            } else {
                let entry_cloned = entry.clone();
                let out = args.out.clone();
                let levels_cloned = levels.clone();
                let args_cloned = args.clone();
                handles.push(thread::spawn(move || {
                    run_entry_isolated(
                        &entry_cloned,
                        &out,
                        &levels_cloned,
                        &args_cloned,
                        timeout_secs,
                    )
                }));
            }
        }
        for h in handles {
            if let Ok(row) = h.join() {
                if row.status == BatchEntryStatus::Invalid {
                    warn!(
                        entry_id = %row.entry_id,
                        error = row.error.as_deref().unwrap_or("?"),
                        "batch entry invalid; continuing"
                    );
                }
                rows.push(row);
            }
        }
    }

    rows.sort_by(|a, b| a.bundle_name.cmp(&b.bundle_name));

    let finished_at = if args.deterministic_clock {
        None
    } else {
        Some(Utc::now())
    };
    let total_entries = u64::try_from(rows.len()).unwrap_or(u64::MAX);
    let ok_entries = u64::try_from(
        rows.iter()
            .filter(|r| r.status == BatchEntryStatus::Ok)
            .count(),
    )
    .unwrap_or(u64::MAX);
    let invalid_entries = total_entries.saturating_sub(ok_entries);

    let summary = BatchSummary {
        schema_version: BATCH_SUMMARY_SCHEMA_VERSION,
        evaluator_version: EVALUATOR_VERSION,
        levels,
        total_entries,
        ok_entries,
        invalid_entries,
        entries: rows,
        started_at,
        finished_at,
    };

    let canonical = canonical_json(&summary).map_err(BatchError::from)?;
    let summary_path = args.out.join(BATCH_SUMMARY_FILE);
    fs::write(&summary_path, &canonical)
        .with_context(|| format!("writing batch summary to {}", summary_path.display()))?;

    if summary.ok_entries == 0 {
        error!(total = summary.total_entries, "every batch entry failed");
        bail!(
            "eval-ladder evaluate batch: every panel entry failed; see {}",
            summary_path.display()
        );
    }

    // Echo a compact status line for operators; full detail is in the
    // summary file.
    println!(
        "batch: total={} ok={} invalid={} summary={}",
        summary.total_entries,
        summary.ok_entries,
        summary.invalid_entries,
        summary_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use eval_ladder_core::{
        BenchmarkId, BenchmarkLanguage, BenchmarkTask, CandidateId, CandidateResolution,
        ContextMode, GenerationMetadata, GenerationMode, PatchFormat, TaskId,
    };
    use eval_ladder_runner::EVAL_LADDER_NAMESPACE;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn fixture_task(tag: &str) -> BenchmarkTask {
        BenchmarkTask::new(
            BenchmarkId::SweBenchVerified,
            TaskId::new(format!("fixture__milestone-h-{tag}")).unwrap(),
            "fixture/milestone-h",
            "1",
            "Milestone H fixture",
            "Batch fixture exercised by milestone_h_acceptance.",
            "deadbeefcafe0000000000000000000000000000",
            "local:fixture",
            "cargo --version",
            BenchmarkLanguage::Rust,
            "https://example.test/fixture/milestone-h",
            Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
        )
    }

    fn fixture_candidate(task: &BenchmarkTask, tag: &str) -> CandidateResolution {
        let uid = Uuid::new_v5(
            &EVAL_LADDER_NAMESPACE,
            format!("milestone-h-candidate-{tag}").as_bytes(),
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

    fn seed_entry(root: &Path, tag: &str) -> PanelEntry {
        let entry_root = root.join(format!("entry-{tag}"));
        fs::create_dir_all(&entry_root).unwrap();
        let task = fixture_task(tag);
        let candidate = fixture_candidate(&task, tag);
        let task_path = entry_root.join("task.json");
        let candidate_path = entry_root.join("candidate.json");
        let patch_path = entry_root.join("patch.diff");
        let workspace_template = entry_root.join("workspace");
        fs::create_dir_all(&workspace_template).unwrap();
        fs::write(
            workspace_template.join("README.md"),
            format!("fixture workspace {tag}\n"),
        )
        .unwrap();
        fs::write(&task_path, serde_json::to_vec_pretty(&task).unwrap()).unwrap();
        fs::write(
            &candidate_path,
            serde_json::to_vec_pretty(&candidate).unwrap(),
        )
        .unwrap();
        fs::write(&patch_path, b"").unwrap();
        PanelEntry {
            task: task_path,
            candidate: candidate_path,
            patch: patch_path,
            workspace_template,
            bundle_name: Some(format!("bundle-{tag}")),
            entry_id: Some(format!("entry-{tag}")),
        }
    }

    fn build_panel(root: &Path, tags: &[&str]) -> PathBuf {
        let panel_path = root.join("panel.jsonl");
        let mut buf = String::new();
        for tag in tags {
            let entry = seed_entry(root, tag);
            let line = serde_json::to_string(&entry).unwrap();
            buf.push_str(&line);
            buf.push('\n');
        }
        fs::write(&panel_path, buf).unwrap();
        panel_path
    }

    fn write_dummy_config(path: &Path) {
        let toml = "name = \"fixture\"\nschema_version = 1\n";
        fs::write(path, toml).unwrap();
    }

    fn default_batch_args(panel: PathBuf, out: PathBuf, config: PathBuf) -> BatchArgs {
        BatchArgs {
            input: panel,
            levels: "L0,L1".into(),
            config,
            out,
            deterministic_clock: true,
            timeout_secs: 60,
            short_timeout_secs: None,
            adaptive_timeouts: false,
            resume: false,
            jobs: 1,
            track: None,
            l1_strategy: "strict".into(),
            rust_target_cache_root: None,
            dedupe_workloads: false,
            seed_tag: "milestone-h".into(),
            strengthening_spec: None,
            strengthening_mode: "full_l2".into(),
            oracle_patch: None,
            policy: None,
            network_accessed: false,
            obligations: None,
            lean_root: None,
        }
    }

    #[test]
    fn milestone_h_batch_summary_is_deterministic() {
        let tmp_a = TempDir::new().unwrap();
        let panel_a = build_panel(tmp_a.path(), &["alpha", "beta", "gamma"]);
        let out_a = tmp_a.path().join("out");
        let config_a = tmp_a.path().join("config.toml");
        write_dummy_config(&config_a);
        let args_a = default_batch_args(panel_a, out_a.clone(), config_a);
        run_batch(args_a).expect("batch a must succeed");
        let summary_a = fs::read(out_a.join(BATCH_SUMMARY_FILE)).unwrap();

        let tmp_b = TempDir::new().unwrap();
        let panel_b = build_panel(tmp_b.path(), &["alpha", "beta", "gamma"]);
        let out_b = tmp_b.path().join("out");
        let config_b = tmp_b.path().join("config.toml");
        write_dummy_config(&config_b);
        let args_b = default_batch_args(panel_b, out_b.clone(), config_b);
        run_batch(args_b).expect("batch b must succeed");
        let summary_b = fs::read(out_b.join(BATCH_SUMMARY_FILE)).unwrap();

        // The batch summary embeds the panel-declared task_path /
        // candidate_path strings which depend on the per-run tempdir.
        // Strip them out before comparing so that we assert determinism
        // of the *content-bearing* fields (bundle_hash, status, level
        // verdicts, counts).
        let a_norm = normalize_summary_for_determinism(&summary_a);
        let b_norm = normalize_summary_for_determinism(&summary_b);
        assert_eq!(
            a_norm, b_norm,
            "batch summary content must be byte-identical after tempdir normalization"
        );

        // The bundle hashes themselves (content-addressed) must match
        // across runs. Verify by parsing the summaries.
        let summary_a: BatchSummary = serde_json::from_slice(&summary_a).unwrap();
        let summary_b: BatchSummary = serde_json::from_slice(&summary_b).unwrap();
        assert_eq!(summary_a.ok_entries, 3);
        assert_eq!(summary_a.invalid_entries, 0);
        assert_eq!(summary_a.total_entries, 3);
        for (a, b) in summary_a.entries.iter().zip(summary_b.entries.iter()) {
            assert_eq!(
                a.bundle_hash, b.bundle_hash,
                "bundle_hash must be stable across reruns for {}",
                a.bundle_name
            );
            assert_eq!(a.status, b.status);
            assert_eq!(a.levels, b.levels);
        }
    }

    fn normalize_summary_for_determinism(bytes: &[u8]) -> Vec<u8> {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        if let Some(entries) = value.get_mut("entries").and_then(|v| v.as_array_mut()) {
            for entry in entries {
                if let Some(obj) = entry.as_object_mut() {
                    obj.insert(
                        "task_path".into(),
                        serde_json::Value::String("<tempdir>/task.json".into()),
                    );
                    obj.insert(
                        "candidate_path".into(),
                        serde_json::Value::String("<tempdir>/candidate.json".into()),
                    );
                }
            }
        }
        canonical_json(&value).unwrap()
    }

    #[test]
    fn milestone_h_batch_is_resilient_to_one_bad_entry() {
        let tmp = TempDir::new().unwrap();
        let panel_path = tmp.path().join("panel.jsonl");
        let good = seed_entry(tmp.path(), "ok");
        let bad = PanelEntry {
            task: tmp.path().join("does-not-exist.json"),
            candidate: tmp.path().join("does-not-exist.json"),
            patch: tmp.path().join("does-not-exist.diff"),
            workspace_template: tmp.path().join("does-not-exist"),
            bundle_name: Some("bundle-bad".into()),
            entry_id: Some("entry-bad".into()),
        };
        let buf = format!(
            "{}\n{}\n",
            serde_json::to_string(&good).unwrap(),
            serde_json::to_string(&bad).unwrap(),
        );
        fs::write(&panel_path, buf).unwrap();

        let out = tmp.path().join("out");
        let config = tmp.path().join("config.toml");
        write_dummy_config(&config);
        let args = default_batch_args(panel_path, out.clone(), config);
        run_batch(args).expect("batch must continue past the bad entry");

        let summary: BatchSummary =
            serde_json::from_slice(&fs::read(out.join(BATCH_SUMMARY_FILE)).unwrap()).unwrap();
        assert_eq!(summary.total_entries, 2);
        assert_eq!(summary.ok_entries, 1);
        assert_eq!(summary.invalid_entries, 1);
        let bad_row = summary
            .entries
            .iter()
            .find(|r| r.bundle_name == "bundle-bad")
            .expect("bad entry must be present in summary");
        assert_eq!(bad_row.status, BatchEntryStatus::Invalid);
        assert!(bad_row
            .error
            .as_deref()
            .unwrap_or("")
            .starts_with("BATCH_LOAD_FAILED"));
    }

    #[test]
    fn milestone_h_resume_uses_run_manifest_without_summary_row() {
        let tmp = TempDir::new().unwrap();
        let panel = build_panel(tmp.path(), &["alpha"]);
        let out = tmp.path().join("out");
        let config = tmp.path().join("config.toml");
        write_dummy_config(&config);

        // First run creates bundle + summary.
        let args = default_batch_args(panel.clone(), out.clone(), config.clone());
        run_batch(args).expect("initial batch run must succeed");

        // Simulate stale/missing summary state while bundle artifacts remain.
        fs::remove_file(out.join(BATCH_SUMMARY_FILE)).unwrap();

        // Resume should skip/reconstruct from bundle artifacts, not rerun
        // into a non-empty bundle_dir.
        let mut resume_args = default_batch_args(panel, out.clone(), config);
        resume_args.resume = true;
        run_batch(resume_args).expect("resume must succeed from run_manifest evidence");

        let summary: BatchSummary =
            serde_json::from_slice(&fs::read(out.join(BATCH_SUMMARY_FILE)).unwrap()).unwrap();
        assert_eq!(summary.total_entries, 1);
        assert_eq!(summary.ok_entries, 1);
        assert_eq!(summary.invalid_entries, 0);
    }

    #[test]
    fn panel_loader_parses_relative_paths() {
        let tmp = TempDir::new().unwrap();
        let panel_path = tmp.path().join("panel.jsonl");
        fs::write(
            &panel_path,
            concat!(
                "# a comment\n",
                "\n",
                "{\"task\": \"t.json\", \"candidate\": \"c.json\", ",
                "\"patch\": \"p.diff\", \"workspace_template\": \"w\"}\n"
            ),
        )
        .unwrap();
        let panel = load_panel(&panel_path).unwrap();
        assert_eq!(panel.len(), 1);
        assert!(panel[0].task.is_absolute());
        assert!(panel[0].task.ends_with("t.json"));
    }

    #[test]
    fn panel_loader_rejects_empty_panel() {
        let tmp = TempDir::new().unwrap();
        let panel_path = tmp.path().join("panel.jsonl");
        fs::write(&panel_path, "# just comments\n\n").unwrap();
        let err = load_panel(&panel_path).unwrap_err();
        assert!(matches!(err, BatchError::EmptyPanel(_)));
    }

    #[test]
    fn panel_loader_rejects_unknown_fields() {
        let tmp = TempDir::new().unwrap();
        let panel_path = tmp.path().join("panel.jsonl");
        fs::write(
            &panel_path,
            "{\"task\": \"t\", \"candidate\": \"c\", \"patch\": \"p\", \
             \"workspace_template\": \"w\", \"bogus_field\": true}\n",
        )
        .unwrap();
        let err = load_panel(&panel_path).unwrap_err();
        assert!(matches!(err, BatchError::Json { .. }));
    }
}
