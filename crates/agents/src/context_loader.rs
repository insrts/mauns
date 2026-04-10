use std::path::PathBuf;

use mauns_core::{
    project,
    types::{AgentsPolicy, MaunsPreferences, RunContext},
};
use tracing::debug;

#[allow(clippy::too_many_arguments)]
pub fn load_run_context(
    dry_run: bool,
    confirm_writes: bool,
    deterministic: bool,
    vibe_mode: bool,
    max_iterations: usize,
    max_retries: usize,
    context_window: usize,
    max_tokens: usize,
) -> RunContext {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project_info = project::detect(&cwd);

    debug!(
        context  = "loader",
        language = %project_info.language,
        project  = %project_info.project_type,
    );

    RunContext {
        dry_run,
        confirm_writes,
        deterministic,
        vibe_mode,
        max_iterations,
        max_retries,
        context_window,
        max_tokens,
        agents_policy: load_agents_md(),
        mauns_prefs: load_mauns_md(),
        project: project_info,
    }
}

fn load_agents_md() -> AgentsPolicy {
    match std::fs::read_to_string("AGENTS.md") {
        Ok(raw) => {
            debug!(loader = "context", "AGENTS.md loaded");
            AgentsPolicy { raw }
        }
        Err(_) => AgentsPolicy::default(),
    }
}

fn load_mauns_md() -> MaunsPreferences {
    let path = match home_dir() {
        Some(h) => h.join("MAUNS.md"),
        None => return MaunsPreferences::default(),
    };
    match std::fs::read_to_string(&path) {
        Ok(raw) => {
            debug!(loader = "context", "MAUNS.md loaded");
            MaunsPreferences { raw }
        }
        Err(_) => MaunsPreferences::default(),
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}
