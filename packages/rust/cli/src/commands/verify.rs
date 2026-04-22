//! `eval-ladder verify` - auditor-grade bundle and trace verification.
//!
//! Milestone J.
//!
//! The verify command is the single reviewer-facing entry point for
//! answering the question "are these artifacts the ones the evaluator
//! actually produced?". It recomputes every persisted digest and
//! reports the result as a machine-readable, byte-deterministic JSON
//! artifact. Three modes:
//!
//! - [`VerifyCmd::Bundle`] - verify one evidence bundle directory.
//! - [`VerifyCmd::Trace`] - verify the hash chain of a single
//!   `trace.jsonl` file.
//! - [`VerifyCmd::RunDir`] - verify every bundle under a run
//!   directory (the shape produced by `evaluate batch`), write a
//!   `verify_report.json` next to the run, and exit non-zero iff any
//!   bundle is bad.
//!
//! The report schema (`VerifyReport`) is stable across runs. Error
//! codes (`VERIFY_*`) are stable so downstream scripts can diff them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use eval_ladder_core::{canonical_json, Sha256Digest};
use eval_ladder_evidence::{verify_bundle, BundleVerifyError};
use eval_ladder_traces::{TraceReader, TraceReaderError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

/// Verify subcommands.
#[derive(Debug, Subcommand)]
pub enum VerifyCmd {
    /// Verify a single evidence bundle directory.
    Bundle(VerifyBundleArgs),
    /// Verify a single `trace.jsonl` hash chain.
    Trace(VerifyTraceArgs),
    /// Verify every bundle directory under a run directory.
    RunDir(VerifyRunDirArgs),
}

/// Arguments for `verify bundle`.
#[derive(Debug, Args)]
pub struct VerifyBundleArgs {
    /// Path to the bundle directory (must contain `artifact_hashes.json`).
    #[arg(long)]
    pub bundle_dir: PathBuf,
    /// Also verify the sibling `trace.jsonl` hash chain (default: on).
    #[arg(long, default_value_t = true)]
    pub verify_trace: bool,
}

/// Arguments for `verify trace`.
#[derive(Debug, Args)]
pub struct VerifyTraceArgs {
    /// Path to the `trace.jsonl` file.
    #[arg(long)]
    pub trace: PathBuf,
}

/// Arguments for `verify run-dir`.
#[derive(Debug, Args)]
pub struct VerifyRunDirArgs {
    /// Path to a run directory (the `--out` of `evaluate batch`).
    #[arg(long)]
    pub run_dir: PathBuf,

    /// Optional override for the report output path. Defaults to
    /// `<run-dir>/verify_report.json`.
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Fail on the first bad bundle instead of verifying every entry.
    /// Defaults to off so a broken bundle does not prevent enumeration
    /// of the rest.
    #[arg(long, default_value_t = false)]
    pub fail_fast: bool,
}

/// Stable error codes produced by the verify pipeline.
///
/// These strings are part of the public artifact contract; downstream
/// diff scripts match on them.
pub mod error_codes {
    pub const FILE_DIGEST_MISMATCH: &str = "VERIFY_FILE_DIGEST_MISMATCH";
    pub const BUNDLE_DIGEST_MISMATCH: &str = "VERIFY_BUNDLE_DIGEST_MISMATCH";
    pub const MISSING_FILE: &str = "VERIFY_MISSING_FILE";
    pub const BUNDLE_PARSE: &str = "VERIFY_BUNDLE_PARSE";
    pub const BUNDLE_IO: &str = "VERIFY_BUNDLE_IO";
    pub const BUNDLE_CORE: &str = "VERIFY_BUNDLE_CORE";
    pub const TRACE_IO: &str = "VERIFY_TRACE_IO";
    pub const TRACE_PARSE: &str = "VERIFY_TRACE_PARSE";
    pub const TRACE_HASH_MISMATCH: &str = "VERIFY_TRACE_HASH_MISMATCH";
    pub const TRACE_CHAIN_BROKEN: &str = "VERIFY_TRACE_CHAIN_BROKEN";
    pub const TRACE_FIRST_NOT_RUN_STARTED: &str = "VERIFY_TRACE_FIRST_NOT_RUN_STARTED";
    pub const TRACE_DUPLICATE_RUN_STARTED: &str = "VERIFY_TRACE_DUPLICATE_RUN_STARTED";
    pub const TRACE_CORE: &str = "VERIFY_TRACE_CORE";
    pub const TRACE_MISSING: &str = "VERIFY_TRACE_MISSING";
    pub const REPORT_WRITE: &str = "VERIFY_REPORT_WRITE";
}

/// Verification status for a single artifact group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifyStatus {
    /// All recomputed digests matched.
    Ok,
    /// At least one digest did not match, or artifact was missing.
    Invalid,
    /// The artifact was not present and not required (e.g. a trace
    /// verification skipped by CLI flag).
    NotApplicable,
}

