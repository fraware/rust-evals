//! # eval-ladder-evidence
//!
//! Reproducible evidence-bundle builder.
//!
//! An evidence bundle is a directory containing every input, artifact, and
//! verdict for a single evaluated candidate, plus a top-level index
//! (`artifact_hashes.json`) that enumerates every file with its SHA-256 digest
//! and byte length, plus a `bundle_hash` computed over the canonical JSON of
//! the index with `bundle_hash` itself elided.
//!
//! Mandatory members are listed in `docs/artifact_spec.md` and enforced by
//! [`BundleBuilder::finalize`].
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod bundle;
pub mod verify;

pub use bundle::{BundleBuilder, BundleBuilderError, EvidenceBundleIndex, FileEntry};
pub use verify::{verify_bundle, BundleVerifyError};

/// Mandatory file names at the root of an evidence bundle.
pub const MANDATORY_BUNDLE_FILES: &[&str] = &[
    "candidate_resolution.json",
    "run_manifest.json",
    "trace.jsonl",
    "official_results.json",
    "patch.diff",
    "container_metadata.json",
    "stdout.log",
    "stderr.log",
    "artifact_hashes.json",
];
