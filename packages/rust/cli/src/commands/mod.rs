//! Subcommand dispatch.

use clap::{Parser, Subcommand};

pub mod analyze;
pub mod batch;
pub mod demo;
pub mod evaluate;
pub mod ingest;
pub mod prove_subset;
pub mod schema;
pub mod verify;

/// Top-level CLI.
#[derive(Debug, Parser)]
#[command(
    name = "eval-ladder",
    version,
    about = "A Rust-first scientific evaluation monorepo for auditing coding-agent benchmarks.",
    long_about = None,
    propagate_version = true,
)]
pub struct Cli {
    /// Increase log verbosity (repeat for more: -v = debug, -vv = trace).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Subcommand.
    #[command(subcommand)]
    pub command: Command,
}

/// All subcommands.
#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)] // clap subcommand args structs; boxing breaks derive
pub enum Command {
    /// Ingest benchmark tasks into normalized manifests.
    #[command(subcommand)]
    Ingest(ingest::IngestCmd),
    /// Evaluate candidate patches along the ladder.
    #[command(subcommand)]
    Evaluate(evaluate::EvaluateCmd),
    /// Run the L4 Lean checker over the curated proof subset.
    ProveSubset(prove_subset::ProveSubsetArgs),
    /// Produce paper-ready analysis outputs from evidence bundles.
    #[command(subcommand)]
    Analyze(analyze::AnalyzeCmd),
    /// Tools for the shipped JSON schemas.
    #[command(subcommand)]
    Schema(schema::SchemaCmd),
    /// Verify evidence bundles and trace hash chains (Milestone J).
    #[command(subcommand)]
    Verify(verify::VerifyCmd),
    /// Run the self-contained reproducibility demo (Milestone K).
    #[command(subcommand)]
    Demo(demo::DemoCmd),
    /// Print detailed version information and exit.
    Version,
}
