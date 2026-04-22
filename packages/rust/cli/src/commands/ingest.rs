//! `eval-ladder ingest <benchmark> --manifest <profile.toml> --source <path>`
//!
//! Reads a locally mirrored SWE-bench-family JSONL source (file or directory
//! of `*.jsonl` files), dispatches to the matching adapter, writes normalized
//! `BenchmarkTask` manifests into the profile's `manifest_dir`, and prints
//! a JSON ingest report to stdout.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use eval_ladder_benchmarks::{adapter_for, IngestOptions};
use eval_ladder_core::BenchmarkId;
use tracing::info;

use crate::config::EvaluatorConfig;

/// Benchmark selector for ingest.
#[derive(Debug, Subcommand)]
pub enum IngestCmd {
    /// Ingest SWE-Bench Verified tasks.
    Verified(IngestArgs),
    /// Ingest SWE-bench-Live tasks.
    Live(IngestArgs),
    /// Ingest Rust-SWE-bench tasks.
    Rust(IngestArgs),
}

/// Shared ingest arguments.
#[derive(Debug, Args)]
pub struct IngestArgs {
    /// Path to the benchmark evaluator configuration TOML.
    ///
    /// The configuration's `manifest_dir` is used as the output directory
    /// for normalized `BenchmarkTask` manifests.
    #[arg(long)]
    pub manifest: PathBuf,

    /// Path to the raw JSONL source (a single `.jsonl` file or a directory
    /// containing `*.jsonl` files).
    #[arg(long)]
    pub source: PathBuf,

    /// Optional override for the manifest output directory. When absent the
    /// evaluator profile's `manifest_dir` is used.
    #[arg(long)]
    pub out_dir: Option<PathBuf>,

    /// Optional task id filter (may be repeated). When empty, every task
    /// in the source is ingested.
    #[arg(long = "only")]
    pub only_task_ids: Vec<String>,

    /// Maximum tasks to ingest (useful for smoke tests).
    #[arg(long)]
    pub limit: Option<u32>,
}

/// Entrypoint.
pub fn run(cmd: IngestCmd) -> Result<()> {
    let (benchmark_id, args) = match cmd {
        IngestCmd::Verified(a) => (BenchmarkId::SweBenchVerified, a),
        IngestCmd::Live(a) => (BenchmarkId::SweBenchLive, a),
        IngestCmd::Rust(a) => (BenchmarkId::RustSweBench, a),
    };

    let config = EvaluatorConfig::from_path(&args.manifest)
        .with_context(|| format!("loading evaluator config from {}", args.manifest.display()))?;
    info!(profile = %config.name, benchmark = %benchmark_id, "ingesting benchmark");

    if !args.source.exists() {
        bail!("source path does not exist: {}", args.source.display());
    }

    let out_dir = args
        .out_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from(&config.manifest_dir));

    let adapter = adapter_for(benchmark_id);
    let opts = IngestOptions {
        only_task_ids: args.only_task_ids,
        output_dir: Some(out_dir),
        limit: args.limit,
    };
    let report = adapter
        .ingest(&args.source, &opts)
        .with_context(|| format!("ingest {benchmark_id}"))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
