//! Stable identifier types.
//!
//! Every identifier in `eval-ladder` is a newtype so that, for example, a
//! `TaskId` cannot be passed where a `CandidateId` is expected. This discipline
//! is load-bearing for the evidence bundle contract.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::CoreError;

/// Benchmark suite discriminator. Matches the schema enum exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkId {
    /// SWE-Bench Verified (static, curated Python subset).
    SweBenchVerified,
    /// SWE-bench-Live (live, per-task Docker image Python benchmark).
    SweBenchLive,
    /// Rust-SWE-bench (Rust-native repository-level tasks).
    RustSweBench,
}

impl BenchmarkId {
    /// Stable string representation used in paths and logs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SweBenchVerified => "swe_bench_verified",
            Self::SweBenchLive => "swe_bench_live",
            Self::RustSweBench => "rust_swe_bench",
        }
    }

    /// All known benchmark identifiers.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::SweBenchVerified,
            Self::SweBenchLive,
            Self::RustSweBench,
        ]
    }
}

impl fmt::Display for BenchmarkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for BenchmarkId {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "swe_bench_verified" => Ok(Self::SweBenchVerified),
            "swe_bench_live" => Ok(Self::SweBenchLive),
            "rust_swe_bench" => Ok(Self::RustSweBench),
            other => Err(CoreError::UnknownBenchmark(other.to_owned())),
        }
    }
}

// ---------------------------------------------------------------------------
// String-backed task identifier.
// ---------------------------------------------------------------------------

/// Benchmark-local task identifier. Opaque string chosen by each benchmark
/// adapter. Must be non-empty and must be stable across re-ingests.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskId(String);

impl TaskId {
    /// Constructs a new `TaskId`, rejecting empty strings.
    pub fn new(s: impl Into<String>) -> Result<Self, CoreError> {
        let s = s.into();
        if s.is_empty() {
            return Err(CoreError::InvalidId {
                kind: "TaskId",
                value: s,
            });
        }
        Ok(Self(s))
    }

    /// Returns the string representation.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// UUID-backed identifiers.
// ---------------------------------------------------------------------------

macro_rules! uuid_id {
    ($(#[$attr:meta])* $name:ident) => {
        $(#[$attr])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Generates a fresh random identifier (`UUIDv4`).
            #[must_use]
            pub fn new_v4() -> Self {
                Self(Uuid::new_v4())
            }

            /// Generates a fresh time-ordered identifier (`UUIDv7`).
            #[must_use]
            pub fn new_v7() -> Self {
                Self(Uuid::now_v7())
            }

            /// Returns the inner UUID.
            #[inline]
            #[must_use]
            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = CoreError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s)
                    .map(Self)
                    .map_err(|_| CoreError::InvalidId {
                        kind: stringify!($name),
                        value: s.to_owned(),
                    })
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }
    };
}

uuid_id!(/// Identifier for a single evaluator invocation (one run per candidate per batch).
    RunId);

uuid_id!(/// Identifier for a `CandidateResolution`.
    CandidateId);

uuid_id!(/// Identifier for an `EvidenceBundle`.
    BundleId);

/// Identifier for a `ProofObligation`. A benchmark-local string (not a UUID)
/// so obligations can be cross-referenced in papers without resolving UUIDs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObligationId(String);

impl ObligationId {
    /// Constructs a new `ObligationId`, rejecting empty strings.
    pub fn new(s: impl Into<String>) -> Result<Self, CoreError> {
        let s = s.into();
        if s.is_empty() {
            return Err(CoreError::InvalidId {
                kind: "ObligationId",
                value: s,
            });
        }
        Ok(Self(s))
    }

    /// Returns the string representation.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ObligationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_id_roundtrips_through_string() {
        for id in BenchmarkId::all() {
            let s = id.as_str();
            let parsed: BenchmarkId = s.parse().expect("known benchmark id must parse");
            assert_eq!(parsed, *id);
        }
    }

    #[test]
    fn task_id_rejects_empty() {
        assert!(TaskId::new("").is_err());
        assert!(TaskId::new("django__django-12345").is_ok());
    }

    #[test]
    fn run_id_roundtrips_through_string() {
        let id = RunId::new_v4();
        let parsed: RunId = id.to_string().parse().unwrap();
        assert_eq!(id, parsed);
    }
}
