//! Iterative agent execution loop.
//!
//! Phase 6 additions:
//!   - Token tracking (prompt + completion) with optional hard limit
//!   - Interrupt support via tokio::signal (Ctrl+C → graceful partial return)
//!   - Smarter retry: detects repeated identical failures and changes strategy
//!   - Key-output memory: stores important outputs for cross-step reuse
//!   - Dependency-aware step ordering via Plan::execution_order()

use std::sync::Arc;

use mauns_core::{
    error::{MaunsError, Result},
    types::{
        AgentAction, ExecutionOutput, Plan, ProgressReporter, RunContext, SkillInput, SkillOutput,
        SkillUsage, StepResult, TokenUsage,
    },
};
use mauns_llm::provider::{LlmProvider, SamplingOptions};
use mauns_skills::skillset::SkillSet;
use tracing::{info, warn};

pub const MAX_SKILL_CALLS: usize = 50;
const MAX_PARSE_RETRIES: usize = 3;

// ---------------------------------------------------------------------------
// Structured execution context
// ---------------------------------------------------------------------------

struct ExecContext {
    summary: String,
    recent: Vec<String>,
    last_error: Option<String>,
    last_reflection: Option<String>,
    /// Named key outputs stored for cross-step reuse.
    key_outputs: Vec<(String, String)>,
    window: usize,
    /// Tracks the last N failure messages to detect loops.
    failure_history: Vec<String>,
}

impl ExecContext {
    fn new(window: usize) -> Self {
        Self {
            summary: String::new(),
            recent: Vec::new(),
            last_error: None,
            last_reflection: None,
            key_outputs: Vec::new(),
            window,
            failure_history: Vec::new(),
        }
    }

    fn push(&mut self, entry: impl Into<String>) {
        let entry = entry.into();
        if self.recent.len() >= self.window {
            let oldest = self.recent.remove(0);
            let digest = truncate(&oldest, 120);
            if self.summary.is_empty() {
                self.summary = format!("Earlier: {digest}");
            } else {
                self.summary.push_str(&format!("; {digest}"));
            }
        }
        self.recent.push(entry);
    }

    fn set_error(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        self.failure_history.push(msg.clone());
        // Keep only the last 5 failures for loop detection.
        if self.failure_history.len() > 5 {
            self.failure_history.remove(0);
        }
        self.last_error = Some(msg);
    }

    fn clear_error(&mut self) {
        self.last_error = None;
    }

    fn set_reflection(&mut self, r: impl Into<String>) {
        self.last_reflection = Some(r.into());
    }

    /// Store an important output by key for later reuse.
    fn store_key_output(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let k = key.into();
        let v = value.into();
        // Replace existing key if present.
        if let Some(existing) = self.key_outputs.iter_mut().find(|(ek, _)| *ek == k) {
            existing.1 = v;
        } else {
            self.key_outputs.push((k, v));
        }
    }

    /// Return true when the same failure message has appeared >= 3 times —
    /// indicating a stuck loop. The caller should change strategy.
    fn is_looping(&self) -> bool {
        if self.failure_history.len() < 3 {
            return false;
        }
        let last = self.failure_history.last().unwrap();
        self.failure_history.iter().filter(|m| *m == last).count() >= 3
    }

    fn render(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        if !self.summary.is_empty() {
            parts.push(format!("Summary of earlier work: {}", self.summary));
        }
        if !self.recent.is_empty() {
            parts.push(format!("Recent steps:\n{}", self.recent.join("\n")));
        }
        if !self.key_outputs.is_empty() {
            let kv: Vec<String> = self
                .key_outputs
                .iter()
                .map(|(k, v)| format!("  {k}: {}", truncate(v, 200)))
                .collect();
            parts.push(format!("Key outputs (reuse these):\n{}", kv.join("\n")));
        }
        if let Some(ref r) = self.last_reflection {
            parts.push(format!("Last reflection: {r}"));
        }
        if let Some(ref e) = self.last_error {
            let loop_warning = if self.is_looping() {
                " [LOOP DETECTED — try a different approach]"
            } else {
                ""
            };
            parts.push(format!("Last error (fix this){loop_warning}: {e}"));
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("\n\n{}", parts.join("\n\n"))
        }
    }
}

