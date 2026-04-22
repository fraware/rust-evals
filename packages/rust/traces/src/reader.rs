//! Trace reader that verifies the hash chain.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use eval_ladder_core::CoreError;
use thiserror::Error;

use crate::event::{EventType, TraceEvent};

/// Errors produced by the trace reader.
#[derive(Debug, Error)]
pub enum TraceReaderError {
    /// File I/O error.
    #[error("trace reader io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON deserialization error.
    #[error("trace reader parse (line {line}): {source}")]
    Parse {
        /// 1-based line number.
        line: usize,
        /// Underlying error.
        source: serde_json::Error,
    },
    /// Core-layer error (canonicalization).
    #[error("trace reader core: {0}")]
    Core(#[from] CoreError),
    /// Event hash recomputation did not match the stored hash.
    #[error("event hash mismatch at line {line}: expected {expected}, computed {computed}")]
    HashMismatch {
        /// 1-based line number.
        line: usize,
        /// Stored event hash.
        expected: String,
        /// Recomputed event hash.
        computed: String,
    },
    /// Chain link to previous event is broken.
    #[error(
        "chain break at line {line}: prev_event_hash {prev:?} did not match preceding event hash"
    )]
    ChainBroken {
        /// 1-based line number.
        line: usize,
        /// The `prev_event_hash` declared by this event.
        prev: Option<String>,
    },
    /// First event was not `RunStarted`.
    #[error("first event must be RunStarted, got {0:?}")]
    FirstEventMustBeRunStarted(EventType),
    /// A `RunStarted` appeared after line 1.
    #[error("RunStarted appeared after line 1, at line {0}")]
    DuplicateRunStarted(usize),
}

/// Streaming reader that parses events and verifies the hash chain.
pub struct TraceReader;

impl TraceReader {
    /// Read and verify all events from a JSONL file.
    ///
    /// Fails fast on the first integrity issue.
    pub fn read_and_verify(path: impl AsRef<Path>) -> Result<Vec<TraceEvent>, TraceReaderError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut events: Vec<TraceEvent> = Vec::new();
        let mut last_hash: Option<eval_ladder_core::Sha256Digest> = None;

        for (i, line) in reader.lines().enumerate() {
            let line_number = i + 1;
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let event: TraceEvent =
                serde_json::from_str(&line).map_err(|source| TraceReaderError::Parse {
                    line: line_number,
                    source,
                })?;

            if i == 0 && event.event_type != EventType::RunStarted {
                return Err(TraceReaderError::FirstEventMustBeRunStarted(
                    event.event_type,
                ));
            }
            if i > 0 && event.event_type == EventType::RunStarted {
                return Err(TraceReaderError::DuplicateRunStarted(line_number));
            }

            // Verify chain link.
            if event.prev_event_hash != last_hash {
                return Err(TraceReaderError::ChainBroken {
                    line: line_number,
                    prev: event
                        .prev_event_hash
                        .as_ref()
                        .map(|d| d.as_str().to_owned()),
                });
            }

            // Recompute this event's hash.
            let recomputed = TraceEvent::compute_hash(
                &event.run_id,
                &event.candidate_id,
                &event.task_id,
                &event.timestamp_utc,
                event.event_type,
                &event.payload,
                event.prev_event_hash.as_ref(),
            )?;
            if recomputed != event.event_hash {
                return Err(TraceReaderError::HashMismatch {
                    line: line_number,
                    expected: event.event_hash.as_str().to_owned(),
                    computed: recomputed.as_str().to_owned(),
                });
            }

            last_hash = Some(event.event_hash.clone());
            events.push(event);
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::TraceWriter;
    use eval_ladder_core::{CandidateId, RunId, TaskId};
    use serde_json::Value;
    use tempfile::tempdir;

    #[test]
    fn writer_output_verifies_cleanly() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("trace.jsonl");
        let mut w = TraceWriter::create(
            &path,
            RunId::new_v4(),
            CandidateId::new_v4(),
            TaskId::new("task-x").unwrap(),
        )
        .unwrap();
        w.append(EventType::RunStarted, Value::Null).unwrap();
        w.append(EventType::PatchApplied, Value::Null).unwrap();
        w.append(EventType::RunFinished, Value::Null).unwrap();
        drop(w);

        let events = TraceReader::read_and_verify(&path).unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn tampered_trace_is_rejected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("trace.jsonl");
        let mut w = TraceWriter::create(
            &path,
            RunId::new_v4(),
            CandidateId::new_v4(),
            TaskId::new("task-y").unwrap(),
        )
        .unwrap();
        w.append(EventType::RunStarted, Value::Null).unwrap();
        w.append(EventType::PatchApplied, serde_json::json!({"files": 1}))
            .unwrap();
        w.append(EventType::RunFinished, Value::Null).unwrap();
        drop(w);

        // Tamper: flip a value inside the middle event's payload.
        let original = std::fs::read_to_string(&path).unwrap();
        let tampered = original.replace("\"files\":1", "\"files\":999");
        std::fs::write(&path, tampered).unwrap();

        let err = TraceReader::read_and_verify(&path).unwrap_err();
        assert!(matches!(err, TraceReaderError::HashMismatch { .. }));
    }
}