/// Per-bundle row in the verify report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)] // contains Option<Sha256Digest>
pub struct VerifyEntryRow {
    /// Stable identifier for the bundle (its directory name).
    pub bundle_name: String,
    /// Absolute path that was verified.
    pub bundle_dir: String,
    /// Overall status: `ok` iff both `bundle` and `trace` are ok or not applicable.
    pub status: VerifyStatus,
    /// Bundle-level digest if the bundle loaded successfully, else `None`.
    pub bundle_hash: Option<Sha256Digest>,
    /// Digest check for files + bundle hash.
    pub bundle: VerifyStatus,
    /// Hash-chain check for `trace.jsonl`.
    pub trace: VerifyStatus,
    /// Optional stable error code (one of `error_codes::*`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Optional human-readable error message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Report artifact written by `verify run-dir` (and useful in-memory
/// for unit tests of the other subcommands).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct VerifyReport {
    /// Schema version for the report itself.
    pub schema_version: u32,
    /// Evaluator version string that produced the report.
    pub evaluator_version: String,
    /// Root that was verified.
    pub run_dir: String,
    /// Total bundles considered.
    pub total: u32,
    /// Bundles that passed every applicable check.
    pub ok: u32,
    /// Bundles that failed at least one applicable check.
    pub invalid: u32,
    /// Per-entry rows, sorted by `bundle_name` for stable diffs.
    pub entries: Vec<VerifyEntryRow>,
}

