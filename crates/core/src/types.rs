use serde::{Deserialize, Serialize};

use crate::project::ProjectInfo;

// ---------------------------------------------------------------------------
// Structured plan types
// ---------------------------------------------------------------------------

/// A single step in a structured plan, with dependency awareness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// 1-based unique identifier within the plan.
    pub id: usize,
    /// Human-readable task description for this step.
    pub task: String,
    /// IDs of steps that must complete before this one begins.
    pub depends_on: Vec<usize>,
}

/// The full structured plan produced by the Planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub task: String,
    pub steps: Vec<Step>,
}

impl Plan {
    /// Return steps in topologically-sorted execution order.
    /// Steps whose dependencies are all satisfied are returned first.
    /// Falls back to id-order on cycles (safe degradation).
    pub fn execution_order(&self) -> Vec<&Step> {
        let mut remaining: Vec<&Step> = self.steps.iter().collect();
        let mut order: Vec<&Step> = Vec::new();
        let mut done_ids: std::collections::HashSet<usize> = std::collections::HashSet::new();

        let max_passes = remaining.len() + 1;
        let mut passes = 0;

        while !remaining.is_empty() && passes < max_passes {
            passes += 1;
            let mut progressed = false;

            remaining.retain(|step| {
                let ready = step.depends_on.iter().all(|dep| done_ids.contains(dep));
                if ready {
                    done_ids.insert(step.id);
                    order.push(step);
                    progressed = true;
                    false
                } else {
                    true
                }
            });

            if !progressed {
                // Cycle or unresolvable deps — append remaining in id-order.
                remaining.sort_by_key(|s| s.id);
                for s in remaining.drain(..) {
                    order.push(s);
                }
                break;
            }
        }

        order
    }
}

// ---------------------------------------------------------------------------
// Execution types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step: Step,
    pub output: String,
    pub retries_used: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput {
    pub task: String,
    pub results: Vec<StepResult>,
    pub summary: String,
    pub iterations: usize,
    pub total_retries: usize,
    pub token_usage: TokenUsage,
}

// ---------------------------------------------------------------------------
// Token tracking
// ---------------------------------------------------------------------------

/// Lightweight approximate token counter (1 token ≈ 4 chars).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens sent to the LLM across all calls.
    pub prompt_tokens: usize,
    /// Tokens received from the LLM across all calls.
    pub completion_tokens: usize,
}

pub trait ProgressReporter: Send + Sync {
    fn on_plan(&self, plan: &Plan);
    fn on_execution_start(&self);
    fn on_step_complete(&self, id: usize, task: &str);
    fn on_step_failure(&self, id: usize, task: &str, error: &str);
    fn on_result(&self, summary: &str);
}

impl TokenUsage {
    pub fn total(&self) -> usize {
        self.prompt_tokens + self.completion_tokens
    }

    /// Approximate token count from a string (1 token ≈ 4 chars).
    pub fn estimate(text: &str) -> usize {
        (text.len() / 4).max(1)
    }

    pub fn add_prompt(&mut self, text: &str) {
        self.prompt_tokens += Self::estimate(text);
    }

    pub fn add_completion(&mut self, text: &str) {
        self.completion_tokens += Self::estimate(text);
    }
}

// ---------------------------------------------------------------------------
// Verification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub passed: bool,
    pub feedback: String,
    /// When false, the pipeline may retry execution.
    pub retry_suggested: bool,
}

// ---------------------------------------------------------------------------
// Task report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TaskReport {
    pub task: String,
    pub plan: Plan,
    pub execution: ExecutionOutput,
    pub verification: VerificationReport,
    pub change_log: Vec<FileChange>,
    pub git_outcome: Option<GitOutcome>,
    pub skill_log: Vec<SkillUsage>,
    pub reasoning_summary: Option<String>,
    /// True when the run was stopped early by Ctrl+C.
    pub interrupted: bool,
}

// ---------------------------------------------------------------------------
// Filesystem types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOperation {
    Create,
    Edit,
    Delete,
}

