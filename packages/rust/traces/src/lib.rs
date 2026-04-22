//! # eval-ladder-traces
//!
//! Append-only JSONL event log with SHA-256 hash chaining.
//!
//! The format is specified in `schemas/trace_event.schema.json`. Each event is
//! written as a single line of JSON (with trailing `\n`). Events include:
//!
//! - `prev_event_hash`: `Some(sha256:...)` for all events except the first,
//!   which must be `RunStarted` with `prev_event_hash = None`.
//! - `event_hash`: `sha256:...` computed over the canonical JSON form of the
//!   event with the `event_hash` field itself omitted.
//!
//! This gives the trace integrity properties: tampering with any event
//! invalidates every subsequent event's chain link.
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod event;
pub mod reader;
pub mod writer;

pub use event::{EventType, TraceEvent, REQUIRED_RUN_EVENTS};
pub use reader::{TraceReader, TraceReaderError};
pub use writer::{TraceWriter, TraceWriterError};
