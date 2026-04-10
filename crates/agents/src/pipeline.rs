use std::sync::Arc;

use mauns_core::{
    error::{MaunsError, Result},
    types::{ProgressReporter, RunContext, TaskReport},
};
use mauns_filesystem::{Filesystem, PathGuard};
use mauns_llm::provider::LlmProvider;
use mauns_skills::{builtin, AgentSkill};
use tracing::{info, warn};

use crate::{
    confirmation::confirm_changes,
    executor::Executor,
    git_orchestrator::{run_git_workflow, GitConfig},
    planner::Planner,
    verifier::Verifier,
};

pub struct Pipeline {
    planner: Planner,
    executor: Executor,
    verifier: Verifier,
    git_config: GitConfig,
    extra_skills: Vec<Arc<dyn AgentSkill>>,
}

impl Pipeline {
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        git_cfg: GitConfig,
        extra_skills: Vec<Arc<dyn AgentSkill>>,
    ) -> Self {
        Self {
            planner: Planner::new(Arc::clone(&provider)),
            executor: Executor::new(Arc::clone(&provider)),
            verifier: Verifier::new(Arc::clone(&provider)),
            git_config: git_cfg,
            extra_skills,
        }
    }

    pub async fn run(
        &self,
        task: &str,
        ctx: &RunContext,
        reporter: Option<&dyn ProgressReporter>,
    ) -> Result<TaskReport> {
        info!(
            pipeline      = "start",
            task          = %task,
            dry_run       = ctx.dry_run,
            vibe          = ctx.vibe_mode,
            deterministic = ctx.deterministic,
            max_iter      = ctx.max_iterations,
            max_retries   = ctx.max_retries,
        );

        let cwd = std::env::current_dir()
            .map_err(|e| MaunsError::Filesystem(format!("cannot get cwd: {e}")))?;
        let guard = Arc::new(PathGuard::new(&cwd)?);

        let mut skill_set = builtin::default_skillset(Arc::clone(&guard), ctx.dry_run);
        for s in &self.extra_skills {
            skill_set = skill_set.with_skill(Arc::clone(s));
        }
        info!(pipeline = "skills", count = skill_set.len());

        let fs = Filesystem::new(ctx.dry_run)?;

        let plan = self.planner.plan(task, ctx, reporter).await?;
        info!(pipeline = "planner", steps = plan.steps.len());

        let (execution, skill_log, interrupted) = self
            .executor
            .execute(
                &plan,
                ctx,
                &skill_set,
                ctx.max_retries,
                ctx.context_window,
                reporter,
            )
            .await?;

        info!(
            pipeline = "executor",
            iterations = execution.iterations,
            retries = execution.total_retries,
            tokens = execution.token_usage.total(),
            interrupted,
        );

        // Skip verification on interrupt — return partial results immediately.
        let verification = if interrupted {
            warn!(pipeline = "verifier", "skipping verification (interrupted)");
            mauns_core::types::VerificationReport {
                passed: false,
                feedback: "Run was interrupted before completion.".to_string(),
                retry_suggested: true,
            }
        } else {
            let v = self.verifier.verify(&execution).await?;
            info!(pipeline = "verifier", passed = v.passed);
            v
        };

        let change_log = fs.into_changes();

        if ctx.confirm_writes && !ctx.vibe_mode && !interrupted {
            confirm_changes(&change_log, ctx.dry_run).map_err(|e| match e {
                MaunsError::Aborted => MaunsError::Aborted,
                other => other,
            })?;
        }

        // Skip git on interrupt or when there is nothing to commit.
        let git_outcome = if interrupted {
            None
        } else {
            run_git_workflow(task, &execution.summary, &change_log, ctx, &self.git_config).await?
        };

        let report = TaskReport {
            task: task.to_string(),
            plan,
            execution,
            verification,
            change_log,
            git_outcome,
            skill_log,
            interrupted,
        };

        if let Some(r) = reporter {
            r.on_result(&report.execution.summary);
        }

        Ok(report)
    }
}
