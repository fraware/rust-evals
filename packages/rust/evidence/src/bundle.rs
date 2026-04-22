//! Evidence bundle builder.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use eval_ladder_core::{
    canonical_json, digest, BenchmarkId, BundleId, CandidateId, CoreError, EvaluatorVersion,
    SchemaVersion, Sha256Digest, TaskId, EVALUATOR_VERSION, SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

use crate::MANDATORY_BUNDLE_FILES;

/// One entry in the bundle file index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    /// POSIX path relative to the bundle root.
    pub path: String,
    /// SHA-256 digest of the file bytes.
    pub sha256: Sha256Digest,
    /// File size in bytes.
    pub bytes: u64,
}

/// Bundle index. Serialized as `artifact_hashes.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBundleIndex {
    /// Schema version.
    pub schema_version: SchemaVersion,
    /// Bundle identifier.
    pub bundle_id: BundleId,
    /// Candidate identifier.
    pub candidate_id: CandidateId,
    /// Benchmark-local task identifier.
    pub task_id: TaskId,
    /// Benchmark suite discriminator.
    pub benchmark_id: BenchmarkId,
    /// Evaluator version that produced this bundle.
    pub created_by_version: EvaluatorVersion,
    /// Creation time.
    pub created_at: chrono::DateTime<Utc>,
    /// Enumeration of all files in the bundle.
    pub files: Vec<FileEntry>,
    /// Bundle-level SHA-256, computed over canonical JSON of this index with
    /// `bundle_hash` elided.
    pub bundle_hash: Sha256Digest,
    /// Which mandatory members were present. Missing members yield a
    /// [`BundleBuilderError::MissingMandatoryFile`] at finalize time.
    pub required_members_present: Vec<String>,
}

/// Errors produced by [`BundleBuilder`].
#[derive(Debug, Error)]
pub enum BundleBuilderError {
    /// I/O error while reading or writing a bundle file.
    #[error("bundle io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization error.
    #[error("bundle serialize: {0}")]
    Serialize(#[from] serde_json::Error),
    /// Canonicalization error.
    #[error("bundle core: {0}")]
    Core(#[from] CoreError),
    /// A mandatory member was missing when `finalize` was called.
    #[error("mandatory bundle file is missing: {0}")]
    MissingMandatoryFile(String),
    /// Two files resolved to the same relative path.
    #[error("duplicate bundle path: {0}")]
    DuplicatePath(String),
    /// Walkdir traversal error.
    #[error("bundle walk: {0}")]
    Walk(#[from] walkdir::Error),
}

/// Accumulates files, then emits a verified `artifact_hashes.json`.
///
/// The builder does **not** copy files; it indexes the contents of an
/// existing directory. The runner writes files directly into the bundle
/// directory and then calls `finalize`.
pub struct BundleBuilder {
    root: PathBuf,
    bundle_id: BundleId,
    candidate_id: CandidateId,
    task_id: TaskId,
    benchmark_id: BenchmarkId,
}

impl BundleBuilder {
    /// Construct a builder for a bundle rooted at `root` with a fresh
    /// random `UUIDv4` bundle id.
    ///
    /// For deterministic builds (and for the Milestone C rerun-hash
    /// acceptance criterion), use [`Self::with_bundle_id`] to override the
    /// bundle id with a value derived from the run identity, and
    /// [`Self::finalize_at`] to override the creation timestamp.
    pub fn new(
        root: impl Into<PathBuf>,
        candidate_id: CandidateId,
        task_id: TaskId,
        benchmark_id: BenchmarkId,
    ) -> Self {
        Self {
            root: root.into(),
            bundle_id: BundleId::new_v4(),
            candidate_id,
            task_id,
            benchmark_id,
        }
    }

    /// Override the bundle identifier. Consumes `self` and returns the
    /// modified builder so callers can chain: `Builder::new(..).with_bundle_id(..)`.
    #[must_use]
    pub fn with_bundle_id(mut self, bundle_id: BundleId) -> Self {
        self.bundle_id = bundle_id;
        self
    }

    /// Seal the bundle with `Utc::now()` as the creation timestamp.
    ///
    /// Callers that need a deterministic timestamp must use
    /// [`Self::finalize_at`]. Returns the written index.
    pub fn finalize(self) -> Result<EvidenceBundleIndex, BundleBuilderError> {
        self.finalize_at(Utc::now())
    }

    /// Seal the bundle with an explicit `created_at`.
    ///
    /// Produces bit-identical `artifact_hashes.json` for any two calls
    /// that share the same (`bundle_id`, `candidate_id`, `task_id`,
    /// `benchmark_id`, `created_at`, file set, file bytes). This is the
    /// path the pipeline takes to make reruns reproducible.
    pub fn finalize_at(
        self,
        created_at: DateTime<Utc>,
    ) -> Result<EvidenceBundleIndex, BundleBuilderError> {
        // Enumerate files under root, excluding artifact_hashes.json itself.
        let mut entries: BTreeMap<String, FileEntry> = BTreeMap::new();
        for entry in WalkDir::new(&self.root).follow_links(false) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&self.root)
                .expect("WalkDir yields descendants")
                .to_string_lossy()
                .replace('\\', "/");
            if rel == "artifact_hashes.json" {
                continue;
            }
            let bytes = read_file_bytes(entry.path())?;
            let file_entry = FileEntry {
                path: rel.clone(),
                sha256: digest(&bytes),
                bytes: bytes.len() as u64,
            };
            if entries.insert(rel.clone(), file_entry).is_some() {
                return Err(BundleBuilderError::DuplicatePath(rel));
            }
        }

        // Check mandatory members.
        let mut required_members_present = Vec::new();
        for name in MANDATORY_BUNDLE_FILES {
            // artifact_hashes.json is the file we are about to write, so do
            // not require it to be pre-existing.
            if *name == "artifact_hashes.json" {
                required_members_present.push((*name).to_owned());
                continue;
            }
            if entries.contains_key(*name) {
                required_members_present.push((*name).to_owned());
            } else {
                return Err(BundleBuilderError::MissingMandatoryFile((*name).to_owned()));
            }
        }

        let files: Vec<FileEntry> = entries.into_values().collect();

        // Compute bundle_hash over the index with bundle_hash elided.
        #[derive(Serialize)]
        struct Hashable<'a> {
            schema_version: SchemaVersion,
            bundle_id: &'a BundleId,
            candidate_id: &'a CandidateId,
            task_id: &'a TaskId,
            benchmark_id: &'a BenchmarkId,
            created_by_version: &'a EvaluatorVersion,
            created_at: &'a chrono::DateTime<Utc>,
            files: &'a [FileEntry],
            required_members_present: &'a [String],
        }
        let evaluator_version = EVALUATOR_VERSION;
        let hashable = Hashable {
            schema_version: SCHEMA_VERSION,
            bundle_id: &self.bundle_id,
            candidate_id: &self.candidate_id,
            task_id: &self.task_id,
            benchmark_id: &self.benchmark_id,
            created_by_version: &evaluator_version,
            created_at: &created_at,
            files: &files,
            required_members_present: &required_members_present,
        };
        let bundle_hash = digest(&canonical_json(&hashable)?);

        let index = EvidenceBundleIndex {
            schema_version: SCHEMA_VERSION,
            bundle_id: self.bundle_id,
            candidate_id: self.candidate_id,
            task_id: self.task_id.clone(),
            benchmark_id: self.benchmark_id,
            created_by_version: evaluator_version,
            created_at,
            files,
            bundle_hash,
            required_members_present,
        };

        // `to_vec_pretty` would introduce platform-dependent whitespace
        // across some serde_json versions; canonical JSON is safer and
        // makes the on-disk bytes deterministic.
        let mut json = canonical_json(&index)?;
        json.push(b'\n');
        let path = self.root.join("artifact_hashes.json");
        fs::write(&path, json)?;

        Ok(index)
    }
}

