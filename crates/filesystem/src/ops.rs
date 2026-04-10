//! High-level, safe filesystem operations.
//!
//! All methods go through [`PathGuard`] for validation.
//! All write/delete operations generate a diff before any mutation and record
//! the change in the embedded [`ChangeTracker`].
//! In dry-run mode, writes and deletes are logged and diffed but never applied.

use std::path::Path;

use mauns_core::{
    error::{MaunsError, Result},
    types::FileOperation,
};
use tracing::{info, warn};

use crate::{
    diff::{diff_for_create, diff_for_delete, unified_diff},
    guard::PathGuard,
    tracker::ChangeTracker,
};

/// Safe filesystem facade used by the agent pipeline.
///
/// Construct once per pipeline run and pass as a shared reference.
#[derive(Debug)]
pub struct Filesystem {
    guard: PathGuard,
    tracker: ChangeTracker,
    dry_run: bool,
}

impl Filesystem {
    /// Create a new `Filesystem` rooted at the current working directory.
    pub fn new(dry_run: bool) -> Result<Self> {
        let cwd = std::env::current_dir().map_err(|e| {
            MaunsError::Filesystem(format!("cannot determine current directory: {e}"))
        })?;
        Ok(Self {
            guard: PathGuard::new(cwd)?,
            tracker: ChangeTracker::new(),
            dry_run,
        })
    }

    /// Create a new `Filesystem` rooted at an explicit path.
    pub fn with_root(root: impl AsRef<Path>, dry_run: bool) -> Result<Self> {
        Ok(Self {
            guard: PathGuard::new(root)?,
            tracker: ChangeTracker::new(),
            dry_run,
        })
    }

    // ------------------------------------------------------------------
    // Read operations (always allowed; no tracking needed)
    // ------------------------------------------------------------------

    /// Read the full text content of a file.
    pub fn read_file(&self, path: impl AsRef<Path>) -> Result<String> {
        let safe = self.guard.validate(path)?;
        std::fs::read_to_string(safe.as_path()).map_err(|e| {
            MaunsError::Filesystem(format!("read_file '{}' failed: {e}", safe))
        })
    }

    /// List directory entries as relative-path strings.
    pub fn list_dir(&self, path: impl AsRef<Path>) -> Result<Vec<String>> {
        let safe = self.guard.validate(path)?;
        let read = std::fs::read_dir(safe.as_path()).map_err(|e| {
            MaunsError::Filesystem(format!("list_dir '{}' failed: {e}", safe))
        })?;

        let mut entries = Vec::new();
        for entry in read {
            let entry = entry.map_err(|e| {
                MaunsError::Filesystem(format!("list_dir entry error: {e}"))
            })?;
            entries.push(entry.file_name().to_string_lossy().into_owned());
        }
        entries.sort();
        Ok(entries)
    }

    // ------------------------------------------------------------------
    // Write operations (gated by dry_run; always diff + track)
    // ------------------------------------------------------------------

    /// Write `content` to `path`, creating the file if it does not exist.
    ///
    /// Always generates and records a diff.
    /// In dry-run mode the file is not touched.
    pub fn write_file(
        &mut self,
        path: impl AsRef<Path>,
        content: &str,
    ) -> Result<String> {
        let safe = self.guard.validate(path)?;
        let path_str = safe.to_string();

        // Determine operation kind and generate diff.
        let (operation, diff) = if safe.as_path().exists() {
            let old = std::fs::read_to_string(safe.as_path()).map_err(|e| {
                MaunsError::Filesystem(format!(
                    "write_file: cannot read existing '{}' for diff: {e}",
                    path_str
                ))
            })?;
            let d = unified_diff(&path_str, &old, content);
            (FileOperation::Edit, d)
        } else {
            let d = diff_for_create(&path_str, content);
            (FileOperation::Create, d)
        };

        if self.dry_run {
            warn!(
                filesystem = "dry-run",
                operation = %operation,
                path = %path_str,
                "skipping write (dry-run mode)"
            );
            self.tracker
                .record_new(&path_str, operation, &diff, false);
            return Ok(diff);
        }

        // Create parent directories if needed.
        if let Some(parent) = safe.as_path().parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                MaunsError::Filesystem(format!(
                    "write_file: cannot create directories for '{}': {e}",
                    path_str
                ))
            })?;
        }

        std::fs::write(safe.as_path(), content).map_err(|e| {
            MaunsError::Filesystem(format!("write_file '{}' failed: {e}", path_str))
        })?;

        info!(filesystem = "write", path = %path_str, "file written");
        self.tracker
            .record_new(&path_str, operation, &diff, true);
        Ok(diff)
    }

    /// Delete a file.
    ///
    /// Always generates a delete-diff and records the change.
    /// In dry-run mode the file is not removed.
    pub fn delete_file(&mut self, path: impl AsRef<Path>) -> Result<String> {
        let safe = self.guard.validate(path)?;
        let path_str = safe.to_string();

        if !safe.as_path().exists() {
            return Err(MaunsError::Filesystem(format!(
                "delete_file: '{}' does not exist",
                path_str
            )));
        }

        let old = std::fs::read_to_string(safe.as_path()).map_err(|e| {
            MaunsError::Filesystem(format!(
                "delete_file: cannot read '{}' for diff: {e}",
                path_str
            ))
        })?;
        let diff = diff_for_delete(&path_str, &old);

        if self.dry_run {
            warn!(
                filesystem = "dry-run",
                operation = "delete",
                path = %path_str,
                "skipping delete (dry-run mode)"
            );
            self.tracker
                .record_new(&path_str, FileOperation::Delete, &diff, false);
            return Ok(diff);
        }

        std::fs::remove_file(safe.as_path()).map_err(|e| {
            MaunsError::Filesystem(format!("delete_file '{}' failed: {e}", path_str))
        })?;

        info!(filesystem = "delete", path = %path_str, "file deleted");
        self.tracker
            .record_new(&path_str, FileOperation::Delete, &diff, true);
        Ok(diff)
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    /// Consume the filesystem and return all recorded changes.
    pub fn into_changes(self) -> Vec<mauns_core::types::FileChange> {
        self.tracker.into_changes()
    }

    /// Borrow the change tracker.
    pub fn changes(&self) -> &[mauns_core::types::FileChange] {
        self.tracker.changes()
    }

    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}
