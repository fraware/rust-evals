//! `eval-ladder demo run` - the 15-minute reproducibility slice.
//!
//! Milestone K.
//!
//! The demo command is the "is it alive?" experience for reviewers. It
//! materializes a tiny, wholly synthetic benchmark slice (no upstream
//! datasets, no network, no container runtime), drives the full batch
//! pipeline over it with a deterministic clock, emits the paper-export
//! tables, and finally re-verifies every byte of the produced bundles
//! in-process. The entire flow runs from a single subcommand with no
//! additional configuration.
//!
//! Why in-process composition:
//!
//! - Spawning subprocesses would force the demo to know how the CLI was
//!   installed, which path to find the binary, and how to thread working
//!   directories. The library functions behind each subcommand
//!   (`batch::run_batch`, `analysis::write_paper_exports`,
//!   `verify::run_run_dir`) are the canonical entry points. The demo
//!   calls them the same way the subcommands do.
//! - Keeping the demo inside the same process guarantees that any
//!   future change to the public evaluator contract fails the Milestone
//!   K acceptance test immediately, not on a separate CI leg.
//!
//! Determinism:
//!
//! - Every generated input (task, candidate, patch, workspace, panel)
//!   is a pure function of the `--out` directory layout. Two demo runs
//!   against the same `--out` produce byte-identical bundle hashes and
//!   a byte-identical `verify_report.json` (modulo absolute path
//!   fields, which are normalized out by the acceptance test).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use clap::{Args, Subcommand};
use eval_ladder_analysis::{load_bundle_dir, paper_export::write_paper_exports, LoadOptions};
use eval_ladder_core::{
    BenchmarkId, BenchmarkLanguage, BenchmarkTask, CandidateId, CandidateResolution, ContextMode,
    GenerationMetadata, GenerationMode, PatchFormat, TaskId,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::commands::batch::{run_batch, BatchArgs, PanelEntry};
use crate::commands::verify::{run_run_dir, VerifyRunDirArgs};

/// Subcommands under `demo`.
#[derive(Debug, Subcommand)]
pub enum DemoCmd {
    /// Run the full reproducibility slice end-to-end.
    Run(DemoRunArgs),
}

/// Arguments for `demo run`.
#[derive(Debug, Args)]
pub struct DemoRunArgs {
    /// Output directory; created if missing. All generated inputs,
    /// bundles, paper exports, and verification reports live under
    /// this root.
    #[arg(long)]
    pub out: PathBuf,

    /// Number of synthetic tasks to materialize. Kept small by default
    /// so the demo runs in under a minute on a developer laptop.
    #[arg(long, default_value_t = 3)]
    pub tasks: u32,

    /// Skip the analysis step. Useful for smoke tests where only the
    /// batch + verify invariants matter.
    #[arg(long, default_value_t = false)]
    pub skip_analyze: bool,
}

/// Deterministic namespace for demo IDs; chosen once and pinned.
const DEMO_NAMESPACE: Uuid = Uuid::from_bytes([
    0x7c, 0xaa, 0x10, 0x33, 0x48, 0xd9, 0x4b, 0xe0, 0x8e, 0x4d, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44,
]);

/// Stable timestamp for all demo-generated artifacts. Using a fixed
/// wall-clock value removes the last source of non-determinism
/// (`BenchmarkTask::added_at`).
fn demo_timestamp() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()
}

/// Summary printed after a successful run. Canonicalizable for tests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DemoSummary {
    /// Number of synthetic tasks evaluated.
    pub tasks: u32,
    /// Absolute path to the generated panel JSONL.
    pub panel_path: String,
    /// Absolute path to the bundles directory.
    pub bundles_dir: String,
    /// Absolute path to the paper-export directory (empty if skipped).
    pub paper_dir: String,
    /// Absolute path to the verify report.
    pub verify_report_path: String,
    /// Wall-clock duration of the demo run, milliseconds.
    pub wall_ms: u64,
}

/// Entrypoint.
pub fn run(cmd: DemoCmd) -> Result<()> {
    match cmd {
        DemoCmd::Run(args) => run_demo(args).map(|_| ()),
    }
}

