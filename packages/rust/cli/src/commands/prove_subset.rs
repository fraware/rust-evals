//! `eval-ladder prove-subset ...`
//!
//! Runs the L4 Lean checker over every evidence bundle in
//! `--candidate-dir` whose task has a matching entry in
//! `--subset`. The command writes one L4 report per bundle under
//! `<bundle>/proof_results.json` (alongside the other L4 artifacts
//! produced by `evaluate candidate --levels L0,L1,L2,L3,L4`), *provided
//! the bundle does not already contain one*. Bundles that already
//! carry `proof_results.json` are skipped to preserve their seal.
//!
//! This subcommand is the batched alternative to running `evaluate
//! candidate --levels L4 --obligations ... --lean-root ...` once per
//! candidate: it reuses the same
//! [`eval_ladder_lean::ExternalProcessChecker`] for every obligation
//! and short-circuits on tasks without an obligation.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Args;
use eval_ladder_core::{CandidateResolution, EvaluatorVersion, EVALUATOR_VERSION};
use eval_ladder_lean::{
    checker::LeanCheckContext, ExternalProcessChecker, LeanChecker, ObligationManifest,
    ProofObligation, ProofReport, PROOF_REPORT_SCHEMA_VERSION,
};
use serde::Serialize;
use tracing::{info, warn};

/// Arguments for `prove-subset`.
#[derive(Debug, Args)]
pub struct ProveSubsetArgs {
    /// Path to the proof subset manifest (JSONL; one `ProofObligation` per line).
    #[arg(long)]
    pub subset: PathBuf,
    /// Directory containing evaluated candidate bundles produced by
    /// `eval-ladder evaluate batch` (or any `evaluate candidate` run).
    /// Each immediate subdirectory is treated as one bundle.
    #[arg(long)]
    pub candidate_dir: PathBuf,
    /// Path to the Lean project root (typically
    /// `packages/lean/EvalLadder`). Passed as `cwd` to the checker
    /// process declared by each obligation.
    #[arg(long)]
    pub lean_root: PathBuf,
    /// Optional summary file. When set, a JSON summary of every
    /// bundle's outcome is written there. The summary is
    /// deterministic: bundles are listed in sorted-path order.
    #[arg(long)]
    pub summary: Option<PathBuf>,
    /// Overwrite `proof_results.json` when a bundle already has one.
    /// Defaults to `false`, which treats existing L4 reports as sealed.
    #[arg(long, default_value_t = false)]
    pub overwrite: bool,
}

/// Entrypoint.
pub fn run(args: ProveSubsetArgs) -> Result<()> {
    if !args.subset.is_file() {
        bail!(
            "obligations manifest does not exist or is not a file: {}",
            args.subset.display()
        );
    }
    if !args.candidate_dir.is_dir() {
        bail!(
            "candidate directory does not exist or is not a directory: {}",
            args.candidate_dir.display()
        );
    }
    if !args.lean_root.is_dir() {
        bail!(
            "lean root does not exist or is not a directory: {}",
            args.lean_root.display()
        );
    }

    let manifest = ObligationManifest::from_path(&args.subset)
        .with_context(|| format!("loading obligations from {}", args.subset.display()))?;
    info!(
        subset = %args.subset.display(),
        candidate_dir = %args.candidate_dir.display(),
        lean_root = %args.lean_root.display(),
        obligations = manifest.len(),
        "prove-subset: starting L4 batch"
    );

    let checker = ExternalProcessChecker::new(&args.lean_root);

    let mut bundle_paths: Vec<PathBuf> = fs::read_dir(&args.candidate_dir)
        .with_context(|| {
            format!(
                "reading candidate directory {}",
                args.candidate_dir.display()
            )
        })?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    bundle_paths.sort();

    let mut outcomes: Vec<ProveSubsetBundleOutcome> = Vec::with_capacity(bundle_paths.len());
    for bundle in &bundle_paths {
        match process_bundle(bundle, &manifest, &checker, &args.lean_root, args.overwrite) {
            Ok(o) => outcomes.push(o),
            Err(e) => {
                warn!(bundle = %bundle.display(), error = %e, "bundle skipped");
                outcomes.push(ProveSubsetBundleOutcome {
                    bundle: bundle.clone(),
                    action: "error".into(),
                    task_id: None,
                    obligation_id: None,
                    status: None,
                    code: None,
                    message: Some(e.to_string()),
                });
            }
        }
    }

    let summary = ProveSubsetSummary {
        schema_version: 1,
        evaluator_version: EVALUATOR_VERSION,
        obligations: manifest.len(),
        bundles: outcomes.len(),
        results: outcomes,
    };
    let rendered = serde_json::to_string_pretty(&summary)?;
    if let Some(path) = &args.summary {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("creating summary parent directory {}", parent.display())
                })?;
            }
        }
        fs::write(path, rendered.as_bytes())
            .with_context(|| format!("writing summary to {}", path.display()))?;
    } else {
        println!("{rendered}");
    }

    Ok(())
}

