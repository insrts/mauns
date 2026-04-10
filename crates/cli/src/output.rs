use mauns_core::types::{FileOperation, TaskReport};

pub fn print_report(report: &TaskReport) {
    println!();
    println!("=== MAUNS TASK REPORT ===");
    println!();

    if report.interrupted {
        println!("  [INTERRUPTED — partial results]");
        println!();
    }

    println!("Task: {}", report.task);
    println!();

    println!("--- PLAN ---");
    for step in &report.plan.steps {
        let deps = if step.depends_on.is_empty() {
            String::new()
        } else {
            format!(
                "  (depends on: {})",
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

    println!("--- EXECUTION ---");
    println!("  iterations:   {}", report.execution.iterations);
    println!("  retries used: {}", report.execution.total_retries);
    println!(
        "  tokens:       {} (prompt: {}, completion: {})",
        report.execution.token_usage.total(),
        report.execution.token_usage.prompt_tokens,
        report.execution.token_usage.completion_tokens,
    );
    println!();

    for result in &report.execution.results {
        let retry_note = if result.retries_used > 0 {
            format!(
                "  [{} retr{}]",
                result.retries_used,
                if result.retries_used == 1 { "y" } else { "ies" }
            )
        } else {
            String::new()
        };
        println!(
            "  [step {}]{retry_note} {}",
            result.step.id, result.step.task
        );
        println!("  output: {}", result.output.trim());
        println!();
    }
    println!("  summary: {}", report.execution.summary.trim());
    println!();

    println!("--- VERIFICATION ---");
    let verdict = if report.verification.passed {
        "PASSED"
    } else {
        "FAILED"
    };
    println!("  verdict:  {verdict}");
    println!("  feedback: {}", report.verification.feedback.trim());
    if report.verification.retry_suggested && !report.verification.passed {
        println!("  note:     retry may improve results");
    }
    println!();

    if !report.change_log.is_empty() {
        println!("--- FILESYSTEM CHANGES ---");
        for change in &report.change_log {
            let status = if change.applied { "applied" } else { "dry-run" };
            let op = match change.operation {
                FileOperation::Create => "create",
                FileOperation::Edit => "edit",
                FileOperation::Delete => "delete",
            };
            println!("  [{}] {} {}", status, op, change.path);
            if !change.diff.is_empty() {
                for line in change.diff.lines() {
                    println!("    {line}");
                }
                println!();
            }
        }
    }

    if !report.skill_log.is_empty() {
        println!("--- SKILL USAGE ---");
        for entry in &report.skill_log {
            let status = if entry.success { "ok" } else { "err" };
            let ts = entry.timestamp.format("%H:%M:%S");
            if entry.message.is_empty() {
                println!("  [{status}] {ts}  {}", entry.skill_name);
            } else {
                println!(
                    "  [{status}] {ts}  {}  -- {}",
                    entry.skill_name,
                    entry.message.trim()
                );
            }
        }
        println!();
    }

    if let Some(ref git) = report.git_outcome {
        println!("--- GIT OUTCOME ---");
        println!("  branch:  {}", git.branch);
        println!("  commit:  {}", git.commit_id);
        if let Some(ref url) = git.pr_url {
            println!("  pr:      {url}");
        } else {
            println!("  pr:      (not created)");
        }
        println!();
    }

    println!("=========================");
    println!();
}