/// Errors specific to the verify pipeline that the CLI layer surfaces
/// rather than catching into a row (panel- / run-level failures).
#[derive(Debug, Error)]
pub enum VerifyError {
    /// Input path did not exist.
    #[error("verify: input path does not exist: {0}")]
    MissingInput(PathBuf),
    /// Run directory contained no bundle directories.
    #[error("verify: no bundle directories found under {0}")]
    EmptyRunDir(PathBuf),
    /// IO error reading the run directory.
    #[error("verify: run-dir io: {0}")]
    Io(#[from] std::io::Error),
    /// Error encoding the report.
    #[error("verify: encoding report: {0}")]
    Encode(#[from] eval_ladder_core::CoreError),
}

const VERIFY_REPORT_SCHEMA_VERSION: u32 = 1;

/// Entrypoint.
pub fn run(cmd: VerifyCmd) -> Result<()> {
    match cmd {
        VerifyCmd::Bundle(args) => run_bundle(args),
        VerifyCmd::Trace(args) => run_trace(args),
        VerifyCmd::RunDir(args) => run_run_dir(args),
    }
}

/// Public helper used by tests and library callers: verify a single
/// bundle directory and return a `VerifyEntryRow`. Never panics; all
/// recoverable failures become `VerifyStatus::Invalid` with a stable
/// error code.
pub fn verify_single_bundle(bundle_dir: &Path, verify_trace_chain: bool) -> VerifyEntryRow {
    let bundle_name = bundle_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("<unnamed>")
        .to_owned();
    let dir_display = bundle_dir.display().to_string();

    if !bundle_dir.is_dir() {
        return VerifyEntryRow {
            bundle_name,
            bundle_dir: dir_display,
            status: VerifyStatus::Invalid,
            bundle_hash: None,
            bundle: VerifyStatus::Invalid,
            trace: VerifyStatus::NotApplicable,
            error_code: Some(error_codes::BUNDLE_IO.to_owned()),
            error: Some(format!("not a directory: {}", bundle_dir.display())),
        };
    }

    let (bundle_status, bundle_hash, bundle_err) = match verify_bundle(bundle_dir) {
        Ok(index) => (VerifyStatus::Ok, Some(index.bundle_hash), None),
        Err(e) => {
            let (code, msg) = classify_bundle_error(&e);
            (VerifyStatus::Invalid, None, Some((code, msg)))
        }
    };

    let (trace_status, trace_err) = if verify_trace_chain {
        let trace_path = bundle_dir.join("trace.jsonl");
        if trace_path.is_file() {
            match TraceReader::read_and_verify(&trace_path) {
                Ok(_) => (VerifyStatus::Ok, None),
                Err(e) => {
                    let (code, msg) = classify_trace_error(&e);
                    (VerifyStatus::Invalid, Some((code, msg)))
                }
            }
        } else {
            (
                VerifyStatus::Invalid,
                Some((
                    error_codes::TRACE_MISSING,
                    "trace.jsonl not found".to_owned(),
                )),
            )
        }
    } else {
        (VerifyStatus::NotApplicable, None)
    };

    // Bundle-level failures take precedence over trace-level
    // failures in the surfaced `error_code` because a mismatched
    // file digest is strictly more fundamental than any chain
    // anomaly that could follow from it.
    let (error_code, error) = match (bundle_err, trace_err) {
        (Some((c, m)), _) | (None, Some((c, m))) => (Some(c.to_owned()), Some(m)),
        (None, None) => (None, None),
    };

    let overall = combine_status(bundle_status, trace_status);

    VerifyEntryRow {
        bundle_name,
        bundle_dir: dir_display,
        status: overall,
        bundle_hash,
        bundle: bundle_status,
        trace: trace_status,
        error_code,
        error,
    }
}

fn combine_status(bundle: VerifyStatus, trace: VerifyStatus) -> VerifyStatus {
    if bundle == VerifyStatus::Invalid || trace == VerifyStatus::Invalid {
        VerifyStatus::Invalid
    } else {
        VerifyStatus::Ok
    }
}

fn classify_bundle_error(e: &BundleVerifyError) -> (&'static str, String) {
    match e {
        BundleVerifyError::Io(_) => (error_codes::BUNDLE_IO, e.to_string()),
        BundleVerifyError::Parse(_) => (error_codes::BUNDLE_PARSE, e.to_string()),
        BundleVerifyError::Core(_) => (error_codes::BUNDLE_CORE, e.to_string()),
        BundleVerifyError::FileDigestMismatch { .. } => {
            (error_codes::FILE_DIGEST_MISMATCH, e.to_string())
        }
        BundleVerifyError::BundleDigestMismatch { .. } => {
            (error_codes::BUNDLE_DIGEST_MISMATCH, e.to_string())
        }
        BundleVerifyError::MissingFile(_) => (error_codes::MISSING_FILE, e.to_string()),
    }
}

fn classify_trace_error(e: &TraceReaderError) -> (&'static str, String) {
    match e {
        TraceReaderError::Io(_) => (error_codes::TRACE_IO, e.to_string()),
        TraceReaderError::Parse { .. } => (error_codes::TRACE_PARSE, e.to_string()),
        TraceReaderError::Core(_) => (error_codes::TRACE_CORE, e.to_string()),
        TraceReaderError::HashMismatch { .. } => (error_codes::TRACE_HASH_MISMATCH, e.to_string()),
        TraceReaderError::ChainBroken { .. } => (error_codes::TRACE_CHAIN_BROKEN, e.to_string()),
        TraceReaderError::FirstEventMustBeRunStarted(_) => {
            (error_codes::TRACE_FIRST_NOT_RUN_STARTED, e.to_string())
        }
        TraceReaderError::DuplicateRunStarted(_) => {
            (error_codes::TRACE_DUPLICATE_RUN_STARTED, e.to_string())
        }
    }
}

fn run_bundle(args: VerifyBundleArgs) -> Result<()> {
    if !args.bundle_dir.exists() {
        bail!(VerifyError::MissingInput(args.bundle_dir.clone()));
    }
    info!(bundle_dir = %args.bundle_dir.display(), "verify bundle");
    let row = verify_single_bundle(&args.bundle_dir, args.verify_trace);
    let bytes = canonical_json(&row).context("encoding verify row")?;
    println!("{}", String::from_utf8_lossy(&bytes));
    if row.status == VerifyStatus::Invalid {
        bail!(
            "bundle {} failed verification: {} ({})",
            row.bundle_name,
            row.error_code.unwrap_or_else(|| "UNKNOWN".to_owned()),
            row.error.unwrap_or_default()
        );
    }
    Ok(())
}

fn run_trace(args: VerifyTraceArgs) -> Result<()> {
    if !args.trace.exists() {
        bail!(VerifyError::MissingInput(args.trace.clone()));
    }
    info!(trace = %args.trace.display(), "verify trace");
    match TraceReader::read_and_verify(&args.trace) {
        Ok(events) => {
            #[derive(Serialize)]
            struct Ok<'a> {
                status: &'a str,
                events: usize,
                trace: String,
            }
            let row = Ok {
                status: "ok",
                events: events.len(),
                trace: args.trace.display().to_string(),
            };
            println!(
                "{}",
                String::from_utf8_lossy(&canonical_json(&row).context("encoding trace row")?)
            );
            Ok(())
        }
        Err(e) => {
            let (code, msg) = classify_trace_error(&e);
            #[derive(Serialize)]
            struct Bad<'a> {
                status: &'a str,
                error_code: &'a str,
                error: &'a str,
                trace: String,
            }
            let row = Bad {
                status: "invalid",
                error_code: code,
                error: &msg,
                trace: args.trace.display().to_string(),
            };
            eprintln!(
                "{}",
                String::from_utf8_lossy(&canonical_json(&row).context("encoding trace row")?)
            );
            bail!(
                "trace {} failed verification: {} ({})",
                args.trace.display(),
                code,
                msg
            );
        }
    }
}