// ---------------------------------------------------------------------------
// Git context (read-only injection)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct GitContext {
    initialized: bool,
    current_branch: String,
    has_staged: bool,
    has_unstaged: bool,
}

impl GitContext {
    fn read() -> Self {
        let cwd = match std::env::current_dir() {
            Ok(d) => d,
            Err(_) => return Self::default(),
        };
        let repo = match git2::Repository::open(&cwd) {
            Ok(r) => r,
            Err(_) => return Self::default(),
        };

        let branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()))
            .unwrap_or_else(|| "(detached)".to_string());

        let (mut staged, mut unstaged) = (false, false);
        if let Ok(statuses) = repo.statuses(None) {
            for entry in statuses.iter() {
                let s = entry.status();
                if s.intersects(
                    git2::Status::INDEX_NEW
                        | git2::Status::INDEX_MODIFIED
                        | git2::Status::INDEX_DELETED,
                ) {
                    staged = true;
                }
                if s.intersects(
                    git2::Status::WT_MODIFIED | git2::Status::WT_NEW | git2::Status::WT_DELETED,
                ) {
                    unstaged = true;
                }
            }
        }

        Self {
            initialized: true,
            current_branch: branch,
            has_staged: staged,
            has_unstaged: unstaged,
        }
    }

    fn render(&self) -> String {
        if !self.initialized {
            return "\n\nGit: no repository present.".to_string();
        }
        format!(
            "\n\nGit state (read-only):\
             \n  branch: {}\
             \n  staged: {}\
             \n  unstaged: {}",
            self.current_branch,
            if self.has_staged { "yes" } else { "no" },
            if self.has_unstaged { "yes" } else { "no" },
        )
    }
}

// ---------------------------------------------------------------------------
// Interrupt flag
// ---------------------------------------------------------------------------

/// Returns a future that resolves when Ctrl+C is pressed.
/// On platforms where signal handling is unavailable, the future never resolves.
async fn wait_for_interrupt() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut s) = signal(SignalKind::interrupt()) {
            s.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    }
    #[cfg(not(unix))]
    {
        if tokio::signal::ctrl_c().await.is_ok() {
            // resolved
        } else {
            std::future::pending::<()>().await;
        }
    }
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

pub struct Executor {
    provider: Arc<dyn LlmProvider>,
}

impl Executor {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    fn sampling(&self, ctx: &RunContext) -> SamplingOptions {
        if ctx.deterministic {
            SamplingOptions::deterministic()
        } else {
            SamplingOptions::standard()
        }
    }

