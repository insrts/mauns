use std::sync::Arc;

use mauns_core::error::{MaunsError, Result};

use crate::{anthropic::AnthropicProvider, openai::OpenAiProvider, provider::LlmProvider};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
}

impl std::str::FromStr for ProviderKind {
    type Err = MaunsError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ProviderKind::OpenAi),
            "anthropic" | "claude" => Ok(ProviderKind::Anthropic),
            other => Err(MaunsError::InvalidProvider(other.to_string())),
        }
    }
}

pub fn build_provider(kind: &ProviderKind) -> Result<Arc<dyn LlmProvider>> {
    match kind {
        ProviderKind::OpenAi => {
            let key = std::env::var("OPENAI_API_KEY").map_err(|_| {
                MaunsError::Config("OPENAI_API_KEY environment variable is not set".to_string())
            })?;
            Ok(Arc::new(OpenAiProvider::new(key)))
        }
        ProviderKind::Anthropic => {
            let key = std::env::var("CLAUDE_API_KEY").map_err(|_| {
                MaunsError::Config("CLAUDE_API_KEY environment variable is not set".to_string())
            })?;
            Ok(Arc::new(AnthropicProvider::new(key)))
        }
    }
}