/// Public so the acceptance test can assert on the returned summary.
pub fn run_demo(args: DemoRunArgs) -> Result<DemoSummary> {
    let start = Instant::now();
    fs::create_dir_all(&args.out)
        .with_context(|| format!("creating demo out dir {}", args.out.display()))?;
    let out = args.out.canonicalize().unwrap_or_else(|_| args.out.clone());

    let inputs_dir = out.join("inputs");
    let bundles_dir = out.join("bundles");
    let paper_dir = out.join("paper");

    info!(
        tasks = args.tasks,
        out = %out.display(),
        "demo: generating synthetic panel"
    );
    let panel_path = seed_inputs(&inputs_dir, args.tasks)?;
    let config_path = write_config(&inputs_dir)?;

    info!(panel = %panel_path.display(), "demo: running batch");
    let batch_args = BatchArgs {
        input: panel_path.clone(),
        levels: "L0,L1".into(),
        config: config_path,
        out: bundles_dir.clone(),
        deterministic_clock: true,
        timeout_secs: 120,
        seed_tag: "milestone-k-demo".into(),
        strengthening_spec: None,
        strengthening_mode: "full_l2".into(),
        oracle_patch: None,
        policy: None,
        network_accessed: false,
        obligations: None,
        lean_root: None,
    };
    run_batch(batch_args).context("demo: batch run failed")?;

    let paper_dir_str = if args.skip_analyze {
        String::new()
    } else {
        info!(bundles = %bundles_dir.display(), "demo: running paper export");
        run_paper_export(&bundles_dir, &paper_dir)?;
        paper_dir.display().to_string()
    };

    info!(bundles = %bundles_dir.display(), "demo: verifying bundles");
    run_run_dir(VerifyRunDirArgs {
        run_dir: bundles_dir.clone(),
        out: None,
        fail_fast: false,
    })
    .context("demo: verification failed")?;

    let summary = DemoSummary {
        tasks: args.tasks,
        panel_path: panel_path.display().to_string(),
        bundles_dir: bundles_dir.display().to_string(),
        paper_dir: paper_dir_str,
        verify_report_path: bundles_dir.join("verify_report.json").display().to_string(),
        wall_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
    };

    println!(
        "demo: ok ({} tasks in {} ms)",
        summary.tasks, summary.wall_ms
    );
    println!("  panel:          {}", summary.panel_path);
    println!("  bundles:        {}", summary.bundles_dir);
    if !summary.paper_dir.is_empty() {
        println!("  paper export:   {}", summary.paper_dir);
    }
    println!("  verify report:  {}", summary.verify_report_path);

    Ok(summary)
}

fn run_paper_export(bundles_dir: &Path, paper_dir: &Path) -> Result<()> {
    fs::create_dir_all(paper_dir)
        .with_context(|| format!("creating paper dir {}", paper_dir.display()))?;
    let input = load_bundle_dir(bundles_dir, &LoadOptions::default())
        .with_context(|| format!("loading bundles from {}", bundles_dir.display()))?;
    write_paper_exports(&input, paper_dir)
        .with_context(|| format!("writing paper exports to {}", paper_dir.display()))?;
    Ok(())
}

fn write_config(inputs_dir: &Path) -> Result<PathBuf> {
    let path = inputs_dir.join("evaluator.toml");
    // The batch path only loads `name` and `schema_version` from this
    // file via `EvaluatorConfig::from_path` (see tests in batch.rs).
    // Everything else is threaded through `BatchArgs`.
    fs::write(&path, "name = \"milestone-k-demo\"\nschema_version = 1\n")
        .with_context(|| format!("writing config {}", path.display()))?;
    Ok(path)
}

fn seed_inputs(inputs_dir: &Path, tasks: u32) -> Result<PathBuf> {
    fs::create_dir_all(inputs_dir)
        .with_context(|| format!("creating inputs dir {}", inputs_dir.display()))?;
    let mut panel_lines = Vec::with_capacity(tasks as usize);
    for i in 0..tasks {
        let tag = format!("demo-{i:02}");
        let entry_dir = inputs_dir.join(&tag);
        fs::create_dir_all(&entry_dir)?;
        let task = synth_task(&tag);
        let candidate = synth_candidate(&task, &tag);

        let task_path = entry_dir.join("task.json");
        let candidate_path = entry_dir.join("candidate.json");
        let patch_path = entry_dir.join("patch.diff");
        let workspace = entry_dir.join("workspace");
        fs::create_dir_all(&workspace)?;
        fs::write(
            workspace.join("README.md"),
            format!("# demo workspace {tag}\n\nSynthetic reproducibility slice.\n"),
        )?;
        fs::write(&task_path, serde_json::to_vec_pretty(&task)?)?;
        fs::write(&candidate_path, serde_json::to_vec_pretty(&candidate)?)?;
        fs::write(&patch_path, b"")?;

        let entry = PanelEntry {
            task: task_path,
            candidate: candidate_path,
            patch: patch_path,
            workspace_template: workspace,
            bundle_name: Some(format!("bundle-{tag}")),
            entry_id: Some(format!("entry-{tag}")),
        };
        panel_lines.push(serde_json::to_string(&entry)?);
    }
    let panel_path = inputs_dir.join("panel.jsonl");
    let mut buf = panel_lines.join("\n");
    buf.push('\n');
    fs::write(&panel_path, buf)
        .with_context(|| format!("writing panel {}", panel_path.display()))?;
    Ok(panel_path)
}

