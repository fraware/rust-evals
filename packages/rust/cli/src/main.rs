//! `eval-ladder` CLI entrypoint.
//!
//! The CLI is the only crate allowed to wire the evaluator crates together.
//! Each subcommand delegates into a module under `cli::commands::*`.
//!
//! Binary crates have no external consumers, so the `unreachable_pub` lint
//! inherited from the workspace is explicitly silenced here. Visibility is
//! still controlled carefully via `pub(crate)` where appropriate inside the
//! crate.

#![deny(missing_docs)]
#![allow(unreachable_pub)]

mod commands;
mod config;
mod logging;

use anyhow::Result;
use clap::Parser;

use commands::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    logging::init(cli.verbose)?;

    match cli.command {
        Command::Ingest(cmd) => commands::ingest::run(cmd),
        Command::Evaluate(cmd) => commands::evaluate::run(cmd),
        Command::ProveSubset(cmd) => commands::prove_subset::run(cmd),
        Command::Analyze(cmd) => commands::analyze::run(cmd),
        Command::Schema(cmd) => commands::schema::run(cmd),
        Command::Verify(cmd) => commands::verify::run(cmd),
        Command::Demo(cmd) => commands::demo::run(cmd),
        Command::Version => {
            println!(
                "eval-ladder {} (schema_version={})",
                eval_ladder_core::EVALUATOR_VERSION,
                eval_ladder_core::SCHEMA_VERSION
            );
            Ok(())
        }
    }
}