    pub async fn execute(
        &self,
        plan: &Plan,
        ctx: &RunContext,
        skills: &SkillSet,
        max_retries: usize,
        context_win: usize,
        reporter: Option<&dyn ProgressReporter>,
    ) -> Result<(ExecutionOutput, Vec<SkillUsage>, bool)> {
        info!(
            agent = "executor",
            steps = plan.steps.len(),
            deterministic = ctx.deterministic,
            "beginning iterative execution"
        );

        let opts = self.sampling(ctx);
        let catalogue = build_catalogue(skills);
        let constraints = build_constraint_note(ctx);
        let dry_run_note = if ctx.dry_run { DRY_RUN_NOTE } else { "" };
        let project_note = build_project_note(ctx);
        let git_ctx = GitContext::read();
        let git_note = git_ctx.render();

        let mut results: Vec<StepResult> = Vec::new();
        let mut skill_log: Vec<SkillUsage> = Vec::new();
        let mut total_calls = 0usize;
        let mut iteration = 0usize;
        let mut total_retries = 0usize;
        let mut token_usage = TokenUsage::default();
        let mut exec_ctx = ExecContext::new(context_win);
        let mut interrupted = false;

        // Interrupt watcher — resolved once by Ctrl+C.
        let interrupt = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let interrupt_clone = interrupt.clone();
        tokio::spawn(async move {
            wait_for_interrupt().await;
            interrupt_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            warn!(
                executor = "interrupt",
                "Ctrl+C received; stopping after current step"
            );
        });

        // Respect Plan::execution_order() for dependency-aware execution.
        let ordered_steps = plan.execution_order();

        if let Some(r) = reporter {
            r.on_execution_start();
        }

        'steps: for step in ordered_steps {
            // Check interrupt before starting each step.
            if interrupt.load(std::sync::atomic::Ordering::SeqCst) {
                warn!(
                    executor = "interrupt",
                    step = step.id,
                    "stopping before step"
                );
                interrupted = true;
                break 'steps;
            }

            info!(agent = "executor", step = step.id, task = %step.task, "executing step");

            let mut step_retries = 0usize;
            let mut step_iter_count = 0usize;
            let mut step_output_acc: Vec<String> = Vec::new();

            'iter: loop {
                iteration += 1;
                step_iter_count += 1;

                if iteration > ctx.max_iterations {
                    warn!(agent = "executor", "max_iterations reached");
                    break 'iter;
                }

                // Token limit check.
                if ctx.max_tokens > 0 && token_usage.total() >= ctx.max_tokens {
                    warn!(
                        agent = "executor",
                        used = token_usage.total(),
                        limit = ctx.max_tokens,
                        "token limit reached"
                    );
                    return Err(MaunsError::LimitExceeded(format!(
                        "token limit ({}) reached after {} tokens",
                        ctx.max_tokens,
                        token_usage.total()
                    )));
                }

                let ctx_rendered = exec_ctx.render();

                let prompt = build_action_prompt(
                    &plan.task,
                    step.id,
                    &step.task,
                    &ctx_rendered,
                    &catalogue,
                    &constraints,
                    dry_run_note,
                    &project_note,
                    &git_note,
                    iteration,
                    ctx.max_iterations,
                );
                token_usage.add_prompt(&prompt);

                let actions = match self.call_and_parse(&prompt, &opts, MAX_PARSE_RETRIES).await {
                    Ok(a) => {
                        exec_ctx.clear_error();
                        a
                    }
                    Err(e) => {
                        step_retries += 1;
                        total_retries += 1;
                        if step_retries > max_retries {
                            return Err(MaunsError::Agent {
                                agent: "executor".to_string(),
                                message: format!(
                                    "step {} failed after {max_retries} retries: {e}",
                                    step.id
                                ),
                            });
                        }
                        let msg = e.to_string();
                        let loop_strategy = if exec_ctx.is_looping() {
                            " Try a completely different approach."
                        } else {
                            ""
                        };
                        exec_ctx.set_error(format!("{msg}{loop_strategy}"));
                        warn!(
                            agent = "executor",
                            step = step.id,
                            retry = step_retries,
                            looping = exec_ctx.is_looping(),
                            "step retry"
                        );
                        continue 'iter;
                    }
                };

                let mut step_done = false;
                let mut had_failure = false;
                let mut iter_parts: Vec<String> = Vec::new();

                for action in actions {
                    match action {
                        AgentAction::Done { ref summary } => {
                            info!(agent = "executor", step = step.id, "Done received");
                            iter_parts.push(format!("[done] {summary}"));
                            // Store the summary as a key output for future steps.
                            exec_ctx.store_key_output(
                                format!("step_{}_result", step.id),
                                summary.clone(),
                            );
                            step_done = true;
                            break;
                        }

                        AgentAction::Note { ref message } => {
                            info!(agent = "executor", note = %message);
                            iter_parts.push(format!("[note] {message}"));
                        }

                        AgentAction::Skill {
                            ref name,
                            ref input,
                        } => {
                            total_calls += 1;
                            if total_calls > MAX_SKILL_CALLS {
                                return Err(MaunsError::LimitExceeded(format!(
                                    "skill calls ({total_calls}) exceeded limit ({MAX_SKILL_CALLS})"
                                )));
                            }

                            let out = match skills.dispatch(name) {
                                Ok(skill) => {
                                    skill
                                        .execute(SkillInput {
                                            params: input.clone(),
                                        })
                                        .await
                                }
                                Err(e) => Err(e),
                            };

                            let usage = record_usage(name, &out);
                            let feedback = format_feedback(name, &out);

                            if !usage.success {
                                had_failure = true;
                                exec_ctx
                                    .set_error(format!("skill {name} failed: {}", usage.message));
                            } else {
                                // Store successful skill outputs as key outputs.
                                exec_ctx.store_key_output(
                                    format!("skill_{name}_last"),
                                    truncate(&feedback, 300).to_string(),
                                );
                            }

                            // Track approximate completion tokens from skill output.
                            token_usage.add_completion(&feedback);

                            skill_log.push(usage);
                            exec_ctx.push(format!("skill:{name} => {}", truncate(&feedback, 200)));
                            iter_parts.push(feedback);
                        }
                    }
                }

                let iter_output = iter_parts.join("\n");
                step_output_acc.push(iter_output.clone());
                exec_ctx.push(format!(
                    "step:{} iter:{step_iter_count} => {}",
                    step.id,
                    truncate(&iter_output, 300)
                ));

                // Reflection step (not in vibe mode).
                if !step_done && !ctx.vibe_mode {
                    let reflection = self
                        .reflect(&plan.task, step.id, &iter_output, had_failure, &opts)
                        .await
                        .unwrap_or_else(|_| "(reflection unavailable)".to_string());
                    token_usage.add_completion(&reflection);
                    exec_ctx.set_reflection(reflection);
                }

                if step_done {
                    if let Some(r) = reporter {
                        r.on_step_complete(step.id, &step.task);
                    }
                    break 'iter;
                }
                if ctx.vibe_mode || !had_failure {
                    if let Some(r) = reporter {
                        r.on_step_complete(step.id, &step.task);
                    }
                    break 'iter;
                }
            }

