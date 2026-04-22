//! [`LevelExtension`] implementation for L3 policy.
//!
//! Threaded into [`eval_ladder_runner::EvaluationPipeline`] via
//! `PipelineInputs::extensions` the same way L2 is. The extension:
//!
//! 1. Builds a [`RunContext`] from the live trace + patch + bundle
//!    state ([`build_run_context`]).
//! 2. Emits a `PolicyCheckStarted` trace event.
//! 3. Evaluates the policy ([`engine::evaluate`]).
//! 4. Emits one `PolicyViolationDetected` trace event per finding, in
//!    evaluation order. This keeps the bundle self-describing even if
//!    the `policy_results.json` artifact is lost.
//! 5. Writes a canonical [`PolicyReport`] to
//!    `policy_results.json` inside the bundle.
//! 6. Emits an `EvaluationResult` keyed at
//!    [`EvaluationLevel::L3PolicyConformant`]. `primary_reason` is the
//!    first finding's `PV_*` code when any finding was emitted, else
//!    `PASS`.
//!
//! # Determinism
//!
//! The extension is a deterministic function of:
//! - the policy document (the caller owns it),
//! - the candidate patch bytes,
//! - the trace events that preceded L3,
//! - the already-computed L0/L1 verdicts,
//! - the candidate's `generation_metadata.random_seed`,
//! - and the static `L3Observation`.
//!
//! Every timestamp comes from [`eval_ladder_runner::Clock::now`], so
//! reruns with a [`eval_ladder_runner::FixedClock`] produce identical
//! bundle hashes. The Milestone E acceptance test verifies this.

use std::path::Path;

use eval_ladder_core::{
    EvaluationLevel, EvaluationResult, EvaluationStatus, EvaluatorVersion, EVALUATOR_VERSION,
};
use eval_ladder_runner::{ExtensionContext, ExtensionError, LevelExtension};
use eval_ladder_traces::{EventType, TraceWriter};
use serde::Serialize;
use serde_json::json;

use crate::context_builder::{build_run_context, ContextBuildError, L3Observation};
use crate::engine::evaluate;
use crate::report::{PolicyFinding, PolicyReport};
use crate::spec::Policy;

/// Stable filename used in the evidence bundle for L3 results.
pub const L3_RESULT_FILE: &str = "policy_results.json";

/// Stable extension name.
pub const L3_EXTENSION_NAME: &str = "l3_policy";

/// L3 extension plugged into
/// [`eval_ladder_runner::EvaluationPipeline`].
///
/// Holds only references so the extension can be stack-allocated next
/// to the pipeline call and dropped deterministically. Auxiliary
/// observations are stored by value because they are cheap.
#[derive(Debug)]
pub struct L3Extension<'a> {
    policy: &'a Policy,
    observation: L3Observation,
}

impl<'a> L3Extension<'a> {
    /// Build an L3 extension over `policy`. The default observation
    /// treats the run as having no outbound network activity, which is
    /// correct for `LocalProcessEngine`. Docker-backed runners should
    /// override via [`Self::with_observation`].
    #[must_use]
    pub const fn new(policy: &'a Policy) -> Self {
        Self {
            policy,
            observation: L3Observation {
                network_accessed: false,
            },
        }
    }

    /// Override the default [`L3Observation`].
    #[must_use]
    pub const fn with_observation(mut self, observation: L3Observation) -> Self {
        self.observation = observation;
        self
    }

    /// Read-only view of the underlying policy.
    #[must_use]
    pub const fn policy(&self) -> &Policy {
        self.policy
    }
}

