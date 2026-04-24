//! Paper-ready export of every analysis table for a run directory.
//!
//! Milestone G materializes the canonical tables into a single output
//! directory so the paper pipeline can reference a stable filename
//! contract:
//!
//! - `score_descent.csv` / `.json`
//! - `conditional_false_success.csv` / `.json`
//! - `rank_stability.csv` / `.json`
//! - `taxonomy.csv` / `.json`
//! - `static_vs_live.csv` / `.json` (Milestone L)
//! - `manifest.json` - canonical-JSON manifest with a SHA-256 of every
//!   emitted file, the evaluator version, and the input row count. This
//!   is the single audit-stable artifact that downstream tooling
//!   (paper builds, CI) hashes to detect drift.
//!
//! The writer is strictly deterministic: rows and JSON keys are sorted
//! the same way the rest of the evaluator canonicalizes data, and
//! timestamps are *not* written into any file. Re-running
//! [`write_paper_exports`] against the same [`AnalysisInput`] must
//! produce byte-identical bytes on disk; this invariant is pinned by the
//! Milestone G acceptance test.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use eval_ladder_core::{canonical_json, digest, EvaluatorVersion, Sha256Digest, EVALUATOR_VERSION};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::csv::write_table;
use crate::input::{AnalysisInput, AnalysisMode};
use crate::rank_stability::{rank_stability, RankStabilityRow};
use crate::score_descent::{self, score_descent, ConditionalFalseSuccessRow, ScoreDescentRow};
use crate::static_vs_live::{static_vs_live, StaticVsLiveRow};
use crate::taxonomy::{taxonomy_counts, TaxonomyRow};

/// Schema version for [`PaperExportManifest`].
///
/// Bumped to `2` in Milestone L when the `static_vs_live.{csv,json}`
/// pair was added to the manifest. Bumped to `3` when
/// `analysis_mode` was added to make raw-vs-cumulative semantics
/// explicit in exported artifacts.
pub const PAPER_EXPORT_SCHEMA_VERSION: u32 = 3;

/// Output file names.
const SCORE_DESCENT_CSV: &str = "score_descent.csv";
const SCORE_DESCENT_JSON: &str = "score_descent.json";
const CONDITIONAL_CSV: &str = "conditional_false_success.csv";
const CONDITIONAL_JSON: &str = "conditional_false_success.json";
const RANK_STABILITY_CSV: &str = "rank_stability.csv";
const RANK_STABILITY_JSON: &str = "rank_stability.json";
const TAXONOMY_CSV: &str = "taxonomy.csv";
const TAXONOMY_JSON: &str = "taxonomy.json";
const STATIC_VS_LIVE_CSV: &str = "static_vs_live.csv";
const STATIC_VS_LIVE_JSON: &str = "static_vs_live.json";
const MANIFEST_JSON: &str = "manifest.json";

/// Errors produced by [`write_paper_exports`].
#[derive(Debug, Error)]
pub enum PaperExportError {
    /// I/O error while writing a file.
    #[error("paper export io at {path}: {source}")]
    Io {
        /// Path being written.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// Canonical JSON serialization failed.
    #[error("paper export canonicalize: {0}")]
    Canonicalize(#[from] eval_ladder_core::CoreError),
    /// JSON value serialization failed.
    #[error("paper export json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Single entry in the paper-export manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaperExport {
    /// File name relative to the export directory.
    pub path: String,
    /// SHA-256 of the written bytes.
    pub sha256: Sha256Digest,
    /// File size in bytes.
    pub bytes: u64,
}

/// Ordered collection of exports written to disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaperExportSet {
    /// One entry per file, sorted by `path`.
    pub files: Vec<PaperExport>,
}

/// Canonical manifest for a paper-export directory.
///
/// Serializes through `eval_ladder_core::canonical_json` so two runs on
/// the same [`AnalysisInput`] produce byte-identical manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaperExportManifest {
    /// Schema version.
    pub schema_version: u32,
    /// Evaluator version that generated the exports.
    pub evaluator_version: EvaluatorVersion,
    /// Row count of the input passed to [`write_paper_exports`].
    pub input_row_count: u64,
    /// Analysis semantics used to produce every table in this export set.
    pub analysis_mode: AnalysisMode,
    /// One entry per emitted file.
    pub files: Vec<PaperExport>,
}

