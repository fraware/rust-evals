//! `eval-ladder analyze` subcommands.
//!
//! All subcommands accept a `--run-dir` pointing at either:
//!
//! - a directory of per-candidate evidence bundles (preferred, Milestone G),
//!   or
//! - a directory containing a pre-built `analysis_input.json` (escape hatch
//!   for callers that assemble the view themselves).
//!
//! When both exist, the bundle directory is authoritative. This keeps the
//! "evidence bundles are the only cross-crate contract" invariant from
//! `docs/architecture.md` intact.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use eval_ladder_analysis::{
    csv, load_bundle_dir, paper_export::write_paper_exports, rank_stability::rank_stability,
    score_descent, static_vs_live::static_vs_live, taxonomy::taxonomy_counts, AnalysisInput,
    AnalysisMode, LoadOptions,
};

/// `analyze` subcommands.
#[derive(Debug, Subcommand)]
pub enum AnalyzeCmd {
    /// Pass rate by level, stratified by benchmark and agent.
    ScoreDescent(AnalyzeArgs),
    /// Conditional false-success rate `P(fail L_{k+1} | pass L_k)`.
    ConditionalFalseSuccess(AnalyzeArgs),
    /// Kendall tau-b between agent leaderboards at every pair of levels.
    RankStability(AnalyzeArgs),
    /// Aggregated false-success taxonomy.
    Taxonomy(AnalyzeArgs),
    /// Per-agent, per-level comparison of static (SWE-bench Verified)
    /// vs live (SWE-bench-Live) pass rates (Milestone L).
    StaticVsLive(AnalyzeArgs),
    /// Emit every paper-ready table into a single directory (Milestone G).
    PaperExport(PaperExportArgs),
}

/// Shared analyze arguments.
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    /// Directory containing per-candidate evidence bundles **or** a
    /// pre-built `analysis_input.json`.
    #[arg(long)]
    pub run_dir: PathBuf,
    /// Optional output path. If omitted the table is written to stdout as CSV.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Optional canonical-JSON sibling file to emit alongside the CSV
    /// (for provenance and deterministic hashing).
    #[arg(long)]
    pub json_out: Option<PathBuf>,
    /// Analysis semantics used to build derived paper tables.
    ///
    /// `raw`: preserve level independence exactly as emitted by the evaluator.
    /// `cumulative`: require lower-level pass preconditions for upper-level
    /// pass when computing derived tables.
    #[arg(long, value_enum, default_value_t = CliAnalysisMode::Raw)]
    pub analysis_mode: CliAnalysisMode,
}

