//! Build a [`RunContext`] from the live [`ExtensionContext`] and the
//! in-flight trace.
//!
//! The L3 policy engine is a pure function over [`RunContext`]
//! ([`crate::engine::evaluate`]). This module is the adapter that
//! converts everything observable at L3 time into that pure input:
//!
//! - `modified_files` come from parsing the candidate patch bytes
//!   ([`crate::diff::modified_paths`]).
//! - `executed_commands` and `trace_events_emitted` come from
//!   re-reading the live `trace.jsonl` via [`TraceReader`], which also
//!   re-validates the hash chain and so doubles as a defense-in-depth
//!   integrity check for earlier rungs.
//! - `reproducible_seed_declared` comes from the candidate's
//!   `generation_metadata.random_seed`.
//! - `nondeterminism_observed` comes from the L1 verdict.
//! - `network_accessed` defaults to whatever the runner reports; for
//!   [`eval_ladder_runner::LocalProcessEngine`] this is always `false`
//!   (no network sandboxing) and the caller may override via
//!   [`L3Observation::with_network_accessed`].
//! - `generated_tests_present` and `dependency_lockfile_edited` are
//!   computed from the patch contents and the bundle layout.

use std::collections::HashSet;
use std::path::Path;

use eval_ladder_core::FailureReason;
use eval_ladder_runner::ExtensionContext;
use eval_ladder_traces::{EventType, TraceEvent, TraceReader, TraceReaderError};
use serde_json::Value;

use crate::diff::{any_lockfile, modified_paths};
use crate::engine::RunContext;

/// Directly-observable signals the runner must supply.
///
/// These are signals the policy extension cannot derive from the
/// trace alone. Today only `network_accessed` lives here; future
/// additions should follow the same "explicitly injected by the
/// runner" pattern.
#[derive(Debug, Clone, Copy, Default)]
pub struct L3Observation {
    /// `true` iff the container engine observed outbound network
    /// activity during the run.
    pub network_accessed: bool,
}

impl L3Observation {
    /// Builder: set the network-accessed flag.
    #[must_use]
    pub const fn with_network_accessed(mut self, v: bool) -> Self {
        self.network_accessed = v;
        self
    }
}

/// Errors raised while assembling a [`RunContext`].
#[derive(Debug, thiserror::Error)]
pub enum ContextBuildError {
    /// Live trace file could not be read or verified.
    #[error("trace: {0}")]
    Trace(#[from] TraceReaderError),
    /// Filesystem I/O error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Build a [`RunContext`] by synthesizing everything L3 can see.
///
/// The trace is re-read from disk and its hash chain re-verified; this
/// also defends against accidental tampering before L3 runs. The
/// trace's `RunFinished` event has not yet been emitted at this point
/// (L3 runs *before* it), but the pipeline contract guarantees it
/// will be, so the synthesized context pre-includes `RunFinished` in
/// `trace_events_emitted` to avoid a spurious `PV_TRACE_INCOMPLETE`
/// finding.
pub fn build_run_context(
    ctx: &ExtensionContext<'_>,
    observation: L3Observation,
) -> Result<RunContext, ContextBuildError> {
    let paths = modified_paths(ctx.patch_bytes);

    let trace_path = ctx.bundle_dir.join("trace.jsonl");
    let events = TraceReader::read_and_verify(&trace_path)?;

    let mut trace_events_emitted: HashSet<EventType> =
        events.iter().map(|e| e.event_type).collect();
    trace_events_emitted.insert(EventType::RunFinished);

    let executed_commands = extract_commands(&events);

    let reproducible_seed_declared = ctx.candidate.generation_metadata.random_seed.is_some();

    let nondeterminism_observed =
        ctx.l1.primary_reason == FailureReason::L1_RERUN_DISAGREEMENT.as_str();

    let dependency_lockfile_edited = any_lockfile(&paths);

    let generated_tests_present = detect_generated_tests(ctx.bundle_dir);

    Ok(RunContext {
        executed_commands,
        modified_files: paths,
        network_accessed: observation.network_accessed,
        reproducible_seed_declared,
        trace_events_emitted,
        generated_tests_present,
        dependency_lockfile_edited,
        nondeterminism_observed,
    })
}

/// Pull the command line from every `OfficialEvalStarted` /
/// `StrengthenedEvalStarted` event payload. Commands are stored as
/// whitespace-joined strings (matching [`crate::engine::evaluate`]'s
/// head-splitting convention) and preserve first-seen order.
fn extract_commands(events: &[TraceEvent]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for ev in events {
        let parts = match ev.event_type {
            EventType::OfficialEvalStarted => commands_from_payload(&ev.payload, "command"),
            EventType::StrengthenedEvalStarted => {
                // L2 emits "commands" as a nested list; also honour "command" for
                // symmetry with L0/L1.
                let mut v = commands_from_payload(&ev.payload, "command");
                v.extend(commands_from_payload(&ev.payload, "commands"));
                v
            }
            _ => continue,
        };
        for cmd in parts {
            if !cmd.is_empty() {
                out.push(cmd);
            }
        }
    }
    out
}

fn commands_from_payload(payload: &Value, key: &str) -> Vec<String> {
    match payload.get(key) {
        Some(Value::Array(arr)) if arr.iter().all(Value::is_string) => {
            // Single `["cmd", "arg", ...]` form.
            let joined = arr
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            vec![joined]
        }
        Some(Value::Array(arr)) => {
            // Array of arrays: e.g. `[["pytest"], ["cargo", "check"]]`.
            arr.iter()
                .filter_map(|v| {
                    let items = v.as_array()?;
                    if !items.iter().all(Value::is_string) {
                        return None;
                    }
                    Some(
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" "),
                    )
                })
                .collect()
        }
        Some(Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn detect_generated_tests(bundle_dir: &Path) -> bool {
    // Convention: generated tests land under `generated_tests/` inside
    // the bundle. The strengthening crate currently does not place
    // anything here, but reserving the directory lets future test
    // generators plug in without changing the policy contract.
    bundle_dir.join("generated_tests").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn commands_from_single_array() {
        let p = json!({"command": ["cargo", "--version"]});
        assert_eq!(
            commands_from_payload(&p, "command"),
            vec!["cargo --version"]
        );
    }

    #[test]
    fn commands_from_string() {
        let p = json!({"command": "pytest -q"});
        assert_eq!(commands_from_payload(&p, "command"), vec!["pytest -q"]);
    }

    #[test]
    fn commands_from_array_of_arrays() {
        let p = json!({"commands": [["pytest"], ["cargo", "check"]]});
        assert_eq!(
            commands_from_payload(&p, "commands"),
            vec!["pytest", "cargo check"]
        );
    }

    #[test]
    fn commands_missing_key_returns_empty() {
        assert!(commands_from_payload(&json!({}), "command").is_empty());
    }
}
