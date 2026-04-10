use mauns_core::types::{FileOperation, Plan, ProgressReporter, TaskReport};

#[derive(Debug, Clone, Copy)]
pub enum Verbosity {
    Normal,
    Verbose,
    Debug,
}

pub struct Ui {
    verbosity: Verbosity,
}

impl ProgressReporter for Ui {
    fn on_plan(&self, plan: &Plan) {
        self.print_plan(plan);
    }

    fn on_step_complete(&self, id: usize, task: &str) {
        self.print_step_completion(id, task);
    }

    fn on_step_failure(&self, id: usize, task: &str, error: &str) {
        self.print_step_failure(id, task, error);
    }

    fn on_result(&self, summary: &str) {
        self.print_result(summary);
    }
}

impl Ui {
    pub fn new(verbosity: Verbosity) -> Self {
        Self { verbosity }
    }

    pub fn print_task(&self, task: &str) {
        println!();
        println!("Running task: {}", task);
        println!();
    }

    pub fn print_plan(&self, plan: &Plan) {
        println!("Plan:");
        for step in &plan.steps {
            println!("{}. {}", step.id, step.task);
        }
        println!();
    }

    pub fn print_execution_start(&self) {
        println!("Execution:");
    }

    pub fn print_step_completion(&self, id: usize, _task: &str) {
        println!("✓ Step {} completed", id);
    }

    pub fn print_step_failure(&self, id: usize, _task: &str, error: &str) {
        println!("✗ Step {} failed: {}", id, error);
    }

    pub fn print_result(&self, summary: &str) {
        println!();
        println!("Result:");
        println!("{}", summary.trim());
        println!();
    }

    pub fn is_verbose(&self) -> bool {
        matches!(self.verbosity, Verbosity::Verbose | Verbosity::Debug)
    }

    pub fn is_debug(&self) -> bool {
        matches!(self.verbosity, Verbosity::Debug)
    }
}

pub fn print_report(report: &TaskReport) {
    if report.interrupted {
        println!();
        println!("Interrupted — partial results only.");
        println!();
    }

    if !report.change_log.is_empty() {
        println!("Changes:");
        for change in &report.change_log {
            let status = if change.applied { "✓" } else { "→" };
            let op = match change.operation {
                FileOperation::Create => "created",
                FileOperation::Edit => "modified",
                FileOperation::Delete => "deleted",
            };
            println!("  {} {} {}", status, op, change.path);
        }
        println!();
    }

    if let Some(ref git) = report.git_outcome {
        println!("Git:");
        println!("  branch:  {}", git.branch);
        println!("  commit:  {}", git.commit_id);
        if let Some(ref url) = git.pr_url {
            println!("  pr:      {url}");
        }
        println!();
    }

    println!("Summary:");
    println!("  tokens:  {} ({} prompt, {} completion)",
        report.execution.token_usage.total(),
        report.execution.token_usage.prompt_tokens,
        report.execution.token_usage.completion_tokens,
    );
    println!("  status:  {}", if report.verification.passed { "Success" } else { "Incomplete" });
    println!();
}