            let final_output = step_output_acc.join("\n---\n");
            results.push(StepResult {
                step: step.clone(),
                output: final_output,
                retries_used: step_retries,
            });
        }

        let summary = self.summarize(plan, &results, &opts).await?;
        token_usage.add_prompt("summarize");
        token_usage.add_completion(&summary);

        info!(
            agent = "executor",
            iters = iteration,
            retries = total_retries,
            tokens = token_usage.total(),
            interrupted,
            "execution complete"
        );

        Ok((
            ExecutionOutput {
                task: plan.task.clone(),
                results,
                summary,
                iterations: iteration,
                total_retries,
                token_usage,
            },
            skill_log,
            interrupted,
        ))
    }

    // -----------------------------------------------------------------------
    // Reflection step
    // -----------------------------------------------------------------------

    async fn reflect(
        &self,
        task: &str,
        step_id: usize,
        step_output: &str,
        had_failure: bool,
        opts: &SamplingOptions,
    ) -> Result<String> {
        let fail_note = if had_failure {
            " (one or more skill calls failed)"
        } else {
            ""
        };

        let prompt = format!(
            "Reflect on this task execution step{fail_note}. \
             2-3 sentences, plain text, no JSON.\n\n\
             Task: {task}\nStep {step_id} output:\n{step_output}\n\n\
             Answer: 1) What succeeded? 2) What failed/is uncertain? 3) What must be done next?"
        );

        self.provider
            .send_prompt_with_options(&prompt, opts)
            .await
            .map_err(|e| MaunsError::Agent {
                agent: "executor".to_string(),
                message: format!("reflection failed: {e}"),
            })
    }

    // -----------------------------------------------------------------------
    // Call + parse with retry
    // -----------------------------------------------------------------------

    async fn call_and_parse(
        &self,
        prompt: &str,
        opts: &SamplingOptions,
        retries: usize,
    ) -> Result<Vec<AgentAction>> {
        let mut last_err = String::new();
        for attempt in 0..=retries {
            let raw = self
                .provider
                .send_prompt_with_options(prompt, opts)
                .await
                .map_err(|e| MaunsError::Agent {
                    agent: "executor".to_string(),
                    message: format!("LLM call failed: {e}"),
                })?;

            match parse_actions(&raw) {
                Ok(a) if !a.is_empty() => return Ok(a),
                Ok(_) => {
                    last_err = format!("empty action list on attempt {attempt}");
                    warn!(agent = "executor", attempt, "empty action list; retrying");
                }
                Err(e) => {
                    last_err = e.clone();
                    warn!(agent = "executor", attempt, error = %e, "parse error; retrying");
                }
            }
        }
        Err(MaunsError::Agent {
            agent: "executor".to_string(),
            message: format!("parse failed after {retries} retries: {last_err}"),
        })
    }

    async fn summarize(
        &self,
        plan: &Plan,
        results: &[StepResult],
        opts: &SamplingOptions,
    ) -> Result<String> {
        let steps_text: String = results
            .iter()
            .map(|r| format!("Step {}: {}\nOutput: {}", r.step.id, r.step.task, r.output))
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            "Summarize what was accomplished in 2-3 sentences. Plain prose, no markdown.\n\n\
             Task: {}\n\nCompleted steps:\n{steps_text}",
            plan.task
        );

        self.provider
            .send_prompt_with_options(&prompt, opts)
            .await
            .map_err(|e| MaunsError::Agent {
                agent: "executor".to_string(),
                message: format!("summary failed: {e}"),
            })
    }
}

