//! Session state — holds all mutable runtime state for the agent session.

use mauns_config::schema::MaunsConfig;
use mauns_core::types::{Plan, TaskReport};

/// The current operational mode of the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMode {
    /// Normal interactive mode — awaiting a task or slash command.
    Interactive,
    /// A task is currently being executed.
    Running,
    /// Dry-run mode: all file writes are simulated.
    DryRun,
    /// Vibe mode: faster, fewer confirmations.
    Vibe,
}

impl std::fmt::Display for SessionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionMode::Interactive => write!(f, "interactive"),
            SessionMode::Running     => write!(f, "running"),
            SessionMode::DryRun     => write!(f, "dry-run"),
            SessionMode::Vibe       => write!(f, "vibe"),
        }
    }
}

/// Full mutable state for one agent session.
pub struct SessionState {
    pub config:        MaunsConfig,
    pub mode:          SessionMode,
    /// Current provider name (may be changed by /models).
    pub provider:      String,
    /// Current model name (empty = provider default).
    pub model:         String,
    /// Whether deterministic sampling is active.
    pub deterministic: bool,
    /// The last plan produced by the planner, if any.
    pub last_plan:     Option<Plan>,
    /// History of task descriptions run in this session.
    pub task_history:  Vec<String>,
    /// Full reports from completed runs in this session.
    pub reports:       Vec<TaskReport>,
    /// Tracks how many tasks have been run.
    pub run_count:     usize,
}

impl SessionState {
    pub fn new(config: MaunsConfig) -> Self {
        let provider = config.provider.clone();
        let model    = config.model.clone();
        Self {
            config,
            mode: SessionMode::Interactive,
            provider,
            model,
            deterministic: false,
            last_plan:     None,
            task_history:  Vec::new(),
            reports:       Vec::new(),
            run_count:     0,
        }
    }

    pub fn is_dry_run(&self) -> bool {
        matches!(self.mode, SessionMode::DryRun) || self.config.safety.dry_run
    }

    pub fn is_vibe(&self) -> bool {
        matches!(self.mode, SessionMode::Vibe)
    }

    pub fn effective_model(&self) -> Option<&str> {
        if self.model.is_empty() { None } else { Some(&self.model) }
    }

    pub fn set_mode(&mut self, mode: SessionMode) {
        self.mode = mode;
    }
}
