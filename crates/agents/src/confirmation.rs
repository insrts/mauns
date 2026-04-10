//! Interactive confirmation prompt for destructive operations.
//!
//! This module enforces the mandatory diff-review gate before any commit.
//! It cannot be suppressed by AGENTS.md or LLM output — only by the
//! explicit `--no-confirm` CLI flag (dry-run implies no confirmation needed).

use std::io::{self, BufRead, Write};

use mauns_core::{
    error::{MaunsError, Result},
    types::{FileChange, FileOperation},
};
use tracing::info;

/// Print all pending diffs and ask the user whether to proceed.
///
/// Returns `Ok(())` if the user confirms, `Err(MaunsError::Aborted)` otherwise.
/// In dry-run mode this function returns immediately without prompting.
pub fn confirm_changes(changes: &[FileChange], dry_run: bool) -> Result<()> {
    let applied: Vec<&FileChange> = changes.iter().filter(|c| c.applied).collect();

    if applied.is_empty() {
        info!(confirmation = "skip", reason = "no applied changes");
        return Ok(());
    }

    if dry_run {
        info!(confirmation = "skip", reason = "dry-run mode");
        return Ok(());
    }

    // Print the full diff for every applied change.
    println!();
    println!("=== PENDING CHANGES ===");
    println!();

    for change in &applied {
        let op = match change.operation {
            FileOperation::Create => "CREATE",
            FileOperation::Edit => "EDIT",
            FileOperation::Delete => "DELETE",
        };
        println!("[{op}] {}", change.path);

        if !change.diff.is_empty() {
            for line in change.diff.lines() {
                println!("  {line}");
            }
        }
        println!();
    }

    println!("=======================");
    println!("{} file(s) will be committed.", applied.len());
    println!();

    // Prompt.
    prompt_user()
}

fn prompt_user() -> Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    loop {
        write!(out, "Apply these changes? (y/n): ").map_err(MaunsError::Io)?;
        out.flush().map_err(MaunsError::Io)?;

        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).map_err(MaunsError::Io)?;

        match line.trim().to_lowercase().as_str() {
            "y" | "yes" => {
                info!(confirmation = "accepted");
                return Ok(());
            }
            "n" | "no" => {
                info!(confirmation = "rejected");
                return Err(MaunsError::Aborted);
            }
            _ => {
                println!("Please enter 'y' or 'n'.");
            }
        }
    }
}
