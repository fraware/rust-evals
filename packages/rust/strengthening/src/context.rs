//! Validation context passed to every L2 sub-validator.
//!
//! The context is a thin borrow-only wrapper around the runner's
//! [`eval_ladder_runner::ExtensionContext`] plus the task-level
//! [`crate::spec::StrengtheningSpec`]. Holding the two separately keeps
//! the runner crate unaware of L2 specifics; the strengthening crate
//! knows how to compose them.

use std::path::Path;

use eval_ladder_runner::{Clock, ContainerEngine, EnvVar, ExtensionContext, ResourceLimits};

use crate::spec::StrengtheningSpec;

/// Per-invocation context handed to every [`crate::Validator`].
///
/// Two lifetimes are carried:
/// - `'ctx` for the underlying [`ExtensionContext`] (bounded to the
///   pipeline run).
/// - `'spec` for the strengthening spec (bounded to the caller's
///   allocation, typically outlives the pipeline run).
///
/// In practice both are collapsed to the single lifetime of the
/// `LevelExtension::run` call.
pub struct ValidationContext<'a> {
    /// Unpatched workspace template.
    pub workspace_template: &'a Path,
    /// Staging root for per-validator workspaces.
    pub staging_root: &'a Path,
    /// Candidate patch bytes.
    pub patch_bytes: &'a [u8],
    /// Oracle patch bytes (only set when a differential spec and an
    /// oracle were supplied).
    pub oracle_patch_bytes: Option<&'a [u8]>,
    /// Resolved image reference, shared with L0/L1.
    pub image_ref: &'a str,
    /// Pipeline-level env.
    pub env: &'a [EnvVar],
    /// Pipeline-level resource limits.
    pub resource_limits: &'a ResourceLimits,
    /// Container engine.
    pub engine: &'a dyn ContainerEngine,
    /// Clock.
    pub clock: &'a dyn Clock,
    /// Task-level strengthening spec.
    pub spec: &'a StrengtheningSpec,
}

impl<'a> ValidationContext<'a> {
    /// Build a [`ValidationContext`] from the runner-supplied
    /// [`ExtensionContext`] plus a [`StrengtheningSpec`] and an optional
    /// oracle patch.
    #[must_use]
    pub fn from_extension(
        ext: &'a ExtensionContext<'a>,
        spec: &'a StrengtheningSpec,
        oracle_patch_bytes: Option<&'a [u8]>,
    ) -> Self {
        Self {
            workspace_template: ext.workspace_template,
            staging_root: ext.staging_root,
            patch_bytes: ext.patch_bytes,
            oracle_patch_bytes,
            image_ref: ext.image_ref,
            env: ext.env,
            resource_limits: ext.resource_limits,
            engine: ext.engine,
            clock: ext.clock,
            spec,
        }
    }
}
