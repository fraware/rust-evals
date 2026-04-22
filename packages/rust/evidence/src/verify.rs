//! Bundle verifier. Recomputes every file's SHA-256 and the overall
//! `bundle_hash` and asserts they match the persisted index.

use std::fs;
use std::path::Path;

use eval_ladder_core::{
    canonical_json, digest, BenchmarkId, BundleId, CandidateId, CoreError, EvaluatorVersion,
    SchemaVersion, Sha256Digest, TaskId,
};
use serde::Serialize;
use thiserror::Error;

use crate::bundle::EvidenceBundleIndex;

/// Errors produced by [`verify_bundle`].
#[derive(Debug, Error)]
pub enum BundleVerifyError {
    /// File system error.
    #[error("bundle verify io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON parse error.
    #[error("bundle verify parse: {0}")]
    Parse(#[from] serde_json::Error),
    /// Canonicalization error.
    #[error("bundle verify core: {0}")]
    Core(#[from] CoreError),
    /// Per-file digest mismatch.
    #[error("file digest mismatch for {path}: expected {expected}, computed {computed}")]
    FileDigestMismatch {
        /// Bundle-relative POSIX path.
        path: String,
        /// Digest recorded in the index.
        expected: String,
        /// Digest computed at verify time.
        computed: String,
    },
    /// Bundle-level digest mismatch.
    #[error("bundle hash mismatch: expected {expected}, computed {computed}")]
    BundleDigestMismatch {
        /// Digest recorded in the index.
        expected: String,
        /// Digest computed at verify time.
        computed: String,
    },
    /// Index declares a file that does not exist on disk.
    #[error("index references missing file: {0}")]
    MissingFile(String),
}

/// Verify a bundle rooted at `root`. Reads `artifact_hashes.json`,
/// re-hashes every declared file, recomputes the bundle hash, and returns
/// the parsed index on success.
pub fn verify_bundle(root: impl AsRef<Path>) -> Result<EvidenceBundleIndex, BundleVerifyError> {
    let root = root.as_ref();
    let index_bytes = fs::read(root.join("artifact_hashes.json"))?;
    let index: EvidenceBundleIndex = serde_json::from_slice(&index_bytes)?;

    // Per-file hashes.
    for entry in &index.files {
        let p = root.join(&entry.path);
        if !p.exists() {
            return Err(BundleVerifyError::MissingFile(entry.path.clone()));
        }
        let bytes = fs::read(&p)?;
        let computed = digest(&bytes);
        if computed != entry.sha256 {
            return Err(BundleVerifyError::FileDigestMismatch {
                path: entry.path.clone(),
                expected: entry.sha256.as_str().to_owned(),
                computed: computed.as_str().to_owned(),
            });
        }
    }

    // Bundle-level hash.
    #[derive(Serialize)]
    struct Hashable<'a> {
        schema_version: SchemaVersion,
        bundle_id: &'a BundleId,
        candidate_id: &'a CandidateId,
        task_id: &'a TaskId,
        benchmark_id: &'a BenchmarkId,
        created_by_version: &'a EvaluatorVersion,
        created_at: &'a chrono::DateTime<chrono::Utc>,
        files: &'a [crate::bundle::FileEntry],
        required_members_present: &'a [String],
    }
    let hashable = Hashable {
        schema_version: index.schema_version,
        bundle_id: &index.bundle_id,
        candidate_id: &index.candidate_id,
        task_id: &index.task_id,
        benchmark_id: &index.benchmark_id,
        created_by_version: &index.created_by_version,
        created_at: &index.created_at,
        files: &index.files,
        required_members_present: &index.required_members_present,
    };
    let recomputed: Sha256Digest = digest(&canonical_json(&hashable)?);
    if recomputed != index.bundle_hash {
        return Err(BundleVerifyError::BundleDigestMismatch {
            expected: index.bundle_hash.as_str().to_owned(),
            computed: recomputed.as_str().to_owned(),
        });
    }

    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleBuilder;
    use crate::MANDATORY_BUNDLE_FILES;
    use eval_ladder_core::{BenchmarkId, CandidateId, TaskId};
    use tempfile::tempdir;

    fn write_mandatory_stub_bundle(root: &Path) {
        for name in MANDATORY_BUNDLE_FILES {
            if *name == "artifact_hashes.json" {
                continue;
            }
            fs::write(root.join(name), "stub").unwrap();
        }
    }

    #[test]
    fn verify_accepts_builder_output() {
        let dir = tempdir().unwrap();
        write_mandatory_stub_bundle(dir.path());
        BundleBuilder::new(
            dir.path(),
            CandidateId::new_v4(),
            TaskId::new("t").unwrap(),
            BenchmarkId::SweBenchVerified,
        )
        .finalize()
        .unwrap();
        verify_bundle(dir.path()).unwrap();
    }

    #[test]
    fn verify_detects_file_tamper() {
        let dir = tempdir().unwrap();
        write_mandatory_stub_bundle(dir.path());
        BundleBuilder::new(
            dir.path(),
            CandidateId::new_v4(),
            TaskId::new("t").unwrap(),
            BenchmarkId::SweBenchVerified,
        )
        .finalize()
        .unwrap();
        fs::write(dir.path().join("stdout.log"), "tampered").unwrap();
        let err = verify_bundle(dir.path()).unwrap_err();
        assert!(matches!(err, BundleVerifyError::FileDigestMismatch { .. }));
    }
}
