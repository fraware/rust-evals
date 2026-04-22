//! `proof_results.json` bundle artifact.
//!
//! Shape:
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "evaluator_version": { ... },
//!   "obligation": { ... ProofObligation ... } | null,
//!   "outcome": { ... LeanCheckOutcome ... } | null,
//!   "status": "valid|invalid|not_applicable",
//!   "code": "L4_OBLIGATION_MET|...",
//!   "message": "...",
//!   "duration_ms": 42,
//!   "started_at": "...",
//!   "finished_at": "..."
//! }
//! ```
//!
//! The aggregate `EvaluationResult` written into the bundle's
//! `proof_results` file (see [`crate::extension::L4_RESULT_FILE`])
//! mirrors `status`, `code` and the timing fields. Keeping the full
//! obligation and checker payload here lets reviewers reproduce the
//! verdict without reading the Lean sources directly.

use chrono::{DateTime, Utc};
use eval_ladder_core::EvaluatorVersion;
use serde::Serialize;

use crate::checker::{LeanCheckOutcome, LeanStatus};
use crate::spec::ProofObligation;

/// Schema version for the artifact.
pub const PROOF_REPORT_SCHEMA_VERSION: u32 = 1;

/// Full L4 bundle artifact.
#[derive(Debug, Clone, Serialize)]
pub struct ProofReport {
    /// Schema version for this artifact.
    pub schema_version: u32,
    /// Evaluator version recording engine at evaluation time.
    pub evaluator_version: EvaluatorVersion,
    /// The obligation evaluated, or `None` when the task had no
    /// obligation in the manifest (`NotApplicable`).
    pub obligation: Option<ProofObligation>,
    /// The structured checker outcome, or `None` when the checker
    /// was not invoked (`NotApplicable` without an obligation, or a
    /// hard harness error).
    pub outcome: Option<LeanCheckOutcome>,
    /// Aggregate status reported in the `EvaluationResult`.
    pub status: LeanStatus,
    /// Stable uppercase code reported in the `EvaluationResult`.
    pub code: String,
    /// Human-readable summary.
    pub message: String,
    /// Checker duration in milliseconds (integer for reviewer
    /// ergonomics; raw timestamps are preserved for reproduction).
    pub duration_ms: u128,
    /// Clock reading at the moment the extension entered its `run`.
    pub started_at: DateTime<Utc>,
    /// Clock reading at the moment the extension produced its
    /// verdict.
    pub finished_at: DateTime<Utc>,
}
