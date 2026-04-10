//! Core AgentSkill trait.

use async_trait::async_trait;
use mauns_core::{
    error::Result,
    types::{SkillInput, SkillOutput},
};

/// Every skill must implement this trait.
///
/// Skills are statically registered at startup.
/// They receive structured input and return structured output.
/// They MUST NOT execute shell commands or access tokens.
#[async_trait]
pub trait AgentSkill: Send + Sync {
    /// Unique machine-readable name (e.g. `file_read`).
    fn name(&self) -> &str;

    /// Human-readable description used in prompts.
    fn description(&self) -> &str;

    /// Execute the skill with the given input.
    async fn execute(&self, input: SkillInput) -> Result<SkillOutput>;
}