impl LevelExtension for L3Extension<'_> {
    fn name(&self) -> &'static str {
        L3_EXTENSION_NAME
    }

    fn level(&self) -> EvaluationLevel {
        EvaluationLevel::L3PolicyConformant
    }

    fn result_file(&self) -> &'static str {
        L3_RESULT_FILE
    }

    fn run(
        &self,
        ctx: &ExtensionContext<'_>,
        trace: &mut TraceWriter,
    ) -> Result<EvaluationResult, ExtensionError> {
        let started_at = ctx.clock.now();
        trace.append_at(
            EventType::PolicyCheckStarted,
            json!({
                "policy_name": self.policy.name,
                "required_trace_events": self.policy.required_trace_events,
                "max_modified_files": self.policy.max_modified_files,
                "network_mode": self.policy.network_mode,
            }),
            started_at,
        )?;

        let run_ctx = build_run_context(ctx, self.observation).map_err(wrap_build_err)?;

        let report = evaluate(self.policy, &run_ctx).map_err(|e| {
            ExtensionError::inner(
                L3_EXTENSION_NAME,
                Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()),
            )
        })?;

        // One trace event per finding so violations are auditable from
        // the trace alone.
        for finding in &report.findings {
            trace.append_at(
                EventType::PolicyViolationDetected,
                json!({
                    "policy_name": self.policy.name,
                    "code": finding.code,
                    "message": finding.message,
                    "evidence_path": finding.evidence_path,
                }),
                ctx.clock.now(),
            )?;
        }

        // Serialize the full report into the bundle before writing the
        // aggregate EvaluationResult. Keeping the two artifacts separate
        // (policy_results.json and the extension's result file) lets
        // downstream analysis consume either shape without reparsing.
        write_canonical_json(
            &ctx.bundle_dir.join(L3_RESULT_FILE),
            &PolicyResultsArtifact {
                schema_version: 1,
                evaluator_version: EVALUATOR_VERSION,
                report: report.clone(),
                run_context_summary: summarize_ctx(&run_ctx),
            },
        )?;

        let finished_at = ctx.clock.now();

        let (status, primary_reason, secondary_reasons) = aggregate(&report.findings);

        let metrics = json!({
            "policy_name": self.policy.name,
            "conformant": report.conformant,
            "findings": report.findings.len(),
            "findings_by_code": findings_histogram(&report.findings),
            "modified_files": run_ctx.modified_files.len(),
            "executed_commands": run_ctx.executed_commands.len(),
        });

        let mut result = EvaluationResult::new(
            ctx.candidate_id,
            ctx.task_id.clone(),
            EvaluationLevel::L3PolicyConformant,
            status,
            primary_reason,
            started_at,
            finished_at,
        );
        result.secondary_reasons = secondary_reasons;
        result.metrics = metrics;
        Ok(result)
    }
}

fn aggregate(findings: &[PolicyFinding]) -> (EvaluationStatus, String, Vec<String>) {
    if findings.is_empty() {
        return (EvaluationStatus::Pass, "PASS".to_owned(), Vec::new());
    }
    let primary = findings[0].code.as_str().to_owned();
    let secondary: Vec<String> = findings
        .iter()
        .skip(1)
        .map(|f| f.code.as_str().to_owned())
        .collect();
    (EvaluationStatus::Fail, primary, secondary)
}

fn findings_histogram(findings: &[PolicyFinding]) -> serde_json::Value {
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    for f in findings {
        *counts.entry(f.code.as_str()).or_insert(0) += 1;
    }
    serde_json::to_value(counts).unwrap_or_else(|_| json!({}))
}

fn summarize_ctx(rc: &crate::engine::RunContext) -> serde_json::Value {
    json!({
        "executed_commands": rc.executed_commands,
        "modified_files": rc.modified_files,
        "network_accessed": rc.network_accessed,
        "reproducible_seed_declared": rc.reproducible_seed_declared,
        "trace_events_emitted": rc
            .trace_events_emitted
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<std::collections::BTreeSet<_>>(),
        "generated_tests_present": rc.generated_tests_present,
        "dependency_lockfile_edited": rc.dependency_lockfile_edited,
        "nondeterminism_observed": rc.nondeterminism_observed,
    })
}

#[derive(Serialize)]
struct PolicyResultsArtifact {
    schema_version: u32,
    evaluator_version: EvaluatorVersion,
    report: PolicyReport,
    run_context_summary: serde_json::Value,
}

fn wrap_build_err(e: ContextBuildError) -> ExtensionError {
    ExtensionError::inner(
        L3_EXTENSION_NAME,
        Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()),
    )
}

fn write_canonical_json<T: Serialize>(path: &Path, value: &T) -> Result<(), ExtensionError> {
    let mut bytes = eval_ladder_core::canonical_json(value).map_err(|e| {
        ExtensionError::inner(
            L3_EXTENSION_NAME,
            Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()),
        )
    })?;
    bytes.push(b'\n');
    std::fs::write(path, bytes)?;
    Ok(())
}
