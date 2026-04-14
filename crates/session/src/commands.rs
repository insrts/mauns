//! Slash command parser and handler.
//!
//! All commands start with `/`.  Unknown commands show a help hint.

use mauns_core::types::FileOperation;
use mauns_llm::{models_for_provider, ProviderKind};

use crate::{
    display::{
        print_diff, print_dim, print_error, print_info, print_section, print_success, print_warning,
    },
    history::CommandHistory,
    state::{SessionMode, SessionState},
};

/// Result of processing a slash command.
pub enum CommandResult {
    /// Session should continue normally.
    Continue,
    /// Session should exit cleanly.
    Exit,
    /// Provider/model changed — caller must rebuild the provider.
    ProviderChanged,
}

/// Parse and execute a slash command.
///
/// Returns `Err` only on unrecoverable internal errors.
pub fn handle_command(
    input: &str,
    state: &mut SessionState,
    history: &CommandHistory,
) -> CommandResult {
    let trimmed = input.trim();
    let (cmd, rest) = match trimmed.split_once(char::is_whitespace) {
        Some((c, r)) => (c, r.trim()),
        None => (trimmed, ""),
    };

    match cmd {
        "/exit" | "/quit" | "/q" => {
            print_info("Goodbye.");
            CommandResult::Exit
        }

        "/help" | "/?" => {
            cmd_help();
            CommandResult::Continue
        }

        "/config" => {
            cmd_config(state, rest);
            CommandResult::Continue
        }

        "/models" => {
            let changed = cmd_models(state, rest);
            if changed {
                CommandResult::ProviderChanged
            } else {
                CommandResult::Continue
            }
        }

        "/plan" => {
            cmd_plan(state);
            CommandResult::Continue
        }

        "/status" => {
            cmd_status(state);
            CommandResult::Continue
        }

        "/history" => {
            cmd_history(history, rest);
            CommandResult::Continue
        }

        "/clear" => {
            // Clear the terminal screen.
            print!("\x1b[2J\x1b[H");
            CommandResult::Continue
        }

        "/diff" => {
            cmd_diff(state);
            CommandResult::Continue
        }

        "/dry-run" => {
            cmd_toggle_dryrun(state);
            CommandResult::Continue
        }

        "/vibe" => {
            cmd_toggle_vibe(state);
            CommandResult::Continue
        }

        "/deterministic" => {
            state.deterministic = !state.deterministic;
            if state.deterministic {
                print_success("Deterministic mode enabled (temperature=0).");
            } else {
                print_info("Deterministic mode disabled.");
            }
            CommandResult::Continue
        }

        "/reset" => {
            cmd_reset(state);
            CommandResult::Continue
        }

        "/workspace" => {
            let cwd = std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string());
            print_info(&format!("Workspace: {cwd}"));
            CommandResult::Continue
        }

        "/files" => {
            cmd_files(state);
            CommandResult::Continue
        }

        "/tokens" => {
            cmd_tokens(state);
            CommandResult::Continue
        }

        _ => {
            print_warning(&format!(
                "Unknown command '{cmd}'. Type /help for a list of commands."
            ));
            CommandResult::Continue
        }
    }
}

// ---------------------------------------------------------------------------
// Individual command implementations
// ---------------------------------------------------------------------------

fn cmd_help() {
    print_section("Commands");
    let cmds = [
        ("/help", "Show this help message"),
        ("/config", "View or set configuration  (/config key value)"),
        (
            "/models",
            "List or switch provider/model  (/models groq llama-3.3-70b-versatile)",
        ),
        ("/plan", "Display the last generated plan"),
        ("/status", "Show current session status"),
        ("/history [n]", "Show last N task inputs (default: 10)"),
        ("/diff", "Show diffs from the last run"),
        ("/files", "List files changed in the last run"),
        ("/tokens", "Show token usage from the last run"),
        ("/dry-run", "Toggle dry-run mode (no disk writes)"),
        ("/vibe", "Toggle vibe mode (faster, fewer prompts)"),
        (
            "/deterministic",
            "Toggle deterministic mode (temperature=0)",
        ),
        ("/reset", "Clear session state (keep config)"),
        ("/workspace", "Show the current working directory"),
        ("/clear", "Clear the terminal screen"),
        ("/exit", "Exit the session"),
    ];
    for (cmd, desc) in &cmds {
        println!("  {:<26}  {desc}", cmd);
    }
    println!();
}