fn process_bundle(
    bundle: &Path,
    manifest: &ObligationManifest,
    checker: &ExternalProcessChecker,
    lean_root: &Path,
    overwrite: bool,
) -> Result<ProveSubsetBundleOutcome> {
    let candidate_path = bundle.join("candidate_resolution.json");
    if !candidate_path.is_file() {
        bail!(
            "bundle {} is missing candidate_resolution.json",
            bundle.display()
        );
    }
    let candidate_bytes = fs::read(&candidate_path)
        .with_context(|| format!("reading {}", candidate_path.display()))?;
    let candidate: CandidateResolution = serde_json::from_slice(&candidate_bytes)
        .with_context(|| format!("parsing {}", candidate_path.display()))?;
    let task_id = candidate.task_id.as_str().to_owned();

    let Some(obligation) = manifest.get(&task_id).cloned() else {
        return Ok(ProveSubsetBundleOutcome {
            bundle: bundle.to_path_buf(),
            action: "skipped_no_obligation".into(),
            task_id: Some(task_id),
            obligation_id: None,
            status: None,
            code: None,
            message: Some("task has no obligation in the manifest".into()),
        });
    };

    let report_path = bundle.join("proof_results.json");
    if report_path.is_file() && !overwrite {
        return Ok(ProveSubsetBundleOutcome {
            bundle: bundle.to_path_buf(),
            action: "skipped_already_sealed".into(),
            task_id: Some(task_id),
            obligation_id: Some(obligation.obligation_id.clone()),
            status: None,
            code: None,
            message: Some("proof_results.json already present; rerun with --overwrite".into()),
        });
    }

    let patch_path = bundle.join("patch.diff");
    let patch_bytes = if patch_path.is_file() {
        fs::read(&patch_path).with_context(|| format!("reading {}", patch_path.display()))?
    } else {
        Vec::new()
    };

    // The workspace used at evaluation time is not part of the bundle
    // contract (bundles intentionally do not re-ship the repo). The
    // checker is invoked with the Lean project root as cwd and is
    // expected to reference obligation-local fixtures via
    // `witness_inputs`; the bundle path is the best available proxy
    // for "workspace" at this stage of the pipeline.
    let lctx = LeanCheckContext {
        lean_root,
        workspace: bundle,
        patch_bytes: &patch_bytes,
    };

    let outcome = checker
        .check(&obligation, &lctx)
        .with_context(|| format!("running Lean checker for task {task_id}"))?;

    let now = chrono::Utc::now();
    let report = ProofReport {
        schema_version: PROOF_REPORT_SCHEMA_VERSION,
        evaluator_version: EVALUATOR_VERSION,
        obligation: Some(obligation.clone()),
        outcome: Some(outcome.clone()),
        status: outcome.status,
        code: outcome.code.clone(),
        message: outcome.message.clone(),
        duration_ms: 0,
        started_at: now,
        finished_at: now,
    };
    let bytes = eval_ladder_core::canonical_json(&report)
        .map_err(|e| anyhow::anyhow!("serializing proof report: {e}"))?;
    let mut out = bytes;
    out.push(b'\n');
    fs::write(&report_path, &out).with_context(|| format!("writing {}", report_path.display()))?;

    Ok(ProveSubsetBundleOutcome {
        bundle: bundle.to_path_buf(),
        action: if overwrite { "overwritten" } else { "written" }.into(),
        task_id: Some(task_id),
        obligation_id: Some(obligation.obligation_id),
        status: Some(format!("{:?}", outcome.status).to_lowercase()),
        code: Some(outcome.code),
        message: Some(outcome.message),
    })
}

#[derive(Serialize)]
struct ProveSubsetSummary {
    schema_version: u32,
    evaluator_version: EvaluatorVersion,
    obligations: usize,
    bundles: usize,
    results: Vec<ProveSubsetBundleOutcome>,
}

#[derive(Serialize)]
struct ProveSubsetBundleOutcome {
    bundle: PathBuf,
    action: String,
    task_id: Option<String>,
    obligation_id: Option<String>,
    status: Option<String>,
    code: Option<String>,
    message: Option<String>,
}

// Keep `ProofObligation` in the import graph so doc-tests and rustdoc
// references remain valid even if the function signature changes.
const _: fn() = || {
    fn assert_sync<T: Sync>() {}
    assert_sync::<ProofObligation>();
};
