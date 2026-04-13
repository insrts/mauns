//! Persistent command history backed by a file in the user's home directory.

use std::path::PathBuf;

const HISTORY_FILE: &str = ".mauns_history";
const MAX_ENTRIES:  usize = 500;

/// Loads history from disk, appends new entries, and saves back.
pub struct CommandHistory {
    entries: Vec<String>,
    path:    Option<PathBuf>,
}

impl CommandHistory {
    pub fn load() -> Self {
        let path = history_path();
        let entries = path.as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| {
                s.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self { entries, path }
    }

    pub fn push(&mut self, entry: impl Into<String>) {
        let e = entry.into();
        if e.trim().is_empty() { return; }
        // Deduplicate consecutive identical entries.
        if self.entries.last().map(|l| l == &e).unwrap_or(false) { return; }
        self.entries.push(e);
        if self.entries.len() > MAX_ENTRIES {
            self.entries.drain(0..self.entries.len() - MAX_ENTRIES);
        }
        self.save();
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Return the last `n` entries, most recent last.
    pub fn recent(&self, n: usize) -> &[String] {
        let start = self.entries.len().saturating_sub(n);
        &self.entries[start..]
    }

    fn save(&self) {
        if let Some(p) = &self.path {
            let _ = std::fs::write(p, self.entries.join("\n") + "\n");
        }
    }
}

fn history_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .map(|h| h.join(HISTORY_FILE))
}