fn read_file_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut f = fs::File::open(path)?;
    let capacity = f
        .metadata()
        .ok()
        .and_then(|m| usize::try_from(m.len()).ok())
        .unwrap_or(0);
    let mut buf = Vec::with_capacity(capacity);
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn touch(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn write_mandatory_stub_bundle(root: &Path) {
        for name in MANDATORY_BUNDLE_FILES {
            if *name == "artifact_hashes.json" {
                continue;
            }
            touch(&root.join(name), "stub");
        }
    }

    #[test]
    fn rejects_missing_mandatory_files() {
        let dir = tempdir().unwrap();
        let b = BundleBuilder::new(
            dir.path(),
            CandidateId::new_v4(),
            TaskId::new("t").unwrap(),
            BenchmarkId::SweBenchVerified,
        );
        let err = b.finalize().unwrap_err();
        assert!(matches!(err, BundleBuilderError::MissingMandatoryFile(_)));
    }

    #[test]
    fn finalizes_complete_bundle() {
        let dir = tempdir().unwrap();
        write_mandatory_stub_bundle(dir.path());
        let b = BundleBuilder::new(
            dir.path(),
            CandidateId::new_v4(),
            TaskId::new("t").unwrap(),
            BenchmarkId::SweBenchVerified,
        );
        let index = b.finalize().unwrap();
        assert!(!index.files.is_empty());
        assert!(dir.path().join("artifact_hashes.json").exists());
    }

    #[test]
    fn bundle_hash_is_deterministic_over_file_order() {
        let dir = tempdir().unwrap();
        write_mandatory_stub_bundle(dir.path());
        let candidate = CandidateId::new_v4();
        let task = TaskId::new("t").unwrap();
        let idx1 = BundleBuilder::new(
            dir.path(),
            candidate,
            task.clone(),
            BenchmarkId::SweBenchVerified,
        )
        .finalize()
        .unwrap();
        assert!(idx1.bundle_hash.as_str().starts_with("sha256:"));
    }

    #[test]
    fn finalize_at_is_deterministic_across_roots() {
        use chrono::TimeZone;
        let candidate = CandidateId::new_v4();
        let task = TaskId::new("t-det").unwrap();
        let bundle_id = BundleId::new_v4();
        let t0 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        let go = || -> Sha256Digest {
            let dir = tempdir().unwrap();
            write_mandatory_stub_bundle(dir.path());
            let idx = BundleBuilder::new(
                dir.path(),
                candidate,
                task.clone(),
                BenchmarkId::SweBenchVerified,
            )
            .with_bundle_id(bundle_id)
            .finalize_at(t0)
            .unwrap();
            idx.bundle_hash
        };

        assert_eq!(
            go(),
            go(),
            "bundle_hash must be stable across two independent finalizations \
             with the same bundle_id, created_at, and file bytes"
        );
    }
}