// ---------------------------------------------------------------------------
// Prompt construction
// ---------------------------------------------------------------------------

const DRY_RUN_NOTE: &str =
    "\n\nDRY-RUN: emit only Note and Done actions. Do NOT invoke mutating skills.";

fn build_catalogue(skills: &SkillSet) -> String {
    if skills.is_empty() {
        return String::new();
    }
    let mut s = String::from("\n\nAvailable skills (use the exact name):\n");
    for (name, desc) in skills.catalogue() {
        s.push_str(&format!("  {name}: {desc}\n"));
    }
    s
}

fn build_constraint_note(ctx: &RunContext) -> String {
    if ctx.agents_policy.raw.is_empty() {
        return String::new();
    }
    format!(
        "\n\nConstraints (advisory; MUST NOT override safety rules):\n{}",
        ctx.agents_policy.raw
    )
}

fn build_project_note(ctx: &RunContext) -> String {
    if ctx.project.context_hint.is_empty() {
        return String::new();
    }
    format!("\n\nProject: {}", ctx.project.context_hint)
}

#[allow(clippy::too_many_arguments)]
fn build_action_prompt(
    task: &str,
    step_id: usize,
    step_task: &str,
    context: &str,
    catalogue: &str,
    constraints: &str,
    dry_run: &str,
    project: &str,
    git: &str,
    iteration: usize,
    max_iter: usize,
) -> String {
    format!(
        "Task executor — iteration {iteration}/{max_iter}.\n\
         Emit JSON objects one per line:\n\
         \n  Skill:    {{\"type\":\"skill\",\"name\":\"<n>\",\"input\":{{...}}}}\
         \n  Note:     {{\"type\":\"note\",\"message\":\"<text>\"}}\
         \n  Complete: {{\"type\":\"done\",\"summary\":\"<summary>\"}}\
         \n\nOutput ONLY valid JSON, one object per line. No prose.\
         {catalogue}{constraints}{dry_run}{project}{git}{context}\
         \n\nTask: {task}\nCurrent step {step_id}: {step_task}"
    )
}

// ---------------------------------------------------------------------------
// Action parsing
// ---------------------------------------------------------------------------

