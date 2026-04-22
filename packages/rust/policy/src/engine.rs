//! Policy evaluation engine.
//!
//! Pure function: given a [`RunContext`] (what actually happened in the run)
//! and a [`Policy`] (what should have happened), emit the ordered list of
//! findings. No I/O; the runner assembles the context.

use std::collections::HashSet;

use eval_ladder_core::PolicyViolation;
use eval_ladder_traces::EventType;
use globset::{Glob, GlobSet, GlobSetBuilder};
use thiserror::Error;

use crate::report::{PolicyFinding, PolicyReport};
use crate::spec::{NetworkMode, Policy};

/// Runtime context assembled from a completed run. Evaluated against a [`Policy`].
#[derive(Debug, Clone, Default)]
pub struct RunContext {
    /// Commands the run actually executed (ordered, may contain duplicates).
    pub executed_commands: Vec<String>,
    /// POSIX paths touched by the patch, relative to the repo root.
    pub modified_files: Vec<String>,
    /// Whether the run made any outbound network request.
    pub network_accessed: bool,
    /// Whether the run declared and used a reproducible seed.
    pub reproducible_seed_declared: bool,
    /// Trace event types actually emitted by the run.
    pub trace_events_emitted: HashSet<EventType>,
    /// Whether any generated tests were present in the bundle.
    pub generated_tests_present: bool,
    /// Whether a dependency lockfile was edited.
    pub dependency_lockfile_edited: bool,
    /// Whether the harness observed non-determinism across the rerun seeds.
    pub nondeterminism_observed: bool,
}

/// Errors produced by the policy engine.
#[derive(Debug, Error)]
pub enum PolicyEngineError {
    /// A glob pattern in the policy did not compile.
    #[error("invalid glob pattern {pattern:?}: {source}")]
    InvalidGlob {
        /// Offending pattern.
        pattern: String,
        /// Underlying `globset` error.
        source: globset::Error,
    },
}

/// Evaluate a policy against a run context and return a structured report.
pub fn evaluate(policy: &Policy, ctx: &RunContext) -> Result<PolicyReport, PolicyEngineError> {
    let mut findings: Vec<PolicyFinding> = Vec::new();

    // 1. Network access.
    match policy.network_mode {
        NetworkMode::Disabled | NetworkMode::None => {
            if ctx.network_accessed {
                findings.push(PolicyFinding {
                    code: PolicyViolation::PV_NET_ACCESS,
                    message: "network access detected while network_mode is disabled".to_owned(),
                    evidence_path: None,
                });
            }
        }
        NetworkMode::HostAllowlist => {
            // Host-level allow-list checking is performed by the runner; if it
            // reaches this layer marked as network_accessed the runner has
            // already decided the access was out-of-policy.
            if ctx.network_accessed {
                findings.push(PolicyFinding {
                    code: PolicyViolation::PV_NET_ACCESS,
                    message: "network access outside declared allow-list".to_owned(),
                    evidence_path: None,
                });
            }
        }
    }

    // 2. Forbidden commands.
    let forbidden: HashSet<&str> = policy
        .forbidden_commands
        .iter()
        .map(String::as_str)
        .collect();
    let allowed: HashSet<&str> = policy.allowed_commands.iter().map(String::as_str).collect();
    for cmd in &ctx.executed_commands {
        let head = cmd.split_whitespace().next().unwrap_or(cmd);
        if forbidden.contains(head) {
            findings.push(PolicyFinding {
                code: PolicyViolation::PV_FORBIDDEN_CMD,
                message: format!("forbidden command invoked: {head}"),
                evidence_path: None,
            });
        } else if !allowed.is_empty() && !allowed.contains(head) {
            findings.push(PolicyFinding {
                code: PolicyViolation::PV_FORBIDDEN_CMD,
                message: format!("command not in allow-list: {head}"),
                evidence_path: None,
            });
        }
    }

    // 3. Edit scope.
    let allowed_globs = compile_globset(&policy.allowed_edit_globs)?;
    let forbidden_globs = compile_globset(&policy.forbidden_edit_globs)?;
    for path in &ctx.modified_files {
        if forbidden_globs.is_match(path) {
            findings.push(PolicyFinding {
                code: PolicyViolation::PV_EDIT_SCOPE,
                message: format!("edit of forbidden path: {path}"),
                evidence_path: None,
            });
            continue;
        }
        if !policy.allowed_edit_globs.is_empty() && !allowed_globs.is_match(path) {
            findings.push(PolicyFinding {
                code: PolicyViolation::PV_EDIT_SCOPE,
                message: format!("edit of path outside allow-list: {path}"),
                evidence_path: None,
            });
        }
    }

    // 4. File-count threshold.
    if let Some(max) = policy.max_modified_files {
        if ctx.modified_files.len() as u64 > u64::from(max) {
            findings.push(PolicyFinding {
                code: PolicyViolation::PV_FILE_COUNT_EXCEEDED,
                message: format!(
                    "patch modifies {} files; max allowed is {}",
                    ctx.modified_files.len(),
                    max
                ),
                evidence_path: None,
            });
        }
    }

    // 5. Dependency lockfile edits.
    if !policy.allow_dependency_lockfile_edits && ctx.dependency_lockfile_edited {
        findings.push(PolicyFinding {
            code: PolicyViolation::PV_DEPENDENCY_EDIT,
            message: "dependency lockfile edited while policy forbids it".to_owned(),
            evidence_path: None,
        });
    }

    // 6. Generated tests.
    if !policy.allow_generated_tests && ctx.generated_tests_present {
        findings.push(PolicyFinding {
            code: PolicyViolation::PV_GENERATED_TEST_DISALLOWED,
            message: "generated tests present while policy forbids them".to_owned(),
            evidence_path: None,
        });
    }

    // 7. Environment purity / determinism.
    if policy.requires_reproducible_seed && !ctx.reproducible_seed_declared {
        findings.push(PolicyFinding {
            code: PolicyViolation::PV_ENV_NONDETERMINISTIC,
            message: "policy requires a reproducible seed but none was declared".to_owned(),
            evidence_path: None,
        });
    }
    if ctx.nondeterminism_observed {
        findings.push(PolicyFinding {
            code: PolicyViolation::PV_ENV_NONDETERMINISTIC,
            message: "rerun disagreement observed; run is non-deterministic".to_owned(),
            evidence_path: None,
        });
    }

    // 8. Trace completeness.
    for required in &policy.required_trace_events {
        if !ctx.trace_events_emitted.contains(required) {
            findings.push(PolicyFinding {
                code: PolicyViolation::PV_TRACE_INCOMPLETE,
                message: format!("required trace event missing: {required:?}"),
                evidence_path: None,
            });
        }
    }

    Ok(PolicyReport::new(&policy.name, findings))
}

