//! Lightweight execution memory.
//!
//! Stores key outputs, decisions, and step results for a single run.
//! Injected into prompts to give the agent continuity across steps.
//! Never persisted to disk — scoped strictly to one pipeline run.

use mauns_core::types::SkillUsage;

/// A single entry in the execution memory.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub kind:    MemoryKind,
    pub content: String,
}

/// The kind of memory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryKind {
    /// An important decision made by the agent.
    Decision,
    /// A key output from a skill or step.
    KeyOutput,
    /// A note the agent explicitly flagged as important.
    AgentNote,
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryKind::Decision  => write!(f, "decision"),
            MemoryKind::KeyOutput => write!(f, "key-output"),
            MemoryKind::AgentNote => write!(f, "note"),
        }
    }
}

/// Run-scoped execution memory.
///
/// Holds up to `capacity` entries; oldest entries are dropped when full.
/// Provides a compact string representation for prompt injection.
#[derive(Debug, Default)]
pub struct ExecutionMemory {
    entries:  Vec<MemoryEntry>,
    capacity: usize,
    /// Snapshot of every skill that ran successfully (for deduplication).
    skill_log: Vec<String>,
}

impl ExecutionMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries:   Vec::new(),
            capacity:  capacity.max(1),
            skill_log: Vec::new(),
        }
    }

    /// Record a decision.
    pub fn remember_decision(&mut self, content: impl Into<String>) {
        self.push(MemoryEntry { kind: MemoryKind::Decision, content: content.into() });
    }

    /// Record a key output.
    pub fn remember_output(&mut self, content: impl Into<String>) {
        self.push(MemoryEntry { kind: MemoryKind::KeyOutput, content: content.into() });
    }

    /// Record an agent note.
    pub fn remember_note(&mut self, content: impl Into<String>) {
        self.push(MemoryEntry { kind: MemoryKind::AgentNote, content: content.into() });
    }

    /// Track a successful skill invocation by name.
    pub fn track_skill(&mut self, usage: &SkillUsage) {
        if usage.success {
            self.skill_log.push(usage.skill_name.clone());
        }
    }

    /// Return a compact string suitable for prompt injection.
    /// Returns an empty string when there are no entries.
    pub fn render(&self) -> String {
        if self.entries.is_empty() && self.skill_log.is_empty() {
            return String::new();
        }

        let mut parts: Vec<String> = Vec::new();

        if !self.entries.is_empty() {
            let block: String = self
                .entries
                .iter()
                .map(|e| format!("  [{}] {}", e.kind, e.content))
                .collect::<Vec<_>>()
                .join("\n");
            parts.push(format!("Execution memory:\n{block}"));
        }

        if !self.skill_log.is_empty() {
            let unique: Vec<&String> = {
                let mut seen = std::collections::HashSet::new();
                self.skill_log.iter().filter(|s| seen.insert(s.as_str())).collect()
            };
            parts.push(format!(
                "Skills used so far: {}",
                unique.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }

        format!("\n\n{}", parts.join("\n\n"))
    }

    fn push(&mut self, entry: MemoryEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_empty_as_empty_string() {
        let m = ExecutionMemory::new(10);
        assert!(m.render().is_empty());
    }

    #[test]
    fn renders_entries() {
        let mut m = ExecutionMemory::new(10);
        m.remember_decision("Use JSON output format");
        let r = m.render();
        assert!(r.contains("decision"));
        assert!(r.contains("Use JSON output format"));
    }

    #[test]
    fn evicts_oldest_on_overflow() {
        let mut m = ExecutionMemory::new(2);
        m.remember_note("a");
        m.remember_note("b");
        m.remember_note("c"); // evicts "a"
        assert_eq!(m.entries.len(), 2);
        assert_eq!(m.entries[0].content, "b");
    }

    #[test]
    fn tracks_skills_deduplicates() {
        let mut m = ExecutionMemory::new(10);
        let usage = SkillUsage {
            skill_name: "file_read".to_string(),
            timestamp:  chrono::Utc::now(),
            success:    true,
            message:    String::new(),
        };
        m.track_skill(&usage);
        m.track_skill(&usage);
        let r = m.render();
        // "file_read" should appear only once in the rendered output
        assert_eq!(r.matches("file_read").count(), 1);
    }
}
