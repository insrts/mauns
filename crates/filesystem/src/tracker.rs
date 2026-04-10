//! Change tracking for all filesystem operations in a single run.

use mauns_core::types::{FileChange, FileOperation};

/// Records every filesystem operation attempted during a pipeline run.
/// The tracker is cheap to clone (internal state behind an Arc<Mutex<...>>
/// would be used in a concurrent scenario; for the current single-threaded
/// pipeline an owned Vec is sufficient).
#[derive(Debug, Default, Clone)]
pub struct ChangeTracker {
    changes: Vec<FileChange>,
}

impl ChangeTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a change.
    pub fn record(&mut self, change: FileChange) {
        self.changes.push(change);
    }

    /// Build a [`FileChange`] and record it immediately.
    pub fn record_new(
        &mut self,
        path: impl Into<String>,
        operation: FileOperation,
        diff: impl Into<String>,
        applied: bool,
    ) {
        self.record(FileChange {
            path: path.into(),
            operation,
            timestamp: chrono::Utc::now(),
            diff: diff.into(),
            applied,
        });
    }

    /// Return all recorded changes.
    pub fn changes(&self) -> &[FileChange] {
        &self.changes
    }

    /// Consume the tracker and return the inner vec.
    pub fn into_changes(self) -> Vec<FileChange> {
        self.changes
    }
}
