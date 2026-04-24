//! Candidate patch application.
//!
//! Milestone C supports two paths:
//!
//! 1. **Empty patch**: the patch bytes are zero-length. This is a noop and
//!    is the path exercised by the fixture pipeline. It lets the pipeline
//!    demonstrate L0/L1 determinism without bringing a diff applier into
//!    scope.
//!
//! 2. **Unified diff**: the patch bytes are forwarded to `git apply
//!    --whitespace=nowarn` launched in the workspace directory. This
//!    requires `git` on `$PATH` (true on every SWE-bench image) and the
//!    workspace to have been prepared as a git working tree. Milestone C
//!    only uses this path in manual runs; CI covers only the empty path.
//!
//! A richer in-process unified-diff applier is scheduled for Milestone D
//! once the L2 strengthening layer has concrete opinions about patch
//! normalization.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Outcome of applying a patch to a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchApplyOutcome {
    /// The patch bytes were empty; nothing was modified.
    Noop,
    /// A non-empty patch was applied successfully.
    Applied,
}

/// Errors produced by [`apply_patch`].
#[derive(Debug, Error)]
pub enum PatchApplyError {
    /// `git` is not available on `$PATH`.
    #[error("git binary not found on PATH")]
    GitNotFound,
    /// Workspace directory is missing.
    #[error("workspace directory does not exist: {0}")]
    WorkspaceMissing(PathBuf),
    /// `git apply` exited with a non-zero status.
    #[error("git apply failed (exit {exit:?}):\nstdout:\n{stdout}\nstderr:\n{stderr}")]
    GitApplyFailed {
        /// Exit code from `git apply`, if any.
        exit: Option<i32>,
        /// Captured stdout.
        stdout: String,
        /// Captured stderr.
        stderr: String,
    },
    /// Filesystem / subprocess I/O error.
    #[error("patch apply io: {0}")]
    Io(#[from] std::io::Error),
}

/// Apply `patch_bytes` to `workspace`.
///
/// - Empty or whitespace-only bytes -> [`PatchApplyOutcome::Noop`].
/// - Non-empty bytes -> invoke `git apply --whitespace=nowarn` with the
///   patch streamed on stdin. Returns [`PatchApplyOutcome::Applied`] on
///   exit code 0.
pub fn apply_patch(
    workspace: &Path,
    patch_bytes: &[u8],
) -> Result<PatchApplyOutcome, PatchApplyError> {
    if !workspace.exists() {
        return Err(PatchApplyError::WorkspaceMissing(workspace.to_path_buf()));
    }
    if patch_bytes.is_empty() || patch_bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(PatchApplyOutcome::Noop);
    }

    which::which("git").map_err(|_| PatchApplyError::GitNotFound)?;

    let mut child = Command::new("git")
        .arg("apply")
        .arg("--whitespace=nowarn")
        .current_dir(workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(patch_bytes)?;
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(PatchApplyOutcome::Applied)
    } else {
        Err(PatchApplyError::GitApplyFailed {
            exit: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn empty_patch_is_noop() {
        let dir = tempdir().unwrap();
        let r = apply_patch(dir.path(), b"").unwrap();
        assert_eq!(r, PatchApplyOutcome::Noop);
    }

    #[test]
    fn whitespace_only_patch_is_noop() {
        let dir = tempdir().unwrap();
        let r = apply_patch(dir.path(), b"\n \t\r\n").unwrap();
        assert_eq!(r, PatchApplyOutcome::Noop);
    }

    #[test]
    fn missing_workspace_is_error() {
        let err = apply_patch(Path::new("/never/exists/xyz"), b"").unwrap_err();
        assert!(matches!(err, PatchApplyError::WorkspaceMissing(_)));
    }
}
