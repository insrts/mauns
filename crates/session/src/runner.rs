//! Agent session runner — the main REPL loop.
//!
//! Reads input from stdin line-by-line, dispatches slash commands
//! or runs agent tasks, and updates `SessionState` after each run.

use std::{
    io::{self, BufRead, Write},
    sync::Arc,
};

use mauns_agents::{
    context_loader::load_run_context,
    git_orchestrator::GitConfig,
    Pipeline,
};
use mauns_core::{
    error::MaunsError,
    types::{Plan, ProgressReporter},
};
use mauns_llm::{
    build_provider_with_model, deterministic::DeterministicProvider,
    provider::LlmProvider, ProviderKind,
};

use crate::{
    commands::{handle_command, CommandResult},
    display::{
        print_dim, print_error, print_info, print_running, print_section,
        print_splash, print_step, print_step_done, print_step_retry, print_success,
        print_warning,
    },
    history::CommandHistory,
    state::{SessionMode, SessionState},
};

/// Drives the full interactive agent session.
pub struct SessionRunner {
    state:   SessionState,
    history: CommandHistory,
}

impl SessionRunner {
    pub fn new(state: SessionState) -> Self {
        Self {
            state,
            history: CommandHistory::load(),
        }
    }

    /// Enter the REPL loop. Blocks until the user exits.
    pub async fn run(mut self) {
        // Validate config has at least one key set.
        if let Err(e) = self.state.config.validate() {
            print_error(&format!("Configuration error: {e}"));
            print_dim("Run /config or set environment variables to fix.");
            println!();
        }

        print_splash(&self.state);

        let stdin  = io::stdin();
        let stdout = io::stdout();

        loop {
            // Print the prompt.
            {
                use crate::display::print_prompt;
                print_prompt(&self.state);
                let mut out = stdout.lock();
                let _ = out.flush();
            }

            // Read a line.
            let mut line = String::new();
            match stdin.lock().read_line(&mut line) {
                Ok(0) => {
                    // EOF (Ctrl+D)
                    println!();
                    print_info("Session ended.");
                    break;
                }
                Err(e) => {
                    print_error(&format!("Read error: {e}"));
                    break;
                }
                Ok(_) => {}
            }

            let input = line.trim().to_string();
            if input.is_empty() {
                continue;
            }

            // Record in history (both tasks and slash commands).
            self.history.push(&input);

            if input.starts_with('/') {
                // Slash command.
                match handle_command(&input, &mut self.state, &self.history) {
                    CommandResult::Exit             => break,
                    CommandResult::Continue         => {}
                    CommandResult::ProviderChanged  => {
                        // Rebuild provider with new selection on next task run.
                        print_dim(&format!(
                            "Provider changed to '{}'. New model will apply on the next task.",
                            self.state.provider
                        ));
                    }
                }
            } else {
                // Task input — run the agent pipeline.
                self.run_task(input).await;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Task execution
    // -----------------------------------------------------------------------

    async fn run_task(&mut self, task: String) {
        // Promote API keys from config into env (LLM registry reads env).
        self.promote_api_keys();

        // Build the provider.
        let kind: ProviderKind = match self.state.provider.parse() {
            Ok(k)  => k,
            Err(_) => {
                print_error(&format!(
                    "Unknown provider '{}'. Use /models to switch.",
                    self.state.provider
                ));
                return;
            }
        };

        let model = self.state.effective_model().map(|s| s.to_string());
        let base = match build_provider_with_model(&kind, model.as_deref()) {
            Ok(p)  => p,
            Err(e) => {
                print_error(&format!("Provider error: {e}"));
                print_dim("Set the API key with: export GROQ_API_KEY=... (or CLAUDE_API_KEY / OPENAI_API_KEY)");
                return;
            }
        };

        let provider: Arc<dyn LlmProvider> = if self.state.deterministic {
            Arc::new(DeterministicProvider::new(base))
        } else {
            base
        };

        // Build run context from current session state.
        let exec = &self.state.config.execution;
        let ctx = load_run_context(
            self.state.is_dry_run(),
            self.state.config.safety.confirm_before_write,
            self.state.deterministic,
            self.state.is_vibe(),
            exec.max_iterations,
            exec.max_retries,
            exec.context_window,
            0, // no token limit in session mode
        );

        let git_cfg = GitConfig::new(self.state.config.git.create_pr, false);

        // Progress reporter: prints live step updates to the terminal.
        let reporter = SessionProgressReporter;

        self.state.set_mode(if self.state.is_dry_run() {
            SessionMode::DryRun
        } else if self.state.is_vibe() {
            SessionMode::Vibe
        } else {
            SessionMode::Running
        });

        print_running(&task);

        let pipeline = Pipeline::new(provider, git_cfg, vec![]);
        let result   = pipeline.run(&task, &ctx, Some(&reporter)).await;

        // Restore interactive mode (unless user set dry-run/vibe persistently).
        if self.state.mode == SessionMode::Running {
            self.state.mode = SessionMode::Interactive;
        }

        match result {
            Ok(report) => {
                // Store plan and report.
                self.state.last_plan = Some(report.plan.clone());
                self.state.task_history.push(task.clone());
                self.state.run_count += 1;

                // Print summary.
                println!();
                print_section("Result");
                println!("  {}", report.execution.summary.trim());

                // Verification verdict.
                println!();
                if report.verification.passed {
                    print_success("Verification passed.");
                } else {
                    print_warning(&format!(
                        "Verification: {}",
                        report.verification.feedback.trim()
                    ));
                    if report.verification.retry_suggested {
                        print_dim("Tip: rerun the task or adjust the prompt.");
                    }
                }

                // File changes summary.
                let applied: Vec<_> = report.change_log.iter()
                    .filter(|c| c.applied)
                    .collect();
                if !applied.is_empty() {
                    println!();
                    print_info(&format!("{} file(s) changed. Use /diff to view.", applied.len()));
                } else if self.state.is_dry_run() {
                    print_dim("Dry-run: no files written.");
                }

                // Git outcome.
                if let Some(ref git) = report.git_outcome {
                    println!();
                    print_success(&format!("Branch: {}", git.branch));
                    if let Some(ref url) = git.pr_url {
                        print_success(&format!("PR: {url}"));
                    }
                }

                // Token usage.
                println!();
                print_dim(&format!(
                    "Tokens: {}  |  Iterations: {}  |  Retries: {}",
                    report.execution.token_usage.total(),
                    report.execution.iterations,
                    report.execution.total_retries,
                ));

                // Interrupted warning.
                if report.interrupted {
                    print_warning("Run was interrupted before completion.");
                }

                self.state.reports.push(report);
            }
            Err(MaunsError::Aborted) => {
                print_warning("Aborted by user.");
            }
            Err(e) => {
                println!();
                print_error(&format!("{e}"));

                // Provide targeted help for common errors.
                match &e {
                    MaunsError::Config(msg) if msg.contains("API_KEY") => {
                        print_dim("Set the key: export GROQ_API_KEY=gsk_...");
                        print_dim("Or use /models to switch provider.");
                    }
                    MaunsError::LimitExceeded(_) => {
                        print_dim("Use /config max_iterations <n> to increase the limit.");
                    }
                    MaunsError::PathTraversal(_) | MaunsError::RestrictedPath(_) => {
                        print_dim("The task tried to access a protected path.");
                        print_dim("Check .maunsignore if you want to allow it.");
                    }
                    _ => {}
                }
            }
        }

        println!();
    }

    // -----------------------------------------------------------------------
    // Key promotion
    // -----------------------------------------------------------------------

    fn promote_api_keys(&self) {
        let cfg = &self.state.config;
        if std::env::var("CLAUDE_API_KEY").is_err() && !cfg.claude.api_key.is_empty() {
            std::env::set_var("CLAUDE_API_KEY", &cfg.claude.api_key);
        }
        if std::env::var("OPENAI_API_KEY").is_err() && !cfg.openai.api_key.is_empty() {
            std::env::set_var("OPENAI_API_KEY", &cfg.openai.api_key);
        }
        if std::env::var("GROQ_API_KEY").is_err() && !cfg.groq.api_key.is_empty() {
            std::env::set_var("GROQ_API_KEY", &cfg.groq.api_key);
        }
    }
}

// ---------------------------------------------------------------------------
// Progress reporter for the session (live step output)
// ---------------------------------------------------------------------------

struct SessionProgressReporter;

impl ProgressReporter for SessionProgressReporter {
    fn on_plan(&self, plan: &Plan) {
        print_section("Plan");
        for step in &plan.steps {
            let deps = if step.depends_on.is_empty() {
                String::new()
            } else {
                format!(
                    "  (after: {})",
                    step.depends_on
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            println!("  [{}] {}{deps}", step.id, step.task);
        }
        println!();
    }

    fn on_execution_start(&self) {
        print_section("Executing");
    }

    fn on_step_complete(&self, id: usize, task: &str) {
        print_step_done(id);
        let _ = task;
    }

    fn on_step_failure(&self, id: usize, task: &str, error: &str) {
        print_step_retry(id, 0);
        let _ = (task, error);
    }

    fn on_result(&self, summary: &str) {
        let _ = summary;
    }
}
