//! Shared normalization helpers used by every adapter.
//!
//! Each adapter composes these helpers differently, but extracting them
//! here guarantees that every benchmark handles issue-id extraction,
//! title truncation, and label construction identically.

use chrono::{DateTime, Utc};

use crate::adapter::BenchmarkAdapterError;
use crate::raw::RawSweBenchRecord;

/// Maximum characters retained for an issue title. SWE-bench problem
/// statements have no separate title; we take the first non-empty line and
/// cap it here so downstream tables do not need to re-truncate.
pub const ISSUE_TITLE_MAX_CHARS: usize = 200;

/// Extract the issue title from a SWE-bench-style problem statement.
///
/// - Trims leading blank lines.
/// - Takes the first non-empty line.
/// - Strips a leading Markdown heading marker (`#`, `##`, ...).
/// - Truncates to [`ISSUE_TITLE_MAX_CHARS`] characters on a character
///   boundary.
#[must_use]
pub fn issue_title_from_problem_statement(text: &str) -> String {
    let first = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let stripped = first.trim_start_matches('#').trim();
    truncate_char_boundary(stripped, ISSUE_TITLE_MAX_CHARS).to_owned()
}

fn truncate_char_boundary(s: &str, max_chars: usize) -> &str {
    if s.chars().count() <= max_chars {
        return s;
    }
    let mut end = 0;
    for (i, (byte_idx, _)) in s.char_indices().enumerate() {
        if i == max_chars {
            return &s[..byte_idx];
        }
        end = byte_idx;
    }
    &s[..end]
}

/// Extract the trailing numeric issue id from a SWE-bench instance id.
///
/// Expected form: `<owner>__<repo>-<issue_number>`. Falls back to the full
/// `instance_id` if the expected shape is not matched.
#[must_use]
pub fn issue_id_from_instance_id(instance_id: &str) -> String {
    instance_id
        .rsplit_once('-')
        .map_or(instance_id, |(_, n)| n)
        .to_owned()
}

/// Parse an RFC 3339 timestamp, returning a clear error with context.
pub fn parse_created_at(
    instance_id: &str,
    raw: &RawSweBenchRecord,
    fallback: DateTime<Utc>,
) -> Result<DateTime<Utc>, BenchmarkAdapterError> {
    match raw.created_at.as_deref() {
        None | Some("") => Ok(fallback),
        Some(ts) => DateTime::parse_from_rfc3339(ts)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                BenchmarkAdapterError::IngestFailed(format!(
                    "task {instance_id}: cannot parse created_at {ts:?}: {e}"
                ))
            }),
    }
}

/// Standard source URL labels for each benchmark.
pub const VERIFIED_SOURCE_URL: &str =
    "https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified";
/// See [`VERIFIED_SOURCE_URL`].
pub const LIVE_SOURCE_URL: &str = "https://huggingface.co/datasets/SWE-bench/SWE-bench-Live";
/// See [`VERIFIED_SOURCE_URL`].
pub const RUST_SOURCE_URL: &str = "https://huggingface.co/datasets/bytedance/Rust-SWE-bench";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_takes_first_non_empty_line() {
        let t = "\n\n# Heading\nbody\nmore\n";
        assert_eq!(issue_title_from_problem_statement(t), "Heading");
    }

    #[test]
    fn title_truncates_on_char_boundary() {
        let long: String = "é".repeat(ISSUE_TITLE_MAX_CHARS + 10);
        let out = issue_title_from_problem_statement(&long);
        assert_eq!(out.chars().count(), ISSUE_TITLE_MAX_CHARS);
    }

    #[test]
    fn title_empty_input_yields_empty() {
        assert_eq!(issue_title_from_problem_statement(""), "");
    }

    #[test]
    fn issue_id_extracts_trailing_number() {
        assert_eq!(issue_id_from_instance_id("astropy__astropy-12907"), "12907");
        assert_eq!(issue_id_from_instance_id("weird-name"), "name");
        assert_eq!(issue_id_from_instance_id("noseparator"), "noseparator");
    }
}
