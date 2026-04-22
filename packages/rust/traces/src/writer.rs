//! Append-only JSONL trace writer.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use eval_ladder_core::{CandidateId, CoreError, RunId, Sha256Digest, TaskId, SCHEMA_VERSION};
use serde_json::Value;
use thiserror::Error;

use crate::event::{EventType, TraceEvent};

/// Errors produced by the trace writer.
#[derive(Debug, Error)]
pub enum TraceWriterError {
    /// File I/O error.
    #[error("trace writer io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization error.
    #[error("trace writer serialize: {0}")]
    Serialize(#[from] serde_json::Error),
    /// Core-layer error (canonicalization).
    #[error("trace writer core: {0}")]
    Core(#[from] CoreError),
    /// The first event of a run must be `RunStarted`.
    #[error("first event must be RunStarted, got {0:?}")]
    FirstEventMustBeRunStarted(EventType),
    /// A second `RunStarted` was emitted.
    #[error("RunStarted emitted more than once")]
    DuplicateRunStarted,
    /// Writer was used after `RunFinished`.
    #[error("writer already sealed by RunFinished")]
    WriterSealed,
}

/// Append-only writer that maintains the hash chain invariant.
///
/// Create with [`Self::create`] and call [`Self::append`] in the event order
/// required by the run. The writer flushes after each event to minimize loss
/// on abrupt termination.
pub struct TraceWriter {
    path: PathBuf,
    inner: BufWriter<File>,
    run_id: RunId,
    candidate_id: CandidateId,
    task_id: TaskId,
    prev_hash: Option<Sha256Digest>,
    started: bool,
    sealed: bool,
}

impl TraceWriter {
    /// Create a new trace file. Fails if the file already exists; this
    /// prevents accidental appends to a sealed chain.
    pub fn create(
        path: impl AsRef<Path>,
        run_id: RunId,
        candidate_id: CandidateId,
        task_id: TaskId,
    ) -> Result<Self, TraceWriterError> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Self {
            path,
            inner: BufWriter::new(file),
            run_id,
            candidate_id,
            task_id,
            prev_hash: None,
            started: false,
            sealed: false,
        })
    }

    /// Append an event to the trace, stamping it with `Utc::now()`.
    ///
    /// Callers that need deterministic timestamps (for example the
    /// evaluation pipeline exercising the rerun-determinism acceptance
    /// criterion) should use [`Self::append_at`] and inject a clock.
    pub fn append(
        &mut self,
        event_type: EventType,
        payload: Value,
    ) -> Result<TraceEvent, TraceWriterError> {
        self.append_at(event_type, payload, Utc::now())
    }

    /// Append an event with an explicit timestamp.
    ///
    /// The hash chain is sealed over the timestamp exactly as it would be
    /// for [`Self::append`]; the two functions produce bit-identical
    /// output when given the same inputs.
    pub fn append_at(
        &mut self,
        event_type: EventType,
        payload: Value,
        timestamp: DateTime<Utc>,
    ) -> Result<TraceEvent, TraceWriterError> {
        if self.sealed {
            return Err(TraceWriterError::WriterSealed);
        }
        if !self.started {
            if event_type != EventType::RunStarted {
                return Err(TraceWriterError::FirstEventMustBeRunStarted(event_type));
            }
        } else if event_type == EventType::RunStarted {
            return Err(TraceWriterError::DuplicateRunStarted);
        }

        let event_hash = TraceEvent::compute_hash(
            &self.run_id,
            &self.candidate_id,
            &self.task_id,
            &timestamp,
            event_type,
            &payload,
            self.prev_hash.as_ref(),
        )?;

        let event = TraceEvent {
            schema_version: SCHEMA_VERSION,
            run_id: self.run_id,
            candidate_id: self.candidate_id,
            task_id: self.task_id.clone(),
            timestamp_utc: timestamp,
            event_type,
            payload,
            prev_event_hash: self.prev_hash.clone(),
            event_hash: event_hash.clone(),
        };

        let line = serde_json::to_string(&event)?;
        self.inner.write_all(line.as_bytes())?;
        self.inner.write_all(b"\n")?;
        self.inner.flush()?;

        self.prev_hash = Some(event_hash);
        self.started = true;
        if event_type == EventType::RunFinished {
            self.sealed = true;
        }
        Ok(event)
    }

    /// Path of the underlying trace file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the writer has observed `RunFinished`.
    #[must_use]
    pub const fn is_sealed(&self) -> bool {
        self.sealed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup() -> (tempfile::TempDir, PathBuf, RunId, CandidateId, TaskId) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("trace.jsonl");
        let run = RunId::new_v4();
        let candidate = CandidateId::new_v4();
        let task = TaskId::new("fixture__task-1").unwrap();
        (dir, path, run, candidate, task)
    }

    #[test]
    fn rejects_wrong_first_event() {
        let (_dir, path, run, candidate, task) = setup();
        let mut w = TraceWriter::create(&path, run, candidate, task).unwrap();
        let err = w.append(EventType::PatchApplied, Value::Null).unwrap_err();
        assert!(matches!(
            err,
            TraceWriterError::FirstEventMustBeRunStarted(_)
        ));
    }

    #[test]
    fn writes_hash_chain() {
        let (_dir, path, run, candidate, task) = setup();
        let mut w = TraceWriter::create(&path, run, candidate, task).unwrap();
        let e1 = w.append(EventType::RunStarted, Value::Null).unwrap();
        let e2 = w.append(EventType::PatchApplied, Value::Null).unwrap();
        assert!(e1.prev_event_hash.is_none());
        assert_eq!(e2.prev_event_hash.as_ref(), Some(&e1.event_hash));
    }

    #[test]
    fn seals_on_run_finished() {
        let (_dir, path, run, candidate, task) = setup();
        let mut w = TraceWriter::create(&path, run, candidate, task).unwrap();
        w.append(EventType::RunStarted, Value::Null).unwrap();
        w.append(EventType::RunFinished, Value::Null).unwrap();
        assert!(w.is_sealed());
        let err = w.append(EventType::PatchApplied, Value::Null).unwrap_err();
        assert!(matches!(err, TraceWriterError::WriterSealed));
    }

    #[test]
    fn append_at_is_deterministic() {
        use chrono::TimeZone;
        let run = RunId::new_v4();
        let candidate = CandidateId::new_v4();
        let task = TaskId::new("fixture__task-1").unwrap();
        let t0 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let t1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 1).unwrap();

        let one = {
            let dir = tempdir().unwrap();
            let path = dir.path().join("trace.jsonl");
            let mut w = TraceWriter::create(&path, run, candidate, task.clone()).unwrap();
            w.append_at(EventType::RunStarted, Value::Null, t0).unwrap();
            w.append_at(EventType::RunFinished, Value::Null, t1)
                .unwrap();
            std::fs::read(&path).unwrap()
        };

        let two = {
            let dir = tempdir().unwrap();
            let path = dir.path().join("trace.jsonl");
            let mut w = TraceWriter::create(&path, run, candidate, task).unwrap();
            w.append_at(EventType::RunStarted, Value::Null, t0).unwrap();
            w.append_at(EventType::RunFinished, Value::Null, t1)
                .unwrap();
            std::fs::read(&path).unwrap()
        };

        assert_eq!(one, two, "explicit timestamps must yield identical bytes");
    }
}
