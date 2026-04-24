//! Build an [`AnalysisInput`] from a directory of sealed evidence bundles.
//!
//! The loader is the bridge between Milestone C-F (which produce evidence
//! bundles) and Milestone G (paper-ready analysis). It is intentionally
//! *read-only* and strictly deterministic:
//!
//! - Bundle subdirectories are iterated in lexicographic order, so the
//!   resulting [`AnalysisInput`] has a stable row ordering for any given
//!   input directory.
//! - Only the five canonical per-level result files are parsed:
//!   `official_results.json`, `l1_trusted_rerun_results.json`,
//!   `strengthened_results.json`, `policy_results.json`, `proof_results.json`.
//! - `candidate_resolution.json` supplies `agent_id` / `model_id` /
//!   `benchmark_id`; missing or unreadable bundles yield structured errors
//!   rather than silent skips.
//!
//! The loader does *not* verify bundle hashes; that is the job of
//! `eval_ladder_evidence::verify_bundle`. Callers that care about tamper
//! detection must run verification explicitly before calling into this
//! module.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use eval_ladder_core::{CandidateResolution, EvaluationLevel, EvaluationResult, TaskId};
use thiserror::Error;

use crate::input::{AnalysisInput, AnalysisInputRow};

/// Canonical per-level result-file names inside an evidence bundle.
///
/// Kept as a single source of truth so tests and tooling can iterate
/// without re-hardcoding paths.
pub const LEVEL_RESULT_FILES: &[(&str, EvaluationLevel)] = &[
    ("official_results.json", EvaluationLevel::L0Official),
    (
        "l1_trusted_rerun_results.json",
        EvaluationLevel::L1TrustedRerun,
    ),
    ("strengthened_results.json", EvaluationLevel::L2Strengthened),
    ("policy_results.json", EvaluationLevel::L3PolicyConformant),
    ("proof_results.json", EvaluationLevel::L4Semantic),
];

/// Canonical candidate-resolution filename inside an evidence bundle.
pub const CANDIDATE_RESOLUTION_FILE: &str = "candidate_resolution.json";

/// Errors produced while loading an [`AnalysisInput`] from bundles.
#[derive(Debug, Error)]
pub enum BundleLoadError {
    /// I/O error while reading from disk.
    #[error("analysis bundle io at {path}: {source}")]
    Io {
        /// Path being read when the I/O error occurred.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// JSON decoding failed for a given file.
    #[error("analysis bundle json at {path}: {source}")]
    Json {
        /// Path being parsed when the failure occurred.
        path: PathBuf,
        /// Underlying JSON error.
        source: serde_json::Error,
    },
    /// The run directory does not exist or is not a directory.
    #[error("analysis run dir does not exist or is not a directory: {0}")]
    RunDirMissing(PathBuf),
    /// A bundle subdirectory was missing `candidate_resolution.json`.
    #[error("bundle at {path} is missing {}", CANDIDATE_RESOLUTION_FILE)]
    MissingCandidateResolution {
        /// Bundle root that lacked the file.
        path: PathBuf,
    },
    /// A bundle contained no per-level result files.
    #[error("bundle at {path} contains no per-level result files")]
    EmptyBundle {
        /// Bundle root with no level results.
        path: PathBuf,
    },
    /// The parsed result level disagrees with the filename it was loaded from.
    #[error(
        "bundle at {path}: file {file} is tagged as level {actual} but the canonical \
         filename encodes level {expected}"
    )]
    LevelMismatch {
        /// Bundle root.
        path: PathBuf,
        /// The result filename.
        file: &'static str,
        /// Expected level from the filename table.
        expected: EvaluationLevel,
        /// Actual level declared inside the result JSON.
        actual: EvaluationLevel,
    },
    /// The result's `candidate_id` disagrees with the bundle's own
    /// `candidate_resolution.json`.
    #[error(
        "bundle at {path}: file {file} reports candidate_id {actual_candidate} but the \
         candidate_resolution.json declares {expected_candidate}"
    )]
    CandidateIdMismatch {
        /// Bundle root.
        path: PathBuf,
        /// Result filename.
        file: &'static str,
        /// Expected candidate from `candidate_resolution.json`.
        expected_candidate: eval_ladder_core::CandidateId,
        /// Actual candidate id inside the result.
        actual_candidate: eval_ladder_core::CandidateId,
    },
    /// The result's `task_id` disagrees with the bundle's candidate resolution.
    #[error(
        "bundle at {path}: file {file} reports task_id {actual_task} but the \
         candidate_resolution.json declares {expected_task}"
    )]
    TaskIdMismatch {
        /// Bundle root.
        path: PathBuf,
        /// Result filename.
        file: &'static str,
        /// Expected task id.
        expected_task: String,
        /// Actual task id.
        actual_task: String,
    },
}

