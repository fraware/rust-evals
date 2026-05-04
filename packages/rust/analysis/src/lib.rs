//! # eval-ladder-analysis
//!
//! Pure, deterministic analysis over evaluation results. No I/O beyond the
//! input/output seams documented on each function.
//!
//! The analysis crate is intentionally stateless: callers load evaluation
//! results from evidence bundles and pass them as an [`AnalysisInput`]; the
//! functions here return structured metric objects that serialize to CSV and
//! JSON.
//!
//! Paper-ready outputs:
//!
//! - [`score_descent::score_descent`] - pass rate by level, per benchmark,
//!   per agent.
//! - [`score_descent::conditional_reversal`] -
//!   `P(fail at L_{k+1} | pass at L_k)`.
//! - [`rank_stability::rank_stability`] - pairwise rank correlation between
//!   agent leaderboards at each level (Kendall tau).
//! - [`taxonomy::taxonomy_counts`] - counts of every stable failure and
//!   policy-violation code.
//! - [`static_vs_live::static_vs_live`] - per-agent comparison of static
//!   (SWE-bench Verified) vs live (SWE-bench-Live) pass rates at each
//!   level; the headline paper table for the "overstatement" claim.
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod bundle_loader;
pub mod csv;
pub mod input;
pub mod paper_export;
pub mod rank_stability;
pub mod score_descent;
pub mod static_vs_live;
pub mod taxonomy;

pub use bundle_loader::{
    load_bundle_dir, BundleLoadError, LoadOptions, TaskCategoryLookup, CANDIDATE_RESOLUTION_FILE,
    LEVEL_RESULT_FILES,
};
pub use input::{
    project_analysis_mode, AnalysisInput, AnalysisInputRow, AnalysisMode,
    CUMULATIVE_PREREQUISITE_NOT_MET,
};
pub use paper_export::{write_paper_exports, PaperExport, PaperExportManifest, PaperExportSet};
