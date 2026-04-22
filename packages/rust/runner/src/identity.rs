//! Deterministic run / bundle identity derivation.
//!
//! `RunId` and `BundleId` are `UUIDv4` by default, which is a fine choice
//! for production but destroys the acceptance criterion that two reruns of
//! the same candidate produce identical evidence. This module provides a
//! deterministic alternative: given a fixed seed string, both identifiers
//! are derived via UUID version 5 (name-based, SHA-1) inside a stable
//! namespace.
//!
//! The namespace UUID is baked into the binary and is documented below.
//! It was generated once and is never rotated; changing it would break
//! reproducibility across versions.
//!
//! Callers that do not care about determinism can keep using
//! `RunId::new_v4()` and `BundleId::new_v4()` directly; this module is
//! strictly additive.

use eval_ladder_core::{BundleId, CandidateId, RunId, TaskId};
use uuid::Uuid;

/// Stable namespace for `eval-ladder` deterministic identifiers.
///
/// Generated once; must not change across releases or the evidence hash
/// space is invalidated. If a future design calls for separate namespaces
/// per identifier kind, they must be introduced as *additional* constants,
/// never as replacements.
pub const EVAL_LADDER_NAMESPACE: Uuid = Uuid::from_bytes([
    0xc8, 0x64, 0x2a, 0x1f, 0x8d, 0x3e, 0x4a, 0x17, 0xaa, 0x92, 0xf5, 0x0d, 0x22, 0x3e, 0x9c, 0x01,
]);

/// A pair of (run, bundle) identifiers produced together so that both
/// carry the same reproducibility regime.
#[derive(Debug, Clone, Copy)]
pub struct RunIdentity {
    /// Identifier of the single evaluator invocation.
    pub run_id: RunId,
    /// Identifier of the evidence bundle produced by this invocation.
    pub bundle_id: BundleId,
}

impl RunIdentity {
    /// Generate fresh random identifiers. The usual production path.
    #[must_use]
    pub fn random() -> Self {
        Self {
            run_id: RunId::new_v4(),
            bundle_id: BundleId::new_v4(),
        }
    }

    /// Derive both identifiers from a deterministic seed.
    ///
    /// The seed must capture every input that can legitimately change the
    /// outcome of a run: minimally the candidate id, the task id, and the
    /// evaluator version. The pipeline composes the seed through
    /// [`DeterministicSeed::build`].
    #[must_use]
    pub fn deterministic(seed: &DeterministicSeed) -> Self {
        let run_uuid = Uuid::new_v5(&EVAL_LADDER_NAMESPACE, seed.run_name().as_bytes());
        let bundle_uuid = Uuid::new_v5(&EVAL_LADDER_NAMESPACE, seed.bundle_name().as_bytes());
        Self {
            run_id: RunId::from(run_uuid),
            bundle_id: BundleId::from(bundle_uuid),
        }
    }
}

/// A composable deterministic identity seed.
///
/// The seed is the concatenation of stable, printable fields, joined by
/// the ASCII unit separator (`0x1F`) to avoid collisions between two
/// different tuples producing the same concatenation.
#[derive(Debug, Clone)]
pub struct DeterministicSeed {
    candidate_id: CandidateId,
    task_id: TaskId,
    evaluator_version: String,
    tag: String,
}

impl DeterministicSeed {
    /// Build a seed. `tag` is an opaque user-provided label; passing
    /// different tags for the same candidate produces different run ids
    /// without changing the candidate id.
    #[must_use]
    pub fn build(
        candidate_id: CandidateId,
        task_id: TaskId,
        evaluator_version: impl Into<String>,
        tag: impl Into<String>,
    ) -> Self {
        Self {
            candidate_id,
            task_id,
            evaluator_version: evaluator_version.into(),
            tag: tag.into(),
        }
    }

    fn joined(&self, kind: &str) -> String {
        const US: char = '\u{1F}';
        format!(
            "eval-ladder{us}{kind}{us}{cand}{us}{task}{us}{ver}{us}{tag}",
            us = US,
            kind = kind,
            cand = self.candidate_id,
            task = self.task_id,
            ver = self.evaluator_version,
            tag = self.tag,
        )
    }

    fn run_name(&self) -> String {
        self.joined("run")
    }

    fn bundle_name(&self) -> String {
        self.joined("bundle")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_seed() -> DeterministicSeed {
        // Use UUIDv5 to make the candidate id itself deterministic in the
        // test; that keeps every layer end-to-end reproducible.
        let candidate_uuid = Uuid::new_v5(&EVAL_LADDER_NAMESPACE, b"test-candidate");
        DeterministicSeed::build(
            CandidateId::from(candidate_uuid),
            TaskId::new("fixture__task-1").unwrap(),
            "0.1.0",
            "t0",
        )
    }

    #[test]
    fn deterministic_is_stable() {
        let a = RunIdentity::deterministic(&fixed_seed());
        let b = RunIdentity::deterministic(&fixed_seed());
        assert_eq!(a.run_id, b.run_id);
        assert_eq!(a.bundle_id, b.bundle_id);
    }

    #[test]
    fn run_and_bundle_ids_differ() {
        let a = RunIdentity::deterministic(&fixed_seed());
        assert_ne!(a.run_id.as_uuid(), a.bundle_id.as_uuid());
    }

    #[test]
    fn different_tags_yield_different_runs() {
        let s1 = DeterministicSeed::build(
            CandidateId::from(Uuid::new_v5(&EVAL_LADDER_NAMESPACE, b"c")),
            TaskId::new("t").unwrap(),
            "0.1.0",
            "t0",
        );
        let s2 = DeterministicSeed::build(
            CandidateId::from(Uuid::new_v5(&EVAL_LADDER_NAMESPACE, b"c")),
            TaskId::new("t").unwrap(),
            "0.1.0",
            "t1",
        );
        let a = RunIdentity::deterministic(&s1);
        let b = RunIdentity::deterministic(&s2);
        assert_ne!(a.run_id, b.run_id);
    }

    #[test]
    fn random_differs_from_itself() {
        let a = RunIdentity::random();
        let b = RunIdentity::random();
        assert_ne!(a.run_id, b.run_id);
    }
}
