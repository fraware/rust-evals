//! Per-run isolated workspace construction.
//!
//! The pipeline never mutates a shared template directory. Instead it
//! materializes an isolated copy under the run's staging root, applies
//! the patch there, and executes the scorer there. This module performs
//! the copy.
//!
//! # Determinism
//!
//! The copy preserves file contents verbatim and skips symlinks and
//! special files. Timestamps are *not* preserved (POSIX mtime is not
//! portable to Windows), so callers who need byte-identical repeat copies
//! must not observe mtimes.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use walkdir::WalkDir;

/// Errors produced by [`prepare_workspace`].
#[derive(Debug, Error)]
pub enum WorkspaceError {
    /// The template directory does not exist on disk.
    #[error("template workspace does not exist: {0}")]
    MissingTemplate(PathBuf),
    /// The destination directory is non-empty.
    #[error("destination must be empty or absent: {0}")]
    DestinationNotEmpty(PathBuf),
    /// An unexpected symlink was encountered.
    #[error("symlinks are not supported in workspace templates: {0}")]
    UnsupportedSymlink(PathBuf),
    /// Filesystem-level error.
    #[error("workspace io: {0}")]
    Io(#[from] io::Error),
    /// Walkdir traversal error.
    #[error("workspace walk: {0}")]
    Walk(#[from] walkdir::Error),
}

/// Recursively copy `template` into `dest`, creating `dest` if it does
/// not already exist.
///
/// `dest` must be empty (or absent); this prevents accidental overwrite
/// of another run's workspace. Symlinks in the template are rejected: a
/// reproducible evidence bundle requires a fully materialized file tree.
pub fn prepare_workspace(template: &Path, dest: &Path) -> Result<(), WorkspaceError> {
    if !template.exists() {
        return Err(WorkspaceError::MissingTemplate(template.to_path_buf()));
    }

    if dest.exists() {
        let mut iter = fs::read_dir(dest)?;
        if iter.next().is_some() {
            return Err(WorkspaceError::DestinationNotEmpty(dest.to_path_buf()));
        }
    } else {
        fs::create_dir_all(dest)?;
    }

    for entry in WalkDir::new(template).follow_links(false) {
        let entry = entry?;
        let source_path = entry.path();
        let rel = source_path
            .strip_prefix(template)
            .expect("WalkDir yields descendants of its root");
        if rel.as_os_str().is_empty() {
            continue;
        }
        let dest_path = dest.join(rel);
        let ft = entry.file_type();
        if ft.is_symlink() {
            return Err(WorkspaceError::UnsupportedSymlink(
                source_path.to_path_buf(),
            ));
        }
        if ft.is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else if ft.is_file() {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source_path, &dest_path)?;
        }
        // Other file kinds (sockets, devices) are silently skipped; they
        // have no place in a benchmark template.
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn copies_flat_template() {
        let src = tempdir().unwrap();
        fs::write(src.path().join("a.txt"), "A").unwrap();
        fs::write(src.path().join("b.txt"), "B").unwrap();
        let dst = tempdir().unwrap();
        // Point dest at a fresh subdir so it is absent.
        let dst = dst.path().join("ws");
        prepare_workspace(src.path(), &dst).unwrap();
        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "A");
        assert_eq!(fs::read_to_string(dst.join("b.txt")).unwrap(), "B");
    }

    #[test]
    fn copies_nested_template() {
        let src = tempdir().unwrap();
        fs::create_dir_all(src.path().join("nested/dir")).unwrap();
        fs::write(src.path().join("nested/dir/x.txt"), "X").unwrap();
        let dst = tempdir().unwrap();
        let dst = dst.path().join("ws");
        prepare_workspace(src.path(), &dst).unwrap();
        assert_eq!(
            fs::read_to_string(dst.join("nested/dir/x.txt")).unwrap(),
            "X"
        );
    }

    #[test]
    fn rejects_nonempty_destination() {
        let src = tempdir().unwrap();
        fs::write(src.path().join("a.txt"), "A").unwrap();
        let dst = tempdir().unwrap();
        fs::write(dst.path().join("preexisting.txt"), "stale").unwrap();
        let err = prepare_workspace(src.path(), dst.path()).unwrap_err();
        assert!(matches!(err, WorkspaceError::DestinationNotEmpty(_)));
    }

    #[test]
    fn rejects_missing_template() {
        let missing = PathBuf::from("/definitely/not/here/for/this/test");
        let dst = tempdir().unwrap();
        let err = prepare_workspace(&missing, dst.path()).unwrap_err();
        assert!(matches!(err, WorkspaceError::MissingTemplate(_)));
    }
}
