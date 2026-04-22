//! Lightweight unified-diff path extractor.
//!
//! The L3 policy engine needs to know which repository files a patch
//! modifies so it can enforce edit-scope and dependency-lockfile rules.
//! We intentionally keep this parser focused on extracting *paths*; we
//! do not attempt to reapply or normalize hunks here (the runner crate
//! already defers that to `git apply`).
//!
//! # Grammar subset
//!
//! The extractor recognises two hunk-header styles and picks up paths
//! from both, de-duplicating the union. This handles every patch format
//! emitted by SWE-bench-family tooling, raw `git format-patch`, and
//! `diff -u`:
//!
//! 1. `diff --git a/PATH b/PATH` lines (preferred; unambiguous).
//! 2. `--- a/PATH` and `+++ b/PATH` lines (fallback for non-git diffs).
//!
//! Paths are returned in first-seen order. That keeps downstream policy
//! reports stable across reruns with the same patch bytes.
//!
//! # Edge cases handled
//!
//! - `/dev/null` sentinels (file creation or deletion) are ignored.
//! - Binary file markers (`Binary files a/... and b/... differ`) are
//!   captured through the `diff --git` header so binary changes still
//!   show up in the file list.
//! - Leading `a/` / `b/` prefixes are stripped; bare paths without
//!   prefix are accepted.
//! - Timestamps in `--- a/PATH\tDATE` / `+++ b/PATH\tDATE` are stripped.
//! - Non-UTF-8 bytes cause the patch to be treated as opaque; the
//!   extractor returns an empty vector rather than panicking.

use std::collections::BTreeSet;

/// Extract the unique set of repository-relative paths modified by a
/// patch.
///
/// Empty `patch_bytes` returns an empty vector. Non-UTF-8 input also
/// returns an empty vector: the policy engine has no reliable way to
/// reason about such patches and will surface that as a separate
/// finding if it wants to.
#[must_use]
pub fn modified_paths(patch_bytes: &[u8]) -> Vec<String> {
    if patch_bytes.is_empty() {
        return Vec::new();
    }
    let Ok(text) = std::str::from_utf8(patch_bytes) else {
        return Vec::new();
    };

    let mut ordered: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    for line in text.lines() {
        if let Some(path) = parse_git_header(line) {
            push_unique(&mut ordered, &mut seen, path);
            continue;
        }
        if let Some(path) = parse_minus_header(line) {
            push_unique(&mut ordered, &mut seen, path);
            continue;
        }
        if let Some(path) = parse_plus_header(line) {
            push_unique(&mut ordered, &mut seen, path);
        }
    }

    ordered
}

fn push_unique(ordered: &mut Vec<String>, seen: &mut BTreeSet<String>, path: String) {
    if seen.insert(path.clone()) {
        ordered.push(path);
    }
}

/// Extract the `b/` path from a `diff --git a/X b/Y` header. Renames are
/// recorded under the destination (`b/`) path.
fn parse_git_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("diff --git ")?;
    // Split once from the right on " b/"; SWE-bench paths can contain
    // spaces but by convention avoid ` b/` inside them.
    let (_a, b) = rest.rsplit_once(" b/")?;
    normalize_path(b)
}

fn parse_minus_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("--- ")?;
    let path = rest.split('\t').next().unwrap_or(rest);
    let stripped = path.strip_prefix("a/").unwrap_or(path);
    if stripped == "/dev/null" {
        return None;
    }
    normalize_path(stripped)
}

fn parse_plus_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("+++ ")?;
    let path = rest.split('\t').next().unwrap_or(rest);
    let stripped = path.strip_prefix("b/").unwrap_or(path);
    if stripped == "/dev/null" {
        return None;
    }
    normalize_path(stripped)
}

fn normalize_path(p: &str) -> Option<String> {
    let trimmed = p.trim();
    if trimmed.is_empty() || trimmed == "/dev/null" {
        return None;
    }
    // Normalize to forward slashes; policy globs are authored in POSIX.
    Some(trimmed.replace('\\', "/"))
}

/// Well-known lockfile basenames across ecosystems.
///
/// The list is intentionally narrow: only files whose edit is
/// semantically a dependency change. Application-level "lock" files
/// (for example `.lock` advisory files used by tooling) are not
/// included.
pub const LOCKFILE_BASENAMES: &[&str] = &[
    "Cargo.lock",
    "poetry.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "Pipfile.lock",
    "uv.lock",
    "requirements.lock",
    "go.sum",
    "Gemfile.lock",
    "composer.lock",
    "mix.lock",
];

/// True if any of the supplied paths is a known dependency lockfile.
#[must_use]
pub fn any_lockfile(paths: &[String]) -> bool {
    paths.iter().any(|p| {
        let base = p.rsplit('/').next().unwrap_or(p);
        LOCKFILE_BASENAMES.contains(&base)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_patch_yields_no_paths() {
        assert!(modified_paths(b"").is_empty());
    }

    #[test]
    fn non_utf8_patch_yields_no_paths() {
        let bytes: &[u8] = &[0xff, 0xfe, 0x00];
        assert!(modified_paths(bytes).is_empty());
    }

    #[test]
    fn git_header_wins_over_fallback() {
        let patch = "\
diff --git a/src/lib.rs b/src/lib.rs
index aaaaaaa..bbbbbbb 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-old
+new
";
        assert_eq!(modified_paths(patch.as_bytes()), vec!["src/lib.rs"]);
    }

    #[test]
    fn diff_u_style_fallback() {
        let patch = "\
--- a/pkg/module.py\t2025-01-01
+++ b/pkg/module.py\t2025-01-02
@@ -1 +1 @@
-old
+new
";
        assert_eq!(modified_paths(patch.as_bytes()), vec!["pkg/module.py"]);
    }

    #[test]
    fn dev_null_is_ignored() {
        let patch = "\
diff --git a/new.txt b/new.txt
new file mode 100644
--- /dev/null
+++ b/new.txt
@@ -0,0 +1 @@
+hello
";
        assert_eq!(modified_paths(patch.as_bytes()), vec!["new.txt"]);
    }

    #[test]
    fn multi_file_patch_preserves_first_seen_order() {
        let patch = "\
diff --git a/b.txt b/b.txt
--- a/b.txt
+++ b/b.txt
@@ -1 +1 @@
-x
+y
diff --git a/a.txt b/a.txt
--- a/a.txt
+++ b/a.txt
@@ -1 +1 @@
-x
+y
";
        assert_eq!(modified_paths(patch.as_bytes()), vec!["b.txt", "a.txt"]);
    }

    #[test]
    fn lockfile_detection() {
        assert!(any_lockfile(&["Cargo.lock".into()]));
        assert!(any_lockfile(&["vendor/poetry.lock".into()]));
        assert!(!any_lockfile(&["src/lib.rs".into(), "README.md".into()]));
    }
}