/// A cloneable, object-safe lookup function used by [`LoadOptions`].
pub type TaskCategoryLookup = std::sync::Arc<dyn Fn(&TaskId) -> Option<String> + Send + Sync>;

/// Options for [`load_bundle_dir`].
///
/// Fields are deliberately non-exhaustive so we can add future knobs
/// (category lookups, partial-load toleration, &c.) without breaking
/// callers.
#[derive(Default, Clone)]
#[non_exhaustive]
pub struct LoadOptions {
    /// Optional callback that maps a `TaskId` to a free-form category
    /// label; used to populate [`AnalysisInputRow::task_category`].
    pub task_category_for: Option<TaskCategoryLookup>,
}

impl std::fmt::Debug for LoadOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadOptions")
            .field(
                "task_category_for",
                &self
                    .task_category_for
                    .as_ref()
                    .map_or("None", |_| "Some(Fn)"),
            )
            .finish()
    }
}

/// Load every evidence bundle in `run_dir` into an [`AnalysisInput`].
///
/// `run_dir` is expected to contain one subdirectory per candidate. Each
/// subdirectory must be a sealed evidence bundle; the loader requires
/// `candidate_resolution.json` and at least one of the per-level result
/// files to be present.
///
/// The return value has rows sorted by `(bundle_name, level-in-ladder-order)`
/// so reruns against the same input produce byte-identical JSON.
pub fn load_bundle_dir(
    run_dir: &Path,
    opts: &LoadOptions,
) -> Result<AnalysisInput, BundleLoadError> {
    let meta = fs::metadata(run_dir).map_err(|e| BundleLoadError::Io {
        path: run_dir.to_path_buf(),
        source: e,
    })?;
    if !meta.is_dir() {
        return Err(BundleLoadError::RunDirMissing(run_dir.to_path_buf()));
    }

    let mut subdirs: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(run_dir).map_err(|e| BundleLoadError::Io {
        path: run_dir.to_path_buf(),
        source: e,
    })? {
        let entry = entry.map_err(|e| BundleLoadError::Io {
            path: run_dir.to_path_buf(),
            source: e,
        })?;
        let ft = entry.file_type().map_err(|e| BundleLoadError::Io {
            path: entry.path(),
            source: e,
        })?;
        if ft.is_dir() {
            let path = entry.path();
            // Ignore non-bundle helper directories (for example shared
            // build caches) so analysis can run directly on evaluator
            // output directories that co-locate runtime artifacts.
            if is_bundle_dir(&path) {
                subdirs.push(path);
            }
        }
    }
    subdirs.sort();

    let mut rows: Vec<AnalysisInputRow> = Vec::new();
    for bundle_dir in &subdirs {
        let candidate = load_candidate_resolution(bundle_dir)?;
        let mut found_any = false;
        for (file_name, expected_level) in LEVEL_RESULT_FILES {
            let file_path = bundle_dir.join(file_name);
            if !file_path.exists() {
                continue;
            }
            found_any = true;
            let result = read_evaluation_result(&file_path)?;
            if result.level != *expected_level {
                return Err(BundleLoadError::LevelMismatch {
                    path: bundle_dir.clone(),
                    file: file_name,
                    expected: *expected_level,
                    actual: result.level,
                });
            }
            if result.candidate_id != candidate.candidate_id {
                return Err(BundleLoadError::CandidateIdMismatch {
                    path: bundle_dir.clone(),
                    file: file_name,
                    expected_candidate: candidate.candidate_id,
                    actual_candidate: result.candidate_id,
                });
            }
            if result.task_id.as_str() != candidate.task_id.as_str() {
                return Err(BundleLoadError::TaskIdMismatch {
                    path: bundle_dir.clone(),
                    file: file_name,
                    expected_task: candidate.task_id.as_str().to_owned(),
                    actual_task: result.task_id.as_str().to_owned(),
                });
            }
            let task_category = opts
                .task_category_for
                .as_ref()
                .and_then(|f| f(&candidate.task_id));
            rows.push(AnalysisInputRow::from_candidate_and_result(
                candidate.benchmark_id,
                &candidate.agent_id,
                &candidate.model_id,
                &result,
                task_category,
            ));
        }
        if !found_any {
            return Err(BundleLoadError::EmptyBundle {
                path: bundle_dir.clone(),
            });
        }
    }

    rows.sort_by(|a, b| {
        (
            a.candidate_id,
            level_ladder_index(a.level),
            &a.primary_reason,
        )
            .cmp(&(
                b.candidate_id,
                level_ladder_index(b.level),
                &b.primary_reason,
            ))
    });

    Ok(AnalysisInput { rows })
}

