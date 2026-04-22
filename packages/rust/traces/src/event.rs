//! Trace event type and serialization helpers.

use chrono::{DateTime, Utc};
use eval_ladder_core::{
    canonical_json, digest, CandidateId, RunId, SchemaVersion, Sha256Digest, TaskId, SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Stable event-type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// Emitted first. Carries run-level metadata in its payload.
    RunStarted,
    /// Container image prepared.
    ContainerPrepared,
    /// Patch applied to the working tree.
    PatchApplied,
    /// Official scorer started.
    OfficialEvalStarted,
    /// Official scorer finished.
    OfficialEvalFinished,
    /// Strengthened validator started.
    StrengthenedEvalStarted,
    /// Strengthened validator finished.
    StrengthenedEvalFinished,
    /// Policy check started.
    PolicyCheckStarted,
    /// Policy violation detected. Multiple events may be emitted per run.
    PolicyViolationDetected,
    /// Proof check started.
    ProofCheckStarted,
    /// Proof check finished.
    ProofCheckFinished,
    /// Emitted last. Carries overall status in its payload.
    RunFinished,
}

/// Required-event set for policy completeness checks.
///
/// The default L3 policy enforces that every run contains at least these events.
pub const REQUIRED_RUN_EVENTS: &[EventType] = &[
    EventType::RunStarted,
    EventType::PatchApplied,
    EventType::OfficialEvalStarted,
    EventType::OfficialEvalFinished,
    EventType::RunFinished,
];

/// Single append-only trace event.
///
/// Mirrors `schemas/trace_event.schema.json`. The serialization order matches
/// the schema for reviewer ergonomics; canonical JSON hashing sorts keys
/// regardless, so this ordering is purely cosmetic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceEvent {
    /// Schema version.
    pub schema_version: SchemaVersion,
    /// Run identifier this event belongs to.
    pub run_id: RunId,
    /// Candidate identifier.
    pub candidate_id: CandidateId,
    /// Benchmark-local task identifier.
    pub task_id: TaskId,
    /// UTC timestamp.
    pub timestamp_utc: DateTime<Utc>,
    /// Event type.
    pub event_type: EventType,
    /// Free-form payload.
    pub payload: Value,
    /// SHA-256 of the previous event's canonical JSON (with its own
    /// `event_hash` omitted). `None` only for `RunStarted`.
    pub prev_event_hash: Option<Sha256Digest>,
    /// SHA-256 of this event's canonical JSON (with `event_hash` omitted).
    pub event_hash: Sha256Digest,
}

impl TraceEvent {
    /// Compute the event hash for an event builder (without the `event_hash`
    /// field). Used by the writer to seal events.
    pub(crate) fn compute_hash(
        run_id: &RunId,
        candidate_id: &CandidateId,
        task_id: &TaskId,
        timestamp_utc: &DateTime<Utc>,
        event_type: EventType,
        payload: &Value,
        prev_event_hash: Option<&Sha256Digest>,
    ) -> Result<Sha256Digest, eval_ladder_core::CoreError> {
        #[derive(Serialize)]
        struct Hashable<'a> {
            schema_version: SchemaVersion,
            run_id: &'a RunId,
            candidate_id: &'a CandidateId,
            task_id: &'a TaskId,
            timestamp_utc: &'a DateTime<Utc>,
            event_type: EventType,
            payload: &'a Value,
            prev_event_hash: Option<&'a Sha256Digest>,
        }
        let bytes = canonical_json(&Hashable {
            schema_version: SCHEMA_VERSION,
            run_id,
            candidate_id,
            task_id,
            timestamp_utc,
            event_type,
            payload,
            prev_event_hash,
        })?;
        Ok(digest(&bytes))
    }
}
