//! Shared normalization helpers used by every adapter.
//!
//! Each adapter composes these helpers differently, but extracting them
//! here guarantees that every benchmark handles issue-id extraction,
//! title truncation, and label construction identically.

use chrono::{DateTime, Utc};
use regex::Regex;

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

/// Build a deterministic pytest entrypoint from SWE-bench test selectors.
///
/// The selector set is `FAIL_TO_PASS`, deduplicated and sorted to guarantee
/// byte-stable manifests.
#[must_use]
pub fn official_pytest_entrypoint(raw: &RawSweBenchRecord) -> String {
    let mut selectors = normalized_fail_to_pass_selectors(raw);
    selectors.sort();
    selectors.dedup();

    if raw.repo.eq_ignore_ascii_case("django/django") {
        if selectors.is_empty() {
            "python tests/runtests.py".to_owned()
        } else {
            format!("python tests/runtests.py {}", selectors.join(" "))
        }
    } else if selectors.is_empty() {
        "python -m pytest".to_owned()
    } else {
        format!("python -m pytest {}", selectors.join(" "))
    }
}

fn normalized_fail_to_pass_selectors(raw: &RawSweBenchRecord) -> Vec<String> {
    let django = raw.repo.eq_ignore_ascii_case("django/django");
    let mut out: Vec<String> = Vec::new();
    let token_re = Regex::new(r"^[A-Za-z0-9_./:-]+$").expect("valid selector regex");
    let django_unittest_re =
        Regex::new(r"^([A-Za-z0-9_]+)\s+\(([A-Za-z0-9_\.]+)\)$").expect("valid django regex");
    for selector in raw.fail_to_pass_list() {
        let s = selector.trim();
        if s.is_empty() {
            continue;
        }
        if django {
            if let Some(caps) = django_unittest_re.captures(s) {
                let test_name = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
                let suite = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
                if !test_name.is_empty() && !suite.is_empty() {
                    out.push(format!("{suite}.{test_name}"));
                    continue;
                }
            }
            for tok in s.split_whitespace() {
                let t = tok.trim();
                if t.is_empty() || t.starts_with("--") || t.contains('(') || t.contains(')') {
                    continue;
                }
                if !token_re.is_match(t) {
                    continue;
                }
                if t.matches('.').count() < 2 {
                    continue;
                }
                out.push(t.to_owned());
            }
        } else if let Some((head, _param)) = s.split_once('[') {
            // Historical selector snapshots may include parametrized ids that
            // no longer match the exact test node name in current fixtures.
            out.push(head.to_owned());
        } else {
            out.push(s.to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    #[test]
    fn pytest_entrypoint_is_sorted_and_deduped() {
        let raw: RawSweBenchRecord = serde_json::from_value(json!({
            "instance_id": "a__b-1",
            "repo": "a/b",
            "base_commit": "deadbeef",
            "problem_statement": "p",
            "FAIL_TO_PASS": ["z::t", "a::t", "z::t"],
            "PASS_TO_PASS": "[\"b::t\",\"a::t\"]"
        }))
        .unwrap();
        let cmd = official_pytest_entrypoint(&raw);
        assert_eq!(cmd, "python -m pytest a::t z::t");
    }

    #[test]
    fn pytest_entrypoint_falls_back_when_no_selectors() {
        let raw: RawSweBenchRecord = serde_json::from_value(json!({
            "instance_id": "a__b-1",
            "repo": "a/b",
            "base_commit": "deadbeef",
            "problem_statement": "p"
        }))
        .unwrap();
        let cmd = official_pytest_entrypoint(&raw);
        assert_eq!(cmd, "python -m pytest");
    }

    #[test]
    fn django_uses_runtests_entrypoint() {
        let raw: RawSweBenchRecord = serde_json::from_value(json!({
            "instance_id": "django__django-1",
            "repo": "django/django",
            "base_commit": "deadbeef",
            "problem_statement": "p",
            "FAIL_TO_PASS": ["auth_tests.test_views.LoginTest.test_security_check"]
        }))
        .unwrap();
        let cmd = official_pytest_entrypoint(&raw);
        assert_eq!(
            cmd,
            "python tests/runtests.py auth_tests.test_views.LoginTest.test_security_check"
        );
    }

    #[test]
    fn strips_pytest_param_suffix_from_selector() {
        let raw: RawSweBenchRecord = serde_json::from_value(json!({
            "instance_id": "a__b-1",
            "repo": "astropy/astropy",
            "base_commit": "deadbeef",
            "problem_statement": "p",
            "FAIL_TO_PASS": ["pkg/tests/test_mod.py::test_case[param1]"]
        }))
        .unwrap();
        let cmd = official_pytest_entrypoint(&raw);
        assert_eq!(cmd, "python -m pytest pkg/tests/test_mod.py::test_case");
    }

    #[test]
    fn django_selector_normalization_filters_noise_tokens() {
        let raw: RawSweBenchRecord = serde_json::from_value(json!({
            "instance_id": "django__django-1",
            "repo": "django/django",
            "base_commit": "deadbeef",
            "problem_statement": "p",
            "FAIL_TO_PASS": [
                "--username isn't provided. auth_tests.test_views.LoginTest.test_security_check (x.y) test_other.case"
            ]
        }))
        .unwrap();
        let cmd = official_pytest_entrypoint(&raw);
        assert_eq!(
            cmd,
            "python tests/runtests.py auth_tests.test_views.LoginTest.test_security_check"
        );
    }

    #[test]
    fn django_unittest_style_selector_is_converted() {
        let raw: RawSweBenchRecord = serde_json::from_value(json!({
            "instance_id": "django__django-1",
            "repo": "django/django",
            "base_commit": "deadbeef",
            "problem_statement": "p",
            "FAIL_TO_PASS": [
                "test_union_with_values_list_and_order (queries.test_qs_combinators.QuerySetSetOperationTests)"
            ]
        }))
        .unwrap();
        let cmd = official_pytest_entrypoint(&raw);
        assert_eq!(
            cmd,
            "python tests/runtests.py queries.test_qs_combinators.QuerySetSetOperationTests.test_union_with_values_list_and_order"
        );
    }
}