fn synth_task(tag: &str) -> BenchmarkTask {
    BenchmarkTask::new(
        BenchmarkId::SweBenchVerified,
        TaskId::new(format!("demo__milestone-k-{tag}")).unwrap(),
        "demo/milestone-k",
        "1",
        format!("Milestone K demo {tag}"),
        "Synthetic demo task; no upstream benchmark data is involved.",
        "deadbeefcafe0000000000000000000000000000",
        "local:demo",
        "cargo --version",
        BenchmarkLanguage::Rust,
        "https://example.test/demo/milestone-k",
        demo_timestamp(),
    )
}

fn synth_candidate(task: &BenchmarkTask, tag: &str) -> CandidateResolution {
    let uid = Uuid::new_v5(&DEMO_NAMESPACE, format!("demo-candidate-{tag}").as_bytes());
    let mut c = CandidateResolution::new(
        task.benchmark_id,
        task.task_id.clone(),
        "demo-harness",
        "demo-model",
        GenerationMode::SingleShot,
        PatchFormat::UnifiedDiff,
        "demo://noop",
        GenerationMetadata {
            temperature: Some(0.0),
            tool_configuration: serde_json::Value::Object(serde_json::Map::new()),
            context_mode: ContextMode::FileLevel,
            repo_reproduction_used: false,
            random_seed: Some(0),
        },
        demo_timestamp(),
    );
    c.candidate_id = CandidateId::from(uid);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn normalize_verify_report(bytes: &[u8]) -> Vec<u8> {
        let mut v: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        if let Some(obj) = v.as_object_mut() {
            obj.insert(
                "run_dir".into(),
                serde_json::Value::String("<run_dir>".into()),
            );
        }
        if let Some(entries) = v.get_mut("entries").and_then(|v| v.as_array_mut()) {
            for entry in entries {
                if let Some(obj) = entry.as_object_mut() {
                    obj.insert(
                        "bundle_dir".into(),
                        serde_json::Value::String("<bundle_dir>".into()),
                    );
                }
            }
        }
        eval_ladder_core::canonical_json(&v).unwrap()
    }

    #[test]
    fn milestone_k_demo_runs_end_to_end() {
        let dir = tempdir().unwrap();
        let summary = run_demo(DemoRunArgs {
            out: dir.path().to_path_buf(),
            tasks: 2,
            skip_analyze: false,
        })
        .expect("demo must run cleanly");

        assert_eq!(summary.tasks, 2);
        assert!(Path::new(&summary.panel_path).is_file(), "panel must exist");
        assert!(
            Path::new(&summary.bundles_dir).is_dir(),
            "bundles dir must exist"
        );
        assert!(
            Path::new(&summary.verify_report_path).is_file(),
            "verify report must exist"
        );
        assert!(!summary.paper_dir.is_empty(), "paper dir must be populated");
        let paper_manifest = Path::new(&summary.paper_dir).join("manifest.json");
        assert!(
            paper_manifest.is_file(),
            "paper-export manifest must exist at {}",
            paper_manifest.display()
        );

        // The shipped verify report must report every bundle ok.
        let report_bytes = fs::read(&summary.verify_report_path).unwrap();
        let report: serde_json::Value = serde_json::from_slice(&report_bytes).unwrap();
        assert_eq!(report["ok"].as_u64().unwrap(), 2);
        assert_eq!(report["invalid"].as_u64().unwrap(), 0);
    }

    #[test]
    fn milestone_k_demo_is_byte_deterministic_across_runs() {
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();
        let summary_a = run_demo(DemoRunArgs {
            out: dir_a.path().to_path_buf(),
            tasks: 2,
            skip_analyze: true,
        })
        .unwrap();
        let summary_b = run_demo(DemoRunArgs {
            out: dir_b.path().to_path_buf(),
            tasks: 2,
            skip_analyze: true,
        })
        .unwrap();

        let report_a = fs::read(&summary_a.verify_report_path).unwrap();
        let report_b = fs::read(&summary_b.verify_report_path).unwrap();
        assert_eq!(
            normalize_verify_report(&report_a),
            normalize_verify_report(&report_b),
            "demo verify report content must be byte-deterministic across runs"
        );

        // Bundle hashes are content-addressed and must be strictly
        // equal across runs regardless of tempdir differences.
        let ra: serde_json::Value = serde_json::from_slice(&report_a).unwrap();
        let rb: serde_json::Value = serde_json::from_slice(&report_b).unwrap();
        let entries_a = ra["entries"].as_array().unwrap();
        let entries_b = rb["entries"].as_array().unwrap();
        assert_eq!(entries_a.len(), entries_b.len());
        for (ea, eb) in entries_a.iter().zip(entries_b.iter()) {
            assert_eq!(ea["bundle_name"], eb["bundle_name"]);
            assert_eq!(ea["bundle_hash"], eb["bundle_hash"]);
            assert_eq!(ea["status"], eb["status"]);
        }
    }
}