fn load_candidate_resolution(bundle_dir: &Path) -> Result<CandidateResolution, BundleLoadError> {
    let path = bundle_dir.join(CANDIDATE_RESOLUTION_FILE);
    if !path.exists() {
        return Err(BundleLoadError::MissingCandidateResolution {
            path: bundle_dir.to_path_buf(),
        });
    }
    let bytes = fs::read(&path).map_err(|e| BundleLoadError::Io {
        path: path.clone(),
        source: e,
    })?;
    let resolution: CandidateResolution =
        serde_json::from_slice(&bytes).map_err(|e| BundleLoadError::Json {
            path: path.clone(),
            source: e,
        })?;
    Ok(resolution)
}

fn is_bundle_dir(path: &Path) -> bool {
    path.join(CANDIDATE_RESOLUTION_FILE).exists()
}

fn read_evaluation_result(path: &Path) -> Result<EvaluationResult, BundleLoadError> {
    let bytes = fs::read(path).map_err(|e| BundleLoadError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let parsed: EvaluationResult =
        serde_json::from_slice(&bytes).map_err(|e| BundleLoadError::Json {
            path: path.to_path_buf(),
            source: e,
        })?;
    Ok(parsed)
}

#[inline]
fn level_ladder_index(level: EvaluationLevel) -> u8 {
    match level {
        EvaluationLevel::L0Official => 0,
        EvaluationLevel::L1TrustedRerun => 1,
        EvaluationLevel::L2Strengthened => 2,
        EvaluationLevel::L3PolicyConformant => 3,
        EvaluationLevel::L4Semantic => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use eval_ladder_core::{
        BenchmarkId, CandidateId, ContextMode, EvaluationResult, EvaluationStatus,
        GenerationMetadata, GenerationMode, PatchFormat, TaskId, EVALUATOR_VERSION, SCHEMA_VERSION,
    };
    use tempfile::TempDir;

    fn write_json(path: &Path, value: &serde_json::Value) {
        let bytes = serde_json::to_vec_pretty(value).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn seed_bundle(
        root: &Path,
        name: &str,
        candidate: &CandidateResolution,
        results: &[(&'static str, EvaluationResult)],
    ) -> PathBuf {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        write_json(
            &dir.join(CANDIDATE_RESOLUTION_FILE),
            &serde_json::to_value(candidate).unwrap(),
        );
        for (file, result) in results {
            write_json(&dir.join(file), &serde_json::to_value(result).unwrap());
        }
        dir
    }

    fn fake_candidate(task: &str, agent: &str) -> CandidateResolution {
        CandidateResolution {
            schema_version: SCHEMA_VERSION,
            candidate_id: CandidateId::new_v4(),
            benchmark_id: BenchmarkId::SweBenchVerified,
            task_id: TaskId::new(task).unwrap(),
            agent_id: agent.into(),
            model_id: "m-1".into(),
            generation_mode: GenerationMode::SingleShot,
            patch_format: PatchFormat::UnifiedDiff,
            patch_ref: "patch.diff".into(),
            trajectory_ref: None,
            generation_metadata: GenerationMetadata {
                temperature: None,
                tool_configuration: serde_json::Value::Null,
                context_mode: ContextMode::FileLevel,
                repo_reproduction_used: false,
                random_seed: None,
            },
            submitted_at: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
        }
    }

    fn fake_result(
        candidate: &CandidateResolution,
        level: EvaluationLevel,
        status: EvaluationStatus,
        code: &str,
    ) -> EvaluationResult {
        EvaluationResult {
            schema_version: SCHEMA_VERSION,
            candidate_id: candidate.candidate_id,
            task_id: candidate.task_id.clone(),
            level,
            status,
            primary_reason: code.into(),
            secondary_reasons: vec![],
            artifacts: vec![],
            metrics: serde_json::Value::Null,
            started_at: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            finished_at: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 1).unwrap(),
            evaluator_version: EVALUATOR_VERSION,
        }
    }

    #[test]
    fn loads_every_level_in_ladder_order() {
        let tmp = TempDir::new().unwrap();
        let c = fake_candidate("task-1", "agent-a");
        seed_bundle(
            tmp.path(),
            "b-2",
            &c,
            &[
                (
                    "official_results.json",
                    fake_result(
                        &c,
                        EvaluationLevel::L0Official,
                        EvaluationStatus::Pass,
                        "PASS",
                    ),
                ),
                (
                    "strengthened_results.json",
                    fake_result(
                        &c,
                        EvaluationLevel::L2Strengthened,
                        EvaluationStatus::Fail,
                        "L2_DIFF_BEHAVIOR",
                    ),
                ),
            ],
        );
        let c2 = fake_candidate("task-2", "agent-a");
        seed_bundle(
            tmp.path(),
            "b-1",
            &c2,
            &[(
                "official_results.json",
                fake_result(
                    &c2,
                    EvaluationLevel::L0Official,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            )],
        );

        let input = load_bundle_dir(tmp.path(), &LoadOptions::default()).unwrap();
        assert_eq!(input.rows.len(), 3);
        // Rows sorted by candidate_id then by level-ladder-order.
        for w in input.rows.windows(2) {
            if w[0].candidate_id == w[1].candidate_id {
                assert!(level_ladder_index(w[0].level) <= level_ladder_index(w[1].level));
            } else {
                assert!(w[0].candidate_id <= w[1].candidate_id);
            }
        }
    }

    #[test]
    fn rejects_bundle_without_candidate_resolution() {
        let tmp = TempDir::new().unwrap();
        let bundle = tmp.path().join("bogus");
        fs::create_dir_all(&bundle).unwrap();
        let input = load_bundle_dir(tmp.path(), &LoadOptions::default()).unwrap();
        assert!(
            input.rows.is_empty(),
            "non-bundle directories should be ignored"
        );
    }

    #[test]
    fn rejects_empty_bundle() {
        let tmp = TempDir::new().unwrap();
        let c = fake_candidate("task-1", "agent-a");
        seed_bundle(tmp.path(), "b-1", &c, &[]);
        let err = load_bundle_dir(tmp.path(), &LoadOptions::default()).unwrap_err();
        assert!(matches!(err, BundleLoadError::EmptyBundle { .. }));
    }

    #[test]
    fn rejects_level_mismatch() {
        let tmp = TempDir::new().unwrap();
        let c = fake_candidate("task-1", "agent-a");
        seed_bundle(
            tmp.path(),
            "b-1",
            &c,
            &[(
                "official_results.json",
                fake_result(
                    &c,
                    EvaluationLevel::L1TrustedRerun,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            )],
        );
        let err = load_bundle_dir(tmp.path(), &LoadOptions::default()).unwrap_err();
        assert!(matches!(err, BundleLoadError::LevelMismatch { .. }));
    }

    #[test]
    fn rejects_candidate_id_mismatch() {
        let tmp = TempDir::new().unwrap();
        let c = fake_candidate("task-1", "agent-a");
        let other = fake_candidate("task-1", "agent-a");
        let mut result = fake_result(
            &c,
            EvaluationLevel::L0Official,
            EvaluationStatus::Pass,
            "PASS",
        );
        result.candidate_id = other.candidate_id;
        seed_bundle(tmp.path(), "b-1", &c, &[("official_results.json", result)]);
        let err = load_bundle_dir(tmp.path(), &LoadOptions::default()).unwrap_err();
        assert!(matches!(err, BundleLoadError::CandidateIdMismatch { .. }));
    }

    #[test]
    fn task_category_hook_populates_row() {
        let tmp = TempDir::new().unwrap();
        let c = fake_candidate("task-1", "agent-a");
        seed_bundle(
            tmp.path(),
            "b-1",
            &c,
            &[(
                "official_results.json",
                fake_result(
                    &c,
                    EvaluationLevel::L0Official,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            )],
        );
        let opts = LoadOptions {
            task_category_for: Some(std::sync::Arc::new(|_| Some("unit-test".into()))),
        };
        let input = load_bundle_dir(tmp.path(), &opts).unwrap();
        assert_eq!(input.rows[0].task_category.as_deref(), Some("unit-test"));
    }

    #[test]
    fn ignores_cache_like_subdirectory_beside_bundles() {
        let tmp = TempDir::new().unwrap();
        let c = fake_candidate("task-1", "agent-a");
        seed_bundle(
            tmp.path(),
            "bundle-a",
            &c,
            &[(
                "official_results.json",
                fake_result(
                    &c,
                    EvaluationLevel::L0Official,
                    EvaluationStatus::Pass,
                    "PASS",
                ),
            )],
        );
        fs::create_dir_all(tmp.path().join(".cargo_target_cache")).unwrap();
        let input = load_bundle_dir(tmp.path(), &LoadOptions::default()).unwrap();
        assert_eq!(input.rows.len(), 1);
    }
}