/// Write every paper table into `out_dir` and return the manifest.
///
/// The directory is created if it does not exist. Existing files with
/// matching names are overwritten.
pub fn write_paper_exports(
    input: &AnalysisInput,
    out_dir: &Path,
    mode: AnalysisMode,
) -> Result<PaperExportManifest, PaperExportError> {
    fs::create_dir_all(out_dir).map_err(|e| PaperExportError::Io {
        path: out_dir.to_path_buf(),
        source: e,
    })?;

    let mut pairs: Vec<PaperExportPair> = Vec::with_capacity(5);

    // Score descent.
    let score_descent_rows = score_descent(input, mode);
    pairs.push(write_csv_and_json(
        out_dir,
        SCORE_DESCENT_CSV,
        SCORE_DESCENT_JSON,
        &[
            "benchmark_id",
            "agent_id",
            "level",
            "passed",
            "evaluated",
            "pass_rate",
        ],
        &score_descent_rows,
        |r: &ScoreDescentRow| {
            vec![
                r.stratum
                    .benchmark_id
                    .map(|b| b.as_str().to_owned())
                    .unwrap_or_default(),
                r.stratum.agent_id.clone().unwrap_or_default(),
                r.level.short_code().to_owned(),
                r.passed.to_string(),
                r.evaluated.to_string(),
                format_optional_f64(r.pass_rate),
            ]
        },
    )?);

    // Conditional false-success rate.
    let cfs_rows = score_descent::conditional_false_success_with_mode(input, mode);
    pairs.push(write_csv_and_json(
        out_dir,
        CONDITIONAL_CSV,
        CONDITIONAL_JSON,
        &[
            "level_from",
            "level_to",
            "n_passed_from",
            "n_failed_to",
            "rate",
        ],
        &cfs_rows,
        |r: &ConditionalFalseSuccessRow| {
            vec![
                r.level_from.short_code().to_owned(),
                r.level_to.short_code().to_owned(),
                r.n_passed_from.to_string(),
                r.n_failed_to.to_string(),
                format_optional_f64(r.rate),
            ]
        },
    )?);

    // Rank stability.
    let rank_rows = rank_stability(input, mode);
    pairs.push(write_csv_and_json(
        out_dir,
        RANK_STABILITY_CSV,
        RANK_STABILITY_JSON,
        &["level_a", "level_b", "n_agents", "kendall_tau_b"],
        &rank_rows,
        |r: &RankStabilityRow| {
            vec![
                r.level_a.short_code().to_owned(),
                r.level_b.short_code().to_owned(),
                r.n_agents.to_string(),
                format_optional_f64(r.kendall_tau_b),
            ]
        },
    )?);

    // Taxonomy.
    let taxonomy_rows = taxonomy_counts(input);
    pairs.push(write_csv_and_json(
        out_dir,
        TAXONOMY_CSV,
        TAXONOMY_JSON,
        &["benchmark_id", "level", "primary_reason", "count"],
        &taxonomy_rows,
        |r: &TaxonomyRow| {
            vec![
                r.benchmark_id.as_str().to_owned(),
                r.level.short_code().to_owned(),
                r.primary_reason.clone(),
                r.count.to_string(),
            ]
        },
    )?);

    // Static-vs-live comparison (Milestone L). Headline paper table.
    let svl_rows = static_vs_live(input, mode);
    pairs.push(write_csv_and_json(
        out_dir,
        STATIC_VS_LIVE_CSV,
        STATIC_VS_LIVE_JSON,
        &[
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
        ],
        &svl_rows,
        |r: &StaticVsLiveRow| {
            vec![
                r.agent_id.clone(),
                r.level.short_code().to_owned(),
                r.static_passed.to_string(),
                r.static_evaluated.to_string(),
                format_optional_f64(r.static_pass_rate),
                r.live_passed.to_string(),
                r.live_evaluated.to_string(),
                format_optional_f64(r.live_pass_rate),
                format_optional_f64(r.delta),
                format_optional_f64(r.ratio),
            ]
        },
    )?);

    let mut files: Vec<PaperExport> = pairs
        .into_iter()
        .flat_map(PaperExportPair::into_iter)
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let row_count = u64::try_from(input.rows.len()).unwrap_or(u64::MAX);
    let manifest = PaperExportManifest {
        schema_version: PAPER_EXPORT_SCHEMA_VERSION,
        evaluator_version: EVALUATOR_VERSION,
        input_row_count: row_count,
        analysis_mode: mode,
        files,
    };

    let canonical = canonical_json(&manifest)?;
    let manifest_path = out_dir.join(MANIFEST_JSON);
    fs::write(&manifest_path, &canonical).map_err(|e| PaperExportError::Io {
        path: manifest_path,
        source: e,
    })?;

    Ok(manifest)
}

/// A `(csv, json)` pair produced by [`write_csv_and_json`].
struct PaperExportPair {
    csv: PaperExport,
    json: PaperExport,
}

impl PaperExportPair {
    fn into_iter(self) -> std::vec::IntoIter<PaperExport> {
        vec![self.csv, self.json].into_iter()
    }
}