impl std::fmt::Display for FileOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOperation::Create => write!(f, "create"),
            FileOperation::Edit => write!(f, "edit"),
            FileOperation::Delete => write!(f, "delete"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub operation: FileOperation,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub diff: String,
    pub applied: bool,
}

// ---------------------------------------------------------------------------
// Run context
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct AgentsPolicy {
    pub raw: String,
}

#[derive(Debug, Clone, Default)]
pub struct MaunsPreferences {
    pub raw: String,
}

#[derive(Debug, Clone)]
pub struct RunContext {
    pub dry_run: bool,
    pub confirm_writes: bool,
    pub deterministic: bool,
    pub vibe_mode: bool,
    pub max_iterations: usize,
    pub max_retries: usize,
    pub context_window: usize,
    /// Optional hard limit on total tokens per run (0 = no limit).
    pub max_tokens: usize,
    pub agents_policy: AgentsPolicy,
    pub mauns_prefs: MaunsPreferences,
    pub project: ProjectInfo,
}

impl Default for RunContext {
    fn default() -> Self {
        Self {
            dry_run: false,
            confirm_writes: false,
            deterministic: false,
            vibe_mode: false,
            max_iterations: 20,
            max_retries: 3,
            context_window: 6,
            max_tokens: 0,
            agents_policy: AgentsPolicy::default(),
            mauns_prefs: MaunsPreferences::default(),
            project: ProjectInfo::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Agent action schema
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentAction {
    Skill {
        name: String,
        input: serde_json::Value,
    },
    Note {
        message: String,
    },
    Done {
        summary: String,
    },
}

// ---------------------------------------------------------------------------
// Git types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GitOutcome {
    pub branch: String,
    pub commit_id: String,
    pub pr_url: Option<String>,
}

// ---------------------------------------------------------------------------
// Skill types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub success: bool,
    pub data: serde_json::Value,
    pub message: String,
}

impl SkillOutput {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data,
            message: String::new(),
        }
    }
    pub fn ok_msg(data: serde_json::Value, message: impl Into<String>) -> Self {
        Self {
            success: true,
            data,
            message: message.into(),
        }
    }
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: serde_json::Value::Null,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsage {
    pub skill_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub success: bool,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_plan(steps: Vec<(usize, Vec<usize>)>) -> Plan {
        Plan {
            task: "test".into(),
            steps: steps
                .into_iter()
                .map(|(id, deps)| Step {
                    id,
                    task: format!("step {id}"),
                    depends_on: deps,
                })
                .collect(),
        }
    }

    #[test]
    fn execution_order_no_deps() {
        let plan = make_plan(vec![(1, vec![]), (2, vec![]), (3, vec![])]);
        let order = plan.execution_order();
        // All ready from start — ids 1,2,3 all appear.
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn execution_order_linear_chain() {
        let plan = make_plan(vec![(1, vec![]), (2, vec![1]), (3, vec![2])]);
        let order = plan.execution_order();
        assert_eq!(order[0].id, 1);
        assert_eq!(order[1].id, 2);
        assert_eq!(order[2].id, 3);
    }

    #[test]
    fn execution_order_diamond() {
        // 1 → 2,3 → 4
        let plan = make_plan(vec![
            (1, vec![]),
            (2, vec![1]),
            (3, vec![1]),
            (4, vec![2, 3]),
        ]);
        let order = plan.execution_order();
        assert_eq!(order[0].id, 1);
        assert_eq!(order[3].id, 4);
    }

    #[test]
    fn token_usage_estimate_nonzero() {
        assert!(TokenUsage::estimate("hello world") > 0);
    }

    #[test]
    fn token_usage_accumulates() {
        let mut u = TokenUsage::default();
        u.add_prompt("hello world");
        u.add_completion("done");
        assert!(u.total() > 0);
        assert!(u.prompt_tokens > 0);
        assert!(u.completion_tokens > 0);
    }
}