fn cmd_config(state: &mut SessionState, rest: &str) {
    if rest.is_empty() {
        // Display current config.
        print_section("Configuration");
        println!("  provider          = {}", state.provider);
        let model_display = if state.model.is_empty() {
            "(default)"
        } else {
            &state.model
        };
        println!("  model             = {model_display}");
        println!("  deterministic     = {}", state.deterministic);
        println!("  dry_run           = {}", state.is_dry_run());
        println!("  vibe              = {}", state.is_vibe());
        println!(
            "  confirm_writes    = {}",
            state.config.safety.confirm_before_write
        );
        println!("  create_pr         = {}", state.config.git.create_pr);
        println!(
            "  max_iterations    = {}",
            state.config.execution.max_iterations
        );
        println!(
            "  max_retries       = {}",
            state.config.execution.max_retries
        );
        println!(
            "  context_window    = {}",
            state.config.execution.context_window
        );
        println!();
        print_dim("Use /config <key> <value> to change a setting.");
        print_dim("Example: /config max_iterations 30");
    } else {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();

        if value.is_empty() {
            print_error(&format!("Usage: /config {key} <value>"));
            return;
        }

        match key {
            "max_iterations" => {
                if let Ok(n) = value.parse::<usize>() {
                    state.config.execution.max_iterations = n;
                    print_success(&format!("max_iterations = {n}"));
                } else {
                    print_error("Value must be a positive integer.");
                }
            }
            "max_retries" => {
                if let Ok(n) = value.parse::<usize>() {
                    state.config.execution.max_retries = n;
                    print_success(&format!("max_retries = {n}"));
                } else {
                    print_error("Value must be a positive integer.");
                }
            }
            "context_window" => {
                if let Ok(n) = value.parse::<usize>() {
                    state.config.execution.context_window = n;
                    print_success(&format!("context_window = {n}"));
                } else {
                    print_error("Value must be a positive integer.");
                }
            }
            "confirm_writes" => {
                state.config.safety.confirm_before_write = matches!(value, "true" | "1" | "yes");
                print_success(&format!(
                    "confirm_writes = {}",
                    state.config.safety.confirm_before_write
                ));
            }
            "create_pr" => {
                state.config.git.create_pr = matches!(value, "true" | "1" | "yes");
                print_success(&format!("create_pr = {}", state.config.git.create_pr));
            }
            other => {
                print_error(&format!(
                    "Unknown config key '{other}'. Type /config to see all keys."
                ));
            }
        }
    }
}

/// Returns true when the provider/model changed.
fn cmd_models(state: &mut SessionState, rest: &str) -> bool {
    if rest.is_empty() {
        // List all providers and their models.
        print_section("Providers & Models");
        for kind in ProviderKind::all() {
            let current = kind.as_str() == state.provider.as_str();
            let marker = if current { " *" } else { "  " };
            println!("{marker} {}", kind.as_str().to_uppercase());
            for (id, desc) in models_for_provider(kind) {
                let active = current && id == state.model.as_str();
                let m = if active { "  > " } else { "    " };
                println!("{m}{:<44} {desc}", id);
            }
            println!();
        }
        print_dim("Usage: /models <provider> [model]");
        print_dim("Example: /models groq llama-3.3-70b-versatile");
        return false;
    }

    let mut parts = rest.splitn(2, char::is_whitespace);
    let provider_str = parts.next().unwrap_or("").trim().to_lowercase();
    let model_str = parts.next().unwrap_or("").trim().to_string();

    let kind: ProviderKind = match provider_str.parse() {
        Ok(k) => k,
        Err(_) => {
            print_error(&format!(
                "Unknown provider '{provider_str}'. Choose from: openai, anthropic, groq"
            ));
            return false;
        }
    };

    // Validate model if specified.
    if !model_str.is_empty() {
        let valid = models_for_provider(&kind)
            .iter()
            .any(|(id, _)| *id == model_str.as_str());
        if !valid {
            print_warning(&format!(
                "Model '{model_str}' is not in the known list for {provider_str}."
            ));
            print_warning("Proceeding anyway — the API will reject it if invalid.");
        }
    }

    state.provider = provider_str;
    state.model = model_str;

    let model_display = if state.model.is_empty() {
        "(default)".to_string()
    } else {
        state.model.clone()
    };
    print_success(&format!(
        "Provider: {}  Model: {model_display}",
        state.provider
    ));
    true
}