fn write_csv_and_json<T: Serialize>(
    out_dir: &Path,
    csv_name: &str,
    json_name: &str,
    header: &[&str],
    rows: &[T],
    to_fields: impl Fn(&T) -> Vec<String>,
) -> Result<PaperExportPair, PaperExportError> {
    // CSV
    let mut csv_bytes: Vec<u8> = Vec::new();
    write_table(&mut csv_bytes, header, rows, &to_fields).map_err(|e| PaperExportError::Io {
        path: out_dir.join(csv_name),
        source: e,
    })?;
    let csv_path = out_dir.join(csv_name);
    fs::write(&csv_path, &csv_bytes).map_err(|e| PaperExportError::Io {
        path: csv_path.clone(),
        source: e,
    })?;
    let csv_entry = PaperExport {
        path: csv_name.to_owned(),
        sha256: digest(&csv_bytes),
        bytes: u64::try_from(csv_bytes.len()).unwrap_or(u64::MAX),
    };

    // JSON (canonical). Wrap in a `Vec` so the serializer sees a sized
    // owned value; `rows` is a `&[T]` which `canonical_json` cannot size.
    let json_value = serde_json::to_value(rows)?;
    let json_bytes = canonical_json(&json_value)?;
    let json_path = out_dir.join(json_name);
    fs::write(&json_path, &json_bytes).map_err(|e| PaperExportError::Io {
        path: json_path.clone(),
        source: e,
    })?;
    let json_entry = PaperExport {
        path: json_name.to_owned(),
        sha256: digest(&json_bytes),
        bytes: u64::try_from(json_bytes.len()).unwrap_or(u64::MAX),
    };

    Ok(PaperExportPair {
        csv: csv_entry,
        json: json_entry,
    })
}

#[inline]
fn format_optional_f64(v: Option<f64>) -> String {
    // Fixed precision so CSV diffs are stable across runs and platforms.
    v.map_or_else(String::new, |v| format!("{v:.6}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::AnalysisInputRow;
    use eval_ladder_core::{BenchmarkId, CandidateId, EvaluationLevel, EvaluationStatus, TaskId};
    use tempfile::TempDir;

    fn fixture_input() -> AnalysisInput {
        let c1 = CandidateId::new_v4();
        let c2 = CandidateId::new_v4();
        AnalysisInput {
            rows: vec![
                AnalysisInputRow {
                    candidate_id: c1,
                    task_id: TaskId::new("t1").unwrap(),
                    benchmark_id: BenchmarkId::SweBenchVerified,
                    agent_id: "a".into(),
                    model_id: "m".into(),
                    level: EvaluationLevel::L0Official,
                    status: EvaluationStatus::Pass,
                    primary_reason: "PASS".into(),
                    task_category: None,
                },
                AnalysisInputRow {
                    candidate_id: c2,
                    task_id: TaskId::new("t2").unwrap(),
                    benchmark_id: BenchmarkId::SweBenchVerified,
                    agent_id: "a".into(),
                    model_id: "m".into(),
                    level: EvaluationLevel::L2Strengthened,
                    status: EvaluationStatus::Fail,
                    primary_reason: "L2_DIFF_BEHAVIOR".into(),
                    task_category: None,
                },
            ],
        }
    }

    #[test]
    fn paper_export_is_deterministic() {
        let input = fixture_input();
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();
        let m_a = write_paper_exports(&input, tmp_a.path(), AnalysisMode::Cumulative).unwrap();
        let m_b = write_paper_exports(&input, tmp_b.path(), AnalysisMode::Cumulative).unwrap();
        assert_eq!(m_a, m_b, "manifest manifests must match");
        for f in &m_a.files {
            let a = fs::read(tmp_a.path().join(&f.path)).unwrap();
            let b = fs::read(tmp_b.path().join(&f.path)).unwrap();
            assert_eq!(a, b, "file {} differs across runs", f.path);
        }
        let manifest_a = fs::read(tmp_a.path().join(MANIFEST_JSON)).unwrap();
        let manifest_b = fs::read(tmp_b.path().join(MANIFEST_JSON)).unwrap();
        assert_eq!(manifest_a, manifest_b);
    }

    #[test]
    fn paper_export_writes_every_expected_file() {
        let input = fixture_input();
        let tmp = TempDir::new().unwrap();
        write_paper_exports(&input, tmp.path(), AnalysisMode::Cumulative).unwrap();
        for f in [
            SCORE_DESCENT_CSV,
            SCORE_DESCENT_JSON,
            CONDITIONAL_CSV,
            CONDITIONAL_JSON,
            RANK_STABILITY_CSV,
            RANK_STABILITY_JSON,
            TAXONOMY_CSV,
            TAXONOMY_JSON,
            STATIC_VS_LIVE_CSV,
            STATIC_VS_LIVE_JSON,
            MANIFEST_JSON,
        ] {
            assert!(
                tmp.path().join(f).is_file(),
                "expected paper export file {f} to exist"
            );
        }
    }
}
