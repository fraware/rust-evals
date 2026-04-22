//! Shared execution helpers for the L2 validator families.
//!
//! All three implemented families (augmented tests, targeted regression,
//! differential behaviour) share the same "run N commands in a fresh
//! patched workspace" skeleton. Isolating it here keeps each validator
//! body short and deterministic.
//!
//! Items are `pub(crate)` because this is a private module used only by
//! the validator families; `redundant_pub_crate` is explicitly allowed
//! to resolve the conflict with `unreachable_pub`.

#![allow(clippy::redundant_pub_crate)]

use std::path::{Path, PathBuf};

use eval_ladder_runner::{
    apply_patch, prepare_workspace, ContainerEngine, EnvVar, ExecOutcome, ExecSpec,
    PatchApplyOutcome, ResourceLimits,
};

use crate::spec::CommandSpec;
use crate::validator::ValidatorError;

/// Prepare a fresh workspace with optional patch applied.
///
/// `workspace_dir` must not yet exist; this function creates it as a
/// direct child of `staging_root`.
pub(crate) fn prepare_patched_workspace(
    template: &Path,
    staging_root: &Path,
    subdir: &str,
    patch_bytes: Option<&[u8]>,
) -> Result<(PathBuf, PatchApplyOutcome), ValidatorError> {
    let dest = staging_root.join(subdir);
    prepare_workspace(template, &dest)?;
    let outcome = match patch_bytes {
        Some(bytes) => apply_patch(&dest, bytes)?,
        None => PatchApplyOutcome::Noop,
    };
    Ok((dest, outcome))
}

/// Run a single [`CommandSpec`] against a prepared workspace.
pub(crate) fn run_command(
    engine: &dyn ContainerEngine,
    image_ref: &str,
    workspace: &Path,
    base_env: &[EnvVar],
    limits: &ResourceLimits,
    cmd: &CommandSpec,
) -> Result<ExecOutcome, ValidatorError> {
    if cmd.command.is_empty() {
        return Err(ValidatorError::InvalidInput(format!(
            "command spec {} has an empty command vector",
            cmd.id
        )));
    }

    let workdir = cmd
        .workdir
        .as_ref()
        .map_or_else(|| workspace.to_path_buf(), |rel| workspace.join(rel));

    let mut env: Vec<EnvVar> = base_env.to_vec();
    env.extend(cmd.env.iter().cloned());

    let spec = ExecSpec::new(
        image_ref.to_owned(),
        &workdir,
        cmd.command.clone(),
        env,
        limits.clone(),
    );

    Ok(engine.exec(&spec)?)
}

/// Compare the observed outcome against the command's expectation.
#[must_use]
pub(crate) fn outcome_matches_expectation(outcome: &ExecOutcome, cmd: &CommandSpec) -> bool {
    if outcome.timed_out {
        return false;
    }
    let expected = cmd.expected_exit_code.unwrap_or(0);
    outcome.exit_code == Some(expected)
}

/// Truncate a stderr sample to `max_bytes`, breaking on UTF-8
/// boundaries, and trim trailing whitespace so bundles diff cleanly.
#[must_use]
pub(crate) fn summarize_stderr(outcome: &ExecOutcome, max_bytes: usize) -> Option<String> {
    if outcome.stderr.is_empty() {
        return None;
    }
    let sample = if outcome.stderr.len() <= max_bytes {
        outcome.stderr.clone()
    } else {
        let mut end = max_bytes;
        while end > 0 && !outcome.stderr.is_char_boundary(end) {
            end -= 1;
        }
        let mut s = outcome.stderr[..end].to_owned();
        s.push_str("...<truncated>");
        s
    };
    Some(sample.trim_end().to_owned())
}