fn cmd_plan(state: &SessionState) {
    match &state.last_plan {
        None => print_info("No plan has been generated yet. Run a task first."),
        Some(plan) => {
            print_section("Last Plan");
            println!("  Task: {}", plan.task);
            println!();
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
    }
}

fn cmd_status(state: &SessionState) {
    print_section("Session Status");
    println!("  mode:          {}", state.mode);
    println!("  provider:      {}", state.provider);
    let model_display = if state.model.is_empty() {
        "(default)".to_string()
    } else {
        state.model.clone()
    };
    println!("  model:         {model_display}");
    println!("  deterministic: {}", state.deterministic);
    println!("  tasks run:     {}", state.run_count);

    if let Some(last) = state.task_history.last() {
        println!("  last task:     {last}");
    }

    if let Some(report) = state.reports.last() {
        let verdict = if report.verification.passed {
            "passed"
        } else {
            "failed"
        };
        println!("  last verdict:  {verdict}");
        println!("  last tokens:   {}", report.execution.token_usage.total());
        println!("  last iters:    {}", report.execution.iterations);
    }
    println!();
}

fn cmd_history(history: &CommandHistory, rest: &str) {
    let n: usize = rest.parse().unwrap_or(10);
    let entries = history.recent(n);
    if entries.is_empty() {
        print_info("No history yet.");
        return;
    }
    print_section("History");
    let start = history.entries().len().saturating_sub(n);
    for (i, entry) in entries.iter().enumerate() {
        println!("  {:>4}  {entry}", start + i + 1);
    }
    println!();
}

fn cmd_diff(state: &SessionState) {
    let report = match state.reports.last() {
        Some(r) => r,
        None => {
            print_info("No runs yet.");
            return;
        }
    };

    let applied: Vec<_> = report.change_log.iter().filter(|c| c.applied).collect();
    if applied.is_empty() {
        print_info("No file changes in the last run.");
        return;
    }

    print_section("Diffs from last run");
    for change in applied {
        let op = match change.operation {
            FileOperation::Create => "CREATE",
            FileOperation::Edit => "EDIT",
            FileOperation::Delete => "DELETE",
        };
        println!("  [{op}] {}", change.path);
        if !change.diff.is_empty() {
            print_diff(&change.diff);
        }
        println!();
    }
}

fn cmd_toggle_dryrun(state: &mut SessionState) {
    if state.mode == SessionMode::DryRun {
        state.mode = SessionMode::Interactive;
        print_info("Dry-run mode disabled.");
    } else {
        state.mode = SessionMode::DryRun;
        print_success("Dry-run mode enabled. File writes will be simulated.");
    }
}

fn cmd_toggle_vibe(state: &mut SessionState) {
    if state.mode == SessionMode::Vibe {
        state.mode = SessionMode::Interactive;
        print_info("Vibe mode disabled.");
    } else {
        state.mode = SessionMode::Vibe;
        print_success("Vibe mode enabled. Faster execution, single iteration per step.");
    }
}

fn cmd_reset(state: &mut SessionState) {
    state.last_plan = None;
    state.reports = Vec::new();
    state.run_count = 0;
    state.mode = SessionMode::Interactive;
    state.deterministic = false;
    print_success("Session state cleared. Config retained.");
}

fn cmd_files(state: &SessionState) {
    let report = match state.reports.last() {
        Some(r) => r,
        None => {
            print_info("No runs yet.");
            return;
        }
    };

    let applied: Vec<_> = report.change_log.iter().filter(|c| c.applied).collect();
    if applied.is_empty() {
        print_info("No file changes in the last run.");
        return;
    }

    print_section("Files changed in last run");
    for change in applied {
        let op = match change.operation {
            FileOperation::Create => "create",
            FileOperation::Edit => "edit",
            FileOperation::Delete => "delete",
        };
        println!("  [{op}] {}", change.path);
    }
    println!();
}

fn cmd_tokens(state: &SessionState) {
    let report = match state.reports.last() {
        Some(r) => r,
        None => {
            print_info("No runs yet.");
            return;
        }
    };

    print_section("Token usage — last run");
    println!(
        "  prompt:     {}",
        report.execution.token_usage.prompt_tokens
    );
    println!(
        "  completion: {}",
        report.execution.token_usage.completion_tokens
    );
    println!("  total:      {}", report.execution.token_usage.total());
    println!();

    // Cumulative across session.
    let total_all: usize = state
        .reports
        .iter()
        .map(|r| r.execution.token_usage.total())
        .sum();
    println!("  session total: {total_all}");
    println!();
}
