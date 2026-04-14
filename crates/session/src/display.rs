//! Terminal display helpers for the agent session.
//! Renders the splash header, prompts, status lines, and task output
//! using crossterm for cross-platform ANSI control.

use crossterm::{
    style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor},
    ExecutableCommand,
};
use std::io::{stdout, Write};

use crate::state::{SessionMode, SessionState};

const VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Splash screen
// ---------------------------------------------------------------------------

pub fn print_splash(state: &SessionState) {
    let mut out = stdout();

    let _ = out.execute(SetForegroundColor(Color::White));
    println!();
    println!(" ╭────────────────────────────────────────╮");

    let _ = out.execute(SetForegroundColor(Color::Cyan));
    let _ = out.execute(SetAttribute(Attribute::Bold));
    println!(" │  >_ Mauns  (v{VERSION:<8})               │");
    let _ = out.execute(SetAttribute(Attribute::Reset));

    let _ = out.execute(SetForegroundColor(Color::White));
    println!(" │                                        │");

    let provider_line = format!(" │  provider: {:<8}  /models to change  │", state.provider);
    println!("{}", provider_line);

    let model_display = if state.model.is_empty() {
        "(default)".to_string()
    } else {
        state.model.chars().take(24).collect()
    };
    let model_line = format!(" │  model:    {:<28}│", model_display);
    println!("{}", model_line);

    let cwd = std::env::current_dir()
        .map(|p| {
            let s = p.display().to_string();
            if s.len() > 28 {
                format!("...{}", &s[s.len() - 25..])
            } else {
                s
            }
        })
        .unwrap_or_else(|_| ".".to_string());
    let dir_line = format!(" │  directory: {:<27}│", cwd);
    println!("{}", dir_line);

    println!(" ╰────────────────────────────────────────╯");
    println!();

    let _ = out.execute(SetForegroundColor(Color::DarkGrey));
    println!("  Tip: Type a task and press Enter to run it.");
    println!("       Use /help to see all available commands.");
    let _ = out.execute(ResetColor);
    println!();

    let _ = out.flush();
}

// ---------------------------------------------------------------------------
// Prompt line
// ---------------------------------------------------------------------------

pub fn print_prompt(state: &SessionState) {
    let mut out = stdout();
    let mode_indicator = match state.mode {
        SessionMode::DryRun => " [dry-run]",
        SessionMode::Vibe => " [vibe]",
        _ => "",
    };

    let _ = out.execute(SetForegroundColor(Color::Green));
    let _ = out.execute(SetAttribute(Attribute::Bold));
    print!("> ");
    let _ = out.execute(SetAttribute(Attribute::Reset));
    let _ = out.execute(SetForegroundColor(Color::DarkGrey));
    print!("{mode_indicator} ");
    let _ = out.execute(ResetColor);
    let _ = out.flush();
}

// ---------------------------------------------------------------------------
// Info / success / error / warning
// ---------------------------------------------------------------------------

pub fn print_info(msg: &str) {
    let mut out = stdout();
    let _ = out.execute(SetForegroundColor(Color::Cyan));
    print!("  ");
    let _ = out.execute(ResetColor);
    println!("{msg}");
}

pub fn print_success(msg: &str) {
    let mut out = stdout();
    let _ = out.execute(SetForegroundColor(Color::Green));
    let _ = out.execute(SetAttribute(Attribute::Bold));
    print!("  [ok] ");
    let _ = out.execute(SetAttribute(Attribute::Reset));
    let _ = out.execute(ResetColor);
    println!("{msg}");
}

pub fn print_error(msg: &str) {
    let mut out = stdout();
    let _ = out.execute(SetForegroundColor(Color::Red));
    let _ = out.execute(SetAttribute(Attribute::Bold));
    eprint!("  [error] ");
    let _ = out.execute(SetAttribute(Attribute::Reset));
    let _ = out.execute(ResetColor);
    eprintln!("{msg}");
}

pub fn print_warning(msg: &str) {
    let mut out = stdout();
    let _ = out.execute(SetForegroundColor(Color::Yellow));
    print!("  [warn] ");
    let _ = out.execute(ResetColor);
    println!("{msg}");
}

pub fn print_dim(msg: &str) {
    let mut out = stdout();
    let _ = out.execute(SetForegroundColor(Color::DarkGrey));
    println!("  {msg}");
    let _ = out.execute(ResetColor);
}

// ---------------------------------------------------------------------------
// Running indicator
// ---------------------------------------------------------------------------

pub fn print_running(task: &str) {
    let mut out = stdout();
    println!();
    let _ = out.execute(SetForegroundColor(Color::Cyan));
    let _ = out.execute(SetAttribute(Attribute::Bold));
    print!("  Running: ");
    let _ = out.execute(SetAttribute(Attribute::Reset));
    let _ = out.execute(ResetColor);
    println!("{task}");
    println!();
    let _ = out.flush();
}

pub fn print_step(id: usize, task: &str) {
    let mut out = stdout();
    let _ = out.execute(SetForegroundColor(Color::Blue));
    print!("  [{id}] ");
    let _ = out.execute(ResetColor);
    println!("{task}");
}

pub fn print_step_done(id: usize) {
    let mut out = stdout();
    let _ = out.execute(SetForegroundColor(Color::Green));
    println!("  [{id}] done");
    let _ = out.execute(ResetColor);
}

pub fn print_step_retry(id: usize, attempt: usize) {
    let mut out = stdout();
    let _ = out.execute(SetForegroundColor(Color::Yellow));
    println!("  [{id}] retry {attempt}...");
    let _ = out.execute(ResetColor);
}

// ---------------------------------------------------------------------------
// Section headers
// ---------------------------------------------------------------------------

pub fn print_section(title: &str) {
    let mut out = stdout();
    println!();
    let _ = out.execute(SetForegroundColor(Color::White));
    let _ = out.execute(SetAttribute(Attribute::Bold));
    println!("  --- {title} ---");
    let _ = out.execute(SetAttribute(Attribute::Reset));
    let _ = out.execute(ResetColor);
}

// ---------------------------------------------------------------------------
// Diff display
// ---------------------------------------------------------------------------

pub fn print_diff(diff: &str) {
    let mut out = stdout();
    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            let _ = out.execute(SetForegroundColor(Color::Green));
        } else if line.starts_with('-') && !line.starts_with("---") {
            let _ = out.execute(SetForegroundColor(Color::Red));
        } else if line.starts_with("@@") {
            let _ = out.execute(SetForegroundColor(Color::Cyan));
        } else {
            let _ = out.execute(SetForegroundColor(Color::DarkGrey));
        }
        println!("    {line}");
        let _ = out.execute(ResetColor);
    }
}