/// Run the `verify run-dir` mode directly (without parsing CLI args).
///
/// Public so in-process callers (for example `eval-ladder demo run`)
/// can compose verification into their own flows without shelling
/// out. The exit semantics are the same as the CLI command.
pub fn run_run_dir(args: VerifyRunDirArgs) -> Result<()> {
    if !args.run_dir.exists() {
        bail!(VerifyError::MissingInput(args.run_dir.clone()));
    }
    if !args.run_dir.is_dir() {
        bail!("run-dir is not a directory: {}", args.run_dir.display());
    }
    info!(run_dir = %args.run_dir.display(), "verify run-dir");

    let bundles = discover_bundles(&args.run_dir).map_err(VerifyError::from)?;
    if bundles.is_empty() {
        bail!(VerifyError::EmptyRunDir(args.run_dir.clone()));
    }

    let mut entries = Vec::with_capacity(bundles.len());
    for (name, dir) in bundles {
        let row = verify_single_bundle(&dir, true);
        if args.fail_fast && row.status == VerifyStatus::Invalid {
            bail!(
                "fail-fast: bundle {} failed verification: {} ({})",
                name,
                row.error_code.unwrap_or_else(|| "UNKNOWN".to_owned()),
                row.error.unwrap_or_default()
            );
        }
        entries.push(row);
    }
    entries.sort_by(|a, b| a.bundle_name.cmp(&b.bundle_name));

    let ok = u32::try_from(
        entries
            .iter()
            .filter(|r| r.status == VerifyStatus::Ok)
            .count(),
    )
    .unwrap_or(u32::MAX);
    let invalid = u32::try_from(
        entries
            .iter()
            .filter(|r| r.status == VerifyStatus::Invalid)
            .count(),
    )
    .unwrap_or(u32::MAX);
    let total = ok.saturating_add(invalid);

    let report = VerifyReport {
        schema_version: VERIFY_REPORT_SCHEMA_VERSION,
        evaluator_version: eval_ladder_core::EVALUATOR_VERSION.to_string(),
        run_dir: args.run_dir.display().to_string(),
        total,
        ok,
        invalid,
        entries,
    };

    let out_path = args
        .out
        .unwrap_or_else(|| args.run_dir.join("verify_report.json"));
    let canonical = canonical_json(&report).context("encoding verify_report.json")?;
    std::fs::write(&out_path, canonical)
        .with_context(|| format!("{}: {}", error_codes::REPORT_WRITE, out_path.display()))?;

    println!(
        "verify: {} ok / {} invalid ({} total) -> {}",
        report.ok,
        report.invalid,
        report.total,
        out_path.display(),
    );

    if report.invalid > 0 {
        bail!(
            "{} bundle(s) failed verification; see {}",
            report.invalid,
            out_path.display()
        );
    }
    Ok(())
}