fn compile_globset(patterns: &[String]) -> Result<GlobSet, PolicyEngineError> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        let glob = Glob::new(p).map_err(|source| PolicyEngineError::InvalidGlob {
            pattern: p.clone(),
            source,
        })?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|source| PolicyEngineError::InvalidGlob {
            pattern: "<globset>".to_owned(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_policy() -> Policy {
        Policy {
            name: "t".into(),
            requires_reproducible_seed: true,
            max_modified_files: Some(2),
            allow_generated_tests: false,
            allow_dependency_lockfile_edits: false,
            network_mode: NetworkMode::Disabled,
            allowed_commands: vec!["pytest".into()],
            forbidden_commands: vec!["curl".into()],
            allowed_edit_globs: vec!["src/**".into()],
            forbidden_edit_globs: vec!["secrets/**".into()],
            required_trace_events: vec![EventType::RunStarted, EventType::RunFinished],
        }
    }

    #[test]
    fn clean_context_yields_no_findings() {
        let policy = base_policy();
        let ctx = RunContext {
            executed_commands: vec!["pytest -q".into()],
            modified_files: vec!["src/foo.py".into()],
            network_accessed: false,
            reproducible_seed_declared: true,
            trace_events_emitted: [EventType::RunStarted, EventType::RunFinished]
                .iter()
                .copied()
                .collect(),
            generated_tests_present: false,
            dependency_lockfile_edited: false,
            nondeterminism_observed: false,
        };
        let report = evaluate(&policy, &ctx).unwrap();
        assert!(
            report.conformant,
            "unexpected findings: {:?}",
            report.findings
        );
    }

    #[test]
    fn detects_network_forbidden_command_edit_scope_and_filecount() {
        let policy = base_policy();
        let ctx = RunContext {
            executed_commands: vec!["curl x".into(), "pytest".into()],
            modified_files: vec![
                "src/a.py".into(),
                "src/b.py".into(),
                "secrets/leak.env".into(),
            ],
            network_accessed: true,
            reproducible_seed_declared: false,
            trace_events_emitted: [EventType::RunStarted].iter().copied().collect(),
            generated_tests_present: true,
            dependency_lockfile_edited: true,
            nondeterminism_observed: true,
        };
        let report = evaluate(&policy, &ctx).unwrap();
        let codes: HashSet<_> = report.findings.iter().map(|f| f.code).collect();
        assert!(codes.contains(&PolicyViolation::PV_NET_ACCESS));
        assert!(codes.contains(&PolicyViolation::PV_FORBIDDEN_CMD));
        assert!(codes.contains(&PolicyViolation::PV_EDIT_SCOPE));
        assert!(codes.contains(&PolicyViolation::PV_FILE_COUNT_EXCEEDED));
        assert!(codes.contains(&PolicyViolation::PV_GENERATED_TEST_DISALLOWED));
        assert!(codes.contains(&PolicyViolation::PV_DEPENDENCY_EDIT));
        assert!(codes.contains(&PolicyViolation::PV_ENV_NONDETERMINISTIC));
        assert!(codes.contains(&PolicyViolation::PV_TRACE_INCOMPLETE));
    }
}