fn parse_actions(raw: &str) -> std::result::Result<Vec<AgentAction>, String> {
    let mut actions = Vec::new();
    let mut errors = Vec::new();

    for (i, line) in raw.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        match serde_json::from_str::<AgentAction>(t) {
            Ok(a) => actions.push(a),
            Err(e) => errors.push(format!("line {}: {e}", i + 1)),
        }
    }

    if actions.is_empty() && !errors.is_empty() {
        return Err(errors.join("; "));
    }
    if !errors.is_empty() {
        for e in &errors {
            warn!(executor = "parse", error = %e, "skipping unparseable line");
        }
    }
    Ok(actions)
}

// ---------------------------------------------------------------------------
// Skill feedback + usage
// ---------------------------------------------------------------------------

fn format_feedback(name: &str, out: &Result<SkillOutput>) -> String {
    match out {
        Ok(o) if o.success => {
            format!("[skill:{name}:ok] {}", truncate(&o.data.to_string(), 400))
        }
        Ok(o) => format!("[skill:{name}:fail] {}", o.message),
        Err(e) => format!("[skill:{name}:err] {e}"),
    }
}

fn record_usage(name: &str, out: &Result<SkillOutput>) -> SkillUsage {
    let (success, message) = match out {
        Ok(o) => (o.success, o.message.clone()),
        Err(e) => (false, e.to_string()),
    };
    SkillUsage {
        skill_name: name.to_string(),
        timestamp: chrono::Utc::now(),
        success,
        message,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut b = max;
    while !s.is_char_boundary(b) {
        b -= 1;
    }
    &s[..b]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_context_compresses_old_entries() {
        let mut ctx = ExecContext::new(2);
        ctx.push("entry1");
        ctx.push("entry2");
        ctx.push("entry3");
        assert!(ctx.summary.contains("entry1"));
        assert_eq!(ctx.recent.len(), 2);
    }

    #[test]
    fn exec_context_detects_loop() {
        let mut ctx = ExecContext::new(6);
        ctx.set_error("same error");
        ctx.set_error("same error");
        ctx.set_error("same error");
        assert!(ctx.is_looping());
    }

    #[test]
    fn exec_context_no_loop_on_different_errors() {
        let mut ctx = ExecContext::new(6);
        ctx.set_error("error A");
        ctx.set_error("error B");
        ctx.set_error("error C");
        assert!(!ctx.is_looping());
    }

    #[test]
    fn exec_context_stores_key_output() {
        let mut ctx = ExecContext::new(6);
        ctx.store_key_output("result", "hello");
        let rendered = ctx.render();
        assert!(rendered.contains("result"));
        assert!(rendered.contains("hello"));
    }

    #[test]
    fn exec_context_replaces_key_output() {
        let mut ctx = ExecContext::new(6);
        ctx.store_key_output("key", "v1");
        ctx.store_key_output("key", "v2");
        assert_eq!(ctx.key_outputs.len(), 1);
        assert_eq!(ctx.key_outputs[0].1, "v2");
    }

    #[test]
    fn loop_warning_appears_in_render() {
        let mut ctx = ExecContext::new(6);
        ctx.set_error("boom");
        ctx.set_error("boom");
        ctx.set_error("boom");
        let r = ctx.render();
        assert!(r.contains("LOOP DETECTED"));
    }

    #[test]
    fn parse_actions_accepts_all_variants() {
        let raw = concat!(
            "{\"type\":\"note\",\"message\":\"hi\"}\n",
            "{\"type\":\"done\",\"summary\":\"done\"}\n",
            "{\"type\":\"skill\",\"name\":\"x\",\"input\":{}}\n"
        );
        let a = parse_actions(raw).unwrap();
        assert_eq!(a.len(), 3);
    }

    #[test]
    fn sampling_deterministic_is_zero_temp() {
        let opts = SamplingOptions::deterministic();
        assert_eq!(opts.temperature, 0.0);
        assert_eq!(opts.top_p, 1.0);
    }

    #[test]
    fn token_usage_estimate_reasonable() {
        let t = TokenUsage::estimate("hello world this is a test");
        assert!(t >= 1);
    }
}
