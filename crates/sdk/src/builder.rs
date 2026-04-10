use std::sync::Arc;

use mauns_agents::{context_loader::load_run_context, git_orchestrator::GitConfig, Pipeline};
use mauns_config::schema::MaunsConfig;
use mauns_core::{error::Result, types::TaskReport};
use mauns_llm::{
    build_provider, deterministic::DeterministicProvider, registry::ProviderKind, LlmProvider,
};
use mauns_skills::AgentSkill;

pub struct Mauns {
    config: MaunsConfig,
    extra_skills: Vec<Arc<dyn AgentSkill>>,
    deterministic: bool,
    max_tokens: usize,
}

impl Default for Mauns {
    fn default() -> Self {
        let config = mauns_config::load_config().unwrap_or_default();
        Self {
            config,
            extra_skills: Vec::new(),
            deterministic: false,
            max_tokens: 0,
        }
    }
}

impl Mauns {
    pub fn with_config(config: MaunsConfig) -> Self {
        Self {
            config,
            extra_skills: Vec::new(),
            deterministic: false,
            max_tokens: 0,
        }
    }
    pub fn provider(mut self, p: impl Into<String>) -> Self {
        self.config.provider = p.into();
        self
    }
    pub fn dry_run(mut self, v: bool) -> Self {
        self.config.safety.dry_run = v;
        self
    }
    pub fn deterministic(mut self, v: bool) -> Self {
        self.deterministic = v;
        self
    }
    pub fn max_tokens(mut self, v: usize) -> Self {
        self.max_tokens = v;
        self
    }
    pub fn with_skill(mut self, s: Arc<dyn AgentSkill>) -> Self {
        self.extra_skills.push(s);
        self
    }

    pub async fn run_task(&self, task: &str) -> Result<TaskReport> {
        self.config.validate()?;

        let kind: ProviderKind = self.config.provider.parse()?;
        let base = build_provider(&kind)?;

        let provider: Arc<dyn LlmProvider> = if self.deterministic {
            Arc::new(DeterministicProvider::new(base))
        } else {
            base
        };

        let git_cfg = GitConfig::new(self.config.git.create_pr, false);
        let exec = &self.config.execution;

        let ctx = load_run_context(
            self.config.safety.dry_run,
            self.config.safety.confirm_before_write,
            self.deterministic,
            false,
            exec.max_iterations,
            exec.max_retries,
            exec.context_window,
            self.max_tokens,
        );

        Pipeline::new(provider, git_cfg, self.extra_skills.clone())
            .run(task, &ctx, None)
            .await
    }

    pub fn config(&self) -> &MaunsConfig {
        &self.config
    }
}