/// Arguments for `analyze paper-export`.
#[derive(Debug, Args)]
pub struct PaperExportArgs {
    /// Directory containing per-candidate evidence bundles **or** a
    /// pre-built `analysis_input.json`.
    #[arg(long)]
    pub run_dir: PathBuf,
    /// Directory in which the paper tables will be written. Created if
    /// missing.
    #[arg(long)]
    pub out_dir: PathBuf,
    /// Analysis semantics used for paper-export tables.
    ///
    /// Defaults to `cumulative` for headline reporting; use `raw` for
    /// appendix/debug exports.
    #[arg(long, value_enum, default_value_t = CliAnalysisMode::Cumulative)]
    pub analysis_mode: CliAnalysisMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliAnalysisMode {
    Raw,
    Cumulative,
}

impl From<CliAnalysisMode> for AnalysisMode {
    fn from(value: CliAnalysisMode) -> Self {
        match value {
            CliAnalysisMode::Raw => AnalysisMode::Raw,
            CliAnalysisMode::Cumulative => AnalysisMode::Cumulative,
        }
    }
}

/// Dispatch.
pub fn run(cmd: AnalyzeCmd) -> Result<()> {
    match cmd {
        AnalyzeCmd::ScoreDescent(args) => run_score_descent(args),
        AnalyzeCmd::ConditionalFalseSuccess(args) => run_conditional_false_success(args),
        AnalyzeCmd::RankStability(args) => run_rank_stability(args),
        AnalyzeCmd::Taxonomy(args) => run_taxonomy(args),
        AnalyzeCmd::StaticVsLive(args) => run_static_vs_live(args),
        AnalyzeCmd::PaperExport(args) => run_paper_export(args),
    }
}

/// Load the [`AnalysisInput`] for a run directory.
///
/// If `analysis_input.json` is present it is used verbatim; otherwise the
/// directory is assumed to contain per-candidate evidence bundles and is
/// walked via [`load_bundle_dir`].
fn load_input(run_dir: &Path) -> Result<AnalysisInput> {
    if !run_dir.exists() {
        anyhow::bail!("{}: run dir does not exist", run_dir.display());
    }
    let explicit = run_dir.join("analysis_input.json");
    if explicit.exists() {
        let bytes = fs::read(&explicit)?;
        let input: AnalysisInput = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", explicit.display()))?;
        return Ok(input);
    }
    load_bundle_dir(run_dir, &LoadOptions::default())
        .with_context(|| format!("loading bundles under {}", run_dir.display()))
}

fn write_output(
    args: &AnalyzeArgs,
    header: &[&str],
    rows_csv: &[Vec<String>],
    json_value: &serde_json::Value,
) -> Result<()> {
    if let Some(path) = &args.out {
        let mut w = fs::File::create(path)?;
        csv::write_table(&mut w, header, rows_csv, |v| v.clone())?;
    } else {
        let stdout = std::io::stdout();
        let mut w = stdout.lock();
        csv::write_table(&mut w, header, rows_csv, |v| v.clone())?;
    }
    if let Some(path) = &args.json_out {
        let bytes = eval_ladder_core::canonical_json(json_value)?;
        fs::write(path, bytes)?;
    }
    Ok(())
}

fn run_score_descent(args: AnalyzeArgs) -> Result<()> {
    let input = load_input(&args.run_dir)?;
    let mode: AnalysisMode = args.analysis_mode.into();
    let table = score_descent::score_descent(&input, mode);
    let header = &[
        "benchmark_id",
        "agent_id",
        "level",
        "passed",
        "evaluated",
        "pass_rate",
    ];
    let rows: Vec<Vec<String>> = table
        .iter()
        .map(|r| {
            vec![
                r.stratum
                    .benchmark_id
                    .map(|b| b.as_str().to_owned())
                    .unwrap_or_default(),
                r.stratum.agent_id.clone().unwrap_or_default(),
                r.level.short_code().to_owned(),
                r.passed.to_string(),
                r.evaluated.to_string(),
                r.pass_rate.map_or_else(String::new, |v| format!("{v:.6}")),
            ]
        })
        .collect();
    write_output(&args, header, &rows, &serde_json::to_value(&table)?)
}

fn run_conditional_false_success(args: AnalyzeArgs) -> Result<()> {
    let input = load_input(&args.run_dir)?;
    let mode: AnalysisMode = args.analysis_mode.into();
    let table = score_descent::conditional_false_success_with_mode(&input, mode);
    let header = &[
        "level_from",
        "level_to",
        "n_passed_from",
        "n_failed_to",
        "rate",
    ];
    let rows: Vec<Vec<String>> = table
        .iter()
        .map(|r| {
            vec![
                r.level_from.short_code().to_owned(),
                r.level_to.short_code().to_owned(),
                r.n_passed_from.to_string(),
                r.n_failed_to.to_string(),
                r.rate.map_or_else(String::new, |v| format!("{v:.6}")),
            ]
        })
        .collect();
    write_output(&args, header, &rows, &serde_json::to_value(&table)?)
}

fn run_rank_stability(args: AnalyzeArgs) -> Result<()> {
    let input = load_input(&args.run_dir)?;
    let mode: AnalysisMode = args.analysis_mode.into();
    let table = rank_stability(&input, mode);
    let header = &["level_a", "level_b", "n_agents", "kendall_tau_b"];
    let rows: Vec<Vec<String>> = table
        .iter()
        .map(|r| {
            vec![
                r.level_a.short_code().to_owned(),
                r.level_b.short_code().to_owned(),
                r.n_agents.to_string(),
                r.kendall_tau_b
                    .map_or_else(String::new, |v| format!("{v:.6}")),
            ]
        })
        .collect();
    write_output(&args, header, &rows, &serde_json::to_value(&table)?)
}

fn run_taxonomy(args: AnalyzeArgs) -> Result<()> {
    let input = load_input(&args.run_dir)?;
    let table = taxonomy_counts(&input);
    let header = &["benchmark_id", "level", "primary_reason", "count"];
    let rows: Vec<Vec<String>> = table
        .iter()
        .map(|r| {
            vec![
                r.benchmark_id.as_str().to_owned(),
                r.level.short_code().to_owned(),
                r.primary_reason.clone(),
                r.count.to_string(),
            ]
        })
        .collect();
    write_output(&args, header, &rows, &serde_json::to_value(&table)?)
}

fn run_static_vs_live(args: AnalyzeArgs) -> Result<()> {
    let input = load_input(&args.run_dir)?;
    let mode: AnalysisMode = args.analysis_mode.into();
    let table = static_vs_live(&input, mode);
    let header = &[
        "agent_id",
        "level",
        "static_passed",
        "static_evaluated",
        "static_pass_rate",
        "live_passed",
        "live_evaluated",
        "live_pass_rate",
        "delta",
        "ratio",
    ];
    let rows: Vec<Vec<String>> = table
        .iter()
        .map(|r| {
            vec![
                r.agent_id.clone(),
                r.level.short_code().to_owned(),
                r.static_passed.to_string(),
                r.static_evaluated.to_string(),
                r.static_pass_rate
                    .map_or_else(String::new, |v| format!("{v:.6}")),
                r.live_passed.to_string(),
                r.live_evaluated.to_string(),
                r.live_pass_rate
                    .map_or_else(String::new, |v| format!("{v:.6}")),
                r.delta.map_or_else(String::new, |v| format!("{v:.6}")),
                r.ratio.map_or_else(String::new, |v| format!("{v:.6}")),
            ]
        })
        .collect();
    write_output(&args, header, &rows, &serde_json::to_value(&table)?)
}

fn run_paper_export(args: PaperExportArgs) -> Result<()> {
    let input = load_input(&args.run_dir)?;
    let mode: AnalysisMode = args.analysis_mode.into();
    let manifest = write_paper_exports(&input, &args.out_dir, mode)
        .with_context(|| format!("writing paper exports into {}", args.out_dir.display()))?;
    let canonical = eval_ladder_core::canonical_json(&manifest)?;
    let stdout = std::io::stdout();
    let mut w = stdout.lock();
    std::io::Write::write_all(&mut w, &canonical)?;
    std::io::Write::write_all(&mut w, b"\n")?;
    Ok(())
}