fn discover_bundles(run_dir: &Path) -> std::io::Result<Vec<(String, PathBuf)>> {
    // A bundle directory is any immediate child that contains
    // `artifact_hashes.json`. We pre-sort by directory name to keep
    // dependent walkers deterministic (the final report is re-sorted
    // on `bundle_name` regardless).
    let mut out: BTreeMap<String, PathBuf> = BTreeMap::new();
    for entry in std::fs::read_dir(run_dir)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if !ty.is_dir() {
            continue;
        }
        let path = entry.path();
        if !path.join("artifact_hashes.json").is_file() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            out.insert(name.to_owned(), path);
        }
    }
    Ok(out.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use eval_ladder_core::{BenchmarkId, BundleId, CandidateId, RunId, TaskId};
    use eval_ladder_evidence::{BundleBuilder, EvidenceBundleIndex, MANDATORY_BUNDLE_FILES};
    use eval_ladder_traces::{EventType, TraceWriter};
    use tempfile::tempdir;
    use uuid::Uuid;

    /// Deterministic namespace for bundle-test IDs. Random once,
    /// then pinned so every fixture derived from the same `tag`
    /// produces byte-identical artifacts across runs.
    const TEST_NAMESPACE: Uuid = Uuid::from_bytes([
        0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ]);

    fn stable_candidate_id(tag: &str) -> CandidateId {
        CandidateId::from(Uuid::new_v5(
            &TEST_NAMESPACE,
            format!("cand:{tag}").as_bytes(),
        ))
    }

    fn stable_run_id(tag: &str) -> RunId {
        RunId::from(Uuid::new_v5(
            &TEST_NAMESPACE,
            format!("run:{tag}").as_bytes(),
        ))
    }

    fn stable_bundle_id(tag: &str) -> BundleId {
        BundleId::from(Uuid::new_v5(
            &TEST_NAMESPACE,
            format!("bundle:{tag}").as_bytes(),
        ))
    }

    fn fixed_timestamp() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()
    }

    /// Write a fully valid, sealed 2-event trace.jsonl at `path`.
    ///
    /// The trace has one `RunStarted` and one `RunFinished` event with
    /// deterministic timestamps so hash-chain checks can't accidentally
    /// rely on clock skew.
    fn write_sealed_trace(path: &Path, run_id: RunId, candidate_id: CandidateId, task_id: TaskId) {
        let mut w = TraceWriter::create(path, run_id, candidate_id, task_id).unwrap();
        let t0 = chrono::Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let t1 = chrono::Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 1).unwrap();
        w.append_at(EventType::RunStarted, serde_json::json!({}), t0)
            .unwrap();
        w.append_at(EventType::RunFinished, serde_json::json!({}), t1)
            .unwrap();
    }

    /// Seed a minimal bundle with a valid, hash-chained trace.jsonl.
    /// Uses stable, tag-derived IDs so two calls with the same `tag`
    /// produce byte-identical bundle hashes.
    fn seed_full_bundle(root: &Path, tag: &str) -> EvidenceBundleIndex {
        let candidate_id = stable_candidate_id(tag);
        let task_id = TaskId::new(format!("t_{tag}")).unwrap();
        let run_id = stable_run_id(tag);
        for name in MANDATORY_BUNDLE_FILES {
            if *name == "artifact_hashes.json" || *name == "trace.jsonl" {
                continue;
            }
            std::fs::write(root.join(name), format!("stub:{name}:{tag}")).unwrap();
        }
        write_sealed_trace(
            &root.join("trace.jsonl"),
            run_id,
            candidate_id,
            task_id.clone(),
        );
        BundleBuilder::new(root, candidate_id, task_id, BenchmarkId::SweBenchVerified)
            .with_bundle_id(stable_bundle_id(tag))
            .finalize_at(fixed_timestamp())
            .unwrap()
    }

    /// Minimal stub bundle with a placeholder trace; ok for tests that
    /// ask `verify_trace=false`.
    fn seed_stub_bundle(root: &Path) -> EvidenceBundleIndex {
        for name in MANDATORY_BUNDLE_FILES {
            if *name == "artifact_hashes.json" {
                continue;
            }
            std::fs::write(root.join(name), format!("stub:{name}")).unwrap();
        }
        BundleBuilder::new(
            root,
            CandidateId::new_v4(),
            TaskId::new("t").unwrap(),
            BenchmarkId::SweBenchVerified,
        )
        .finalize()
        .unwrap()
    }

    #[test]
    fn single_bundle_passes_when_untampered() {
        let dir = tempdir().unwrap();
        seed_stub_bundle(dir.path());
        let row = verify_single_bundle(dir.path(), false);
        assert_eq!(row.status, VerifyStatus::Ok);
        assert_eq!(row.bundle, VerifyStatus::Ok);
        assert_eq!(row.trace, VerifyStatus::NotApplicable);
        assert!(row.bundle_hash.is_some());
    }

    #[test]
    fn single_bundle_detects_file_tamper() {
        let dir = tempdir().unwrap();
        seed_stub_bundle(dir.path());
        std::fs::write(dir.path().join("stdout.log"), "tampered").unwrap();
        let row = verify_single_bundle(dir.path(), false);
        assert_eq!(row.status, VerifyStatus::Invalid);
        assert_eq!(
            row.error_code.as_deref(),
            Some(error_codes::FILE_DIGEST_MISMATCH)
        );
    }

    #[test]
    fn non_directory_input_is_rejected_cleanly() {
        let dir = tempdir().unwrap();
        let fake = dir.path().join("not-a-dir");
        let row = verify_single_bundle(&fake, false);
        assert_eq!(row.status, VerifyStatus::Invalid);
        assert_eq!(row.error_code.as_deref(), Some(error_codes::BUNDLE_IO));
    }

    #[test]
    fn missing_trace_is_reported_when_verification_requested() {
        let dir = tempdir().unwrap();
        seed_stub_bundle(dir.path());
        // Removing trace.jsonl after sealing trips the bundle-level
        // MissingFile check first, which is the more specific failure.
        std::fs::remove_file(dir.path().join("trace.jsonl")).unwrap();
        let row = verify_single_bundle(dir.path(), true);
        assert_eq!(row.status, VerifyStatus::Invalid);
        assert_eq!(row.error_code.as_deref(), Some(error_codes::MISSING_FILE));
    }

    // ------------------------------------------------------------------
    // Milestone J acceptance: bundle + trace verification end-to-end.
    // ------------------------------------------------------------------

    fn seed_run_dir(tags: &[&str]) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        for tag in tags {
            let bundle_dir = dir.path().join(format!("bundle-{tag}"));
            std::fs::create_dir_all(&bundle_dir).unwrap();
            seed_full_bundle(&bundle_dir, tag);
        }
        dir
    }

    #[test]
    fn milestone_j_run_dir_passes_all_ok_bundles() {
        let run = seed_run_dir(&["alpha", "beta", "gamma"]);
        let args = VerifyRunDirArgs {
            run_dir: run.path().to_path_buf(),
            out: None,
            fail_fast: false,
        };
        run_run_dir(args).expect("all-ok run-dir must verify successfully");
        let report_bytes = std::fs::read(run.path().join("verify_report.json")).unwrap();
        let report: VerifyReport = serde_json::from_slice(&report_bytes).unwrap();
        assert_eq!(report.total, 3);
        assert_eq!(report.ok, 3);
        assert_eq!(report.invalid, 0);
        assert_eq!(report.entries.len(), 3);
        let names: Vec<&str> = report
            .entries
            .iter()
            .map(|r| r.bundle_name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["bundle-alpha", "bundle-beta", "bundle-gamma"],
            "entries must be sorted by bundle_name"
        );
        for row in &report.entries {
            assert_eq!(row.status, VerifyStatus::Ok);
            assert_eq!(row.bundle, VerifyStatus::Ok);
            assert_eq!(row.trace, VerifyStatus::Ok);
            assert!(row.bundle_hash.is_some());
            assert!(row.error_code.is_none());
            assert!(row.error.is_none());
        }
    }

    #[test]
    fn milestone_j_run_dir_detects_single_tampered_bundle() {
        let run = seed_run_dir(&["clean", "tampered"]);
        // Tamper one bundle's stdout.log without updating the hash index.
        std::fs::write(run.path().join("bundle-tampered").join("stdout.log"), "bad").unwrap();

        let args = VerifyRunDirArgs {
            run_dir: run.path().to_path_buf(),
            out: None,
            fail_fast: false,
        };
        let err = run_run_dir(args).expect_err("tampered bundle must fail run-dir verify");
        let msg = format!("{err}");
        assert!(
            msg.contains("1 bundle(s) failed verification"),
            "unexpected: {msg}"
        );

        // The report must still be written.
        let report_bytes = std::fs::read(run.path().join("verify_report.json")).unwrap();
        let report: VerifyReport = serde_json::from_slice(&report_bytes).unwrap();
        assert_eq!(report.total, 2);
        assert_eq!(report.ok, 1);
        assert_eq!(report.invalid, 1);

        let clean_row = report
            .entries
            .iter()
            .find(|r| r.bundle_name == "bundle-clean")
            .unwrap();
        assert_eq!(clean_row.status, VerifyStatus::Ok);

        let bad_row = report
            .entries
            .iter()
            .find(|r| r.bundle_name == "bundle-tampered")
            .unwrap();
        assert_eq!(bad_row.status, VerifyStatus::Invalid);
        assert_eq!(bad_row.bundle, VerifyStatus::Invalid);
        assert_eq!(
            bad_row.error_code.as_deref(),
            Some(error_codes::FILE_DIGEST_MISMATCH)
        );
    }

    #[test]
    fn milestone_j_verify_report_is_deterministic() {
        let run_a = seed_run_dir(&["alpha", "beta"]);
        let run_b = seed_run_dir(&["alpha", "beta"]);

        run_run_dir(VerifyRunDirArgs {
            run_dir: run_a.path().to_path_buf(),
            out: None,
            fail_fast: false,
        })
        .unwrap();
        run_run_dir(VerifyRunDirArgs {
            run_dir: run_b.path().to_path_buf(),
            out: None,
            fail_fast: false,
        })
        .unwrap();

        let raw_a = std::fs::read(run_a.path().join("verify_report.json")).unwrap();
        let raw_b = std::fs::read(run_b.path().join("verify_report.json")).unwrap();

        // `run_dir` and `bundle_dir` strings embed the per-run tempdir
        // path, so we normalize them to a placeholder before comparing
        // the byte-for-byte content of the report.
        let norm_a = normalize_report_for_determinism(&raw_a);
        let norm_b = normalize_report_for_determinism(&raw_b);
        assert_eq!(
            norm_a, norm_b,
            "verify_report content must be deterministic"
        );

        // The bundle_hash values are content-addressed; tempdir paths
        // never affect them. They must be byte-identical across runs.
        let report_a: VerifyReport = serde_json::from_slice(&raw_a).unwrap();
        let report_b: VerifyReport = serde_json::from_slice(&raw_b).unwrap();
        assert_eq!(report_a.entries.len(), report_b.entries.len());
        for (a, b) in report_a.entries.iter().zip(report_b.entries.iter()) {
            assert_eq!(a.bundle_name, b.bundle_name);
            assert_eq!(a.bundle_hash, b.bundle_hash, "bundle_hash must be stable");
            assert_eq!(a.status, b.status);
            assert_eq!(a.bundle, b.bundle);
            assert_eq!(a.trace, b.trace);
        }
    }

    fn normalize_report_for_determinism(bytes: &[u8]) -> Vec<u8> {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "run_dir".into(),
                serde_json::Value::String("<run_dir>".into()),
            );
        }
        if let Some(entries) = value.get_mut("entries").and_then(|v| v.as_array_mut()) {
            for entry in entries {
                if let Some(obj) = entry.as_object_mut() {
                    obj.insert(
                        "bundle_dir".into(),
                        serde_json::Value::String("<bundle_dir>".into()),
                    );
                }
            }
        }
        canonical_json(&value).unwrap()
    }

    #[test]
    fn milestone_j_trace_chain_tamper_is_reported() {
        let dir = tempdir().unwrap();
        seed_full_bundle(dir.path(), "trace");
        // Overwrite trace.jsonl with a chain that parses but whose
        // recomputed event hash will not match - single garbled line.
        // We first read the existing trace so we keep the index's
        // recorded SHA-256 happy by NOT modifying the file contents
        // after finalize; instead, we re-seed the bundle and trace,
        // then break only the trace.
        let tamper_dir = tempdir().unwrap();
        seed_full_bundle(tamper_dir.path(), "fresh");
        // Corrupt the middle byte of the event_hash hex in line 1.
        let trace_path = tamper_dir.path().join("trace.jsonl");
        let content = std::fs::read_to_string(&trace_path).unwrap();
        let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
        lines[0] = lines[0].replace(r#""event_hash":""#, r#""event_hash":"ff"#);
        std::fs::write(&trace_path, lines.join("\n") + "\n").unwrap();
        // This also breaks the bundle-level digest, so we expect the
        // bundle check to fail first with FILE_DIGEST_MISMATCH.
        let row = verify_single_bundle(tamper_dir.path(), true);
        assert_eq!(row.status, VerifyStatus::Invalid);
        assert_eq!(
            row.error_code.as_deref(),
            Some(error_codes::FILE_DIGEST_MISMATCH),
            "any mutation of trace.jsonl must first trip the file digest check"
        );
    }
}
