//! JSONL obligation manifest.
//!
//! `datasets/derived/proof_subset/manifest.jsonl` is the authoritative
//! index of every [`ProofObligation`] attached to the curated subset.
//! The file has one obligation per line (JSONL). Empty lines and
//! comments (lines whose first non-whitespace character is `#`) are
//! tolerated so reviewers can annotate the manifest in PRs.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use thiserror::Error;

use crate::spec::{ProofObligation, ProofObligationLoadError};

/// In-memory obligation index keyed by `task_id`.
///
/// Iteration order matches the original JSONL line order so downstream
/// summaries (for example `prove-subset` output) are stable across
/// reruns.
#[derive(Debug, Clone, Default)]
pub struct ObligationManifest {
    by_task: BTreeMap<String, ProofObligation>,
    ordered_task_ids: Vec<String>,
}

impl ObligationManifest {
    /// Empty manifest (every task gets `NotApplicable`).
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            by_task: BTreeMap::new(),
            ordered_task_ids: Vec::new(),
        }
    }

    /// Load a manifest from a JSONL file. Fails fast on the first
    /// malformed line.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ObligationManifestError> {
        let p = path.as_ref();
        let file = File::open(p).map_err(|source| ObligationManifestError::Io {
            path: p.display().to_string(),
            source,
        })?;
        Self::from_reader(BufReader::new(file))
    }

    /// Load a manifest from any `BufRead`.
    pub fn from_reader<R: BufRead>(reader: R) -> Result<Self, ObligationManifestError> {
        let mut out = Self::empty();
        for (i, line) in reader.lines().enumerate() {
            let line_number = i + 1;
            let line = line.map_err(|source| ObligationManifestError::Io {
                path: format!("<line {line_number}>"),
                source,
            })?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let obl = ProofObligation::from_slice(trimmed.as_bytes()).map_err(|source| {
                ObligationManifestError::Line {
                    line: line_number,
                    source,
                }
            })?;
            out.insert(obl)
                .map_err(|source| ObligationManifestError::Line {
                    line: line_number,
                    source,
                })?;
        }
        Ok(out)
    }

    /// Insert an obligation. Rejects duplicate `task_id`s: one
    /// obligation per task is the current contract.
    pub fn insert(&mut self, obl: ProofObligation) -> Result<(), ProofObligationLoadError> {
        let tid = obl.task_id.clone();
        if self.by_task.contains_key(&tid) {
            return Err(ProofObligationLoadError::Invalid(format!(
                "duplicate obligation for task_id {tid}"
            )));
        }
        self.by_task.insert(tid.clone(), obl);
        self.ordered_task_ids.push(tid);
        Ok(())
    }

    /// Lookup by task id (accepts anything that stringifies to the id).
    #[must_use]
    pub fn get(&self, task_id: &str) -> Option<&ProofObligation> {
        self.by_task.get(task_id)
    }

    /// True when the manifest contains no obligations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_task.is_empty()
    }

    /// Number of obligations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_task.len()
    }

    /// Obligations in declaration (JSONL) order.
    pub fn iter_ordered(&self) -> impl Iterator<Item = &ProofObligation> {
        self.ordered_task_ids
            .iter()
            .filter_map(|tid| self.by_task.get(tid))
    }
}

/// Errors produced while loading a manifest.
#[derive(Debug, Error)]
pub enum ObligationManifestError {
    /// File system error.
    #[error("manifest io ({path}): {source}")]
    Io {
        /// Path the loader tried to read (or synthetic line marker).
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// Parse/validation error on a specific line.
    #[error("manifest line {line}: {source}")]
    Line {
        /// 1-based line number.
        line: usize,
        /// Underlying error.
        #[source]
        source: ProofObligationLoadError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obl(task_id: &str) -> ProofObligation {
        ProofObligation {
            schema_version: 1,
            obligation_id: format!("obl.{task_id}"),
            task_id: task_id.to_owned(),
            property_name: "p".into(),
            property_type: crate::spec::PropertyType::NoPanicOrInvalidState,
            target_files: vec!["src/lib.rs".into()],
            informal_statement: "s".into(),
            formal_statement_ref: "r".into(),
            proof_checker: crate::spec::ObligationProofChecker {
                command: "lake".into(),
                args: vec!["env".into(), "lean".into()],
            },
            pass_criterion: "L4_OBLIGATION_MET".into(),
            difficulty: crate::spec::Difficulty {
                reviewer_hours: 1.0,
            },
            selection_rationale: crate::spec::SelectionRationale {
                one_or_two_sentence_property: true,
                local_scope: true,
                matters_to_issue: true,
                strictly_stronger_than_tests: true,
                bounded_effort: true,
            },
            witness_inputs: Vec::new(),
            expected_touched_symbols: Vec::new(),
        }
    }

    #[test]
    fn load_jsonl_skips_blank_and_comment_lines() {
        let mut body = String::new();
        body.push_str("# header comment\n");
        body.push_str(&serde_json::to_string(&obl("task-a")).unwrap());
        body.push('\n');
        body.push('\n');
        body.push_str("   # indented comment\n");
        body.push_str(&serde_json::to_string(&obl("task-b")).unwrap());
        body.push('\n');

        let m = ObligationManifest::from_reader(body.as_bytes()).unwrap();
        assert_eq!(m.len(), 2);
        assert!(m.get("task-a").is_some());
        assert!(m.get("task-b").is_some());
    }

    #[test]
    fn rejects_duplicate_task_id() {
        let mut body = String::new();
        body.push_str(&serde_json::to_string(&obl("task-a")).unwrap());
        body.push('\n');
        body.push_str(&serde_json::to_string(&obl("task-a")).unwrap());
        body.push('\n');

        let err = ObligationManifest::from_reader(body.as_bytes()).unwrap_err();
        assert!(matches!(err, ObligationManifestError::Line { .. }));
    }

    #[test]
    fn iter_ordered_preserves_declaration_order() {
        let mut body = String::new();
        body.push_str(&serde_json::to_string(&obl("task-z")).unwrap());
        body.push('\n');
        body.push_str(&serde_json::to_string(&obl("task-a")).unwrap());
        body.push('\n');
        let m = ObligationManifest::from_reader(body.as_bytes()).unwrap();
        let tids: Vec<_> = m.iter_ordered().map(|o| o.task_id.clone()).collect();
        assert_eq!(tids, vec!["task-z", "task-a"]);
    }
}
