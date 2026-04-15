use std::sync::Arc;

use mauns_config::schema::MaunsConfig;
use mauns_core::error::{MaunsError, Result};

use crate::{
    anthropic::AnthropicProvider, groq::GroqProvider, openai::OpenAiProvider, provider::LlmProvider,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Groq,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "openai",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Groq => "groq",
        }
    }

    pub fn all() -> &'static [ProviderKind] {
        &[
            ProviderKind::OpenAi,
            ProviderKind::Anthropic,
            ProviderKind::Groq,
        ]
    }
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ProviderKind {
    type Err = MaunsError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ProviderKind::OpenAi),
            "anthropic" | "claude" => Ok(ProviderKind::Anthropic),
            "groq" => Ok(ProviderKind::Groq),
            other => Err(MaunsError::InvalidProvider(other.to_string())),
        }
    }
}

/// Build a provider, optionally overriding the model.
pub fn build_provider(kind: &ProviderKind, config: &MaunsConfig) -> Result<Arc<dyn LlmProvider>> {
    build_provider_with_model(kind, config, None)
}

/// Build a provider with an explicit model override.
pub fn build_provider_with_model(
    kind: &ProviderKind,
    config: &MaunsConfig,
    model: Option<&str>,
) -> Result<Arc<dyn LlmProvider>> {
    match kind {
        ProviderKind::OpenAi => {
            let key = config.openai.api_key.clone();
            let p = OpenAiProvider::new(key, model.map(|s| s.to_string()));
            Ok(Arc::new(p))
        }
        ProviderKind::Anthropic => {
            let key = config.claude.api_key.clone();
            let p = AnthropicProvider::new(key, model.map(|s| s.to_string()));
            Ok(Arc::new(p))
        }
        ProviderKind::Groq => {
            let key = config.groq.api_key.clone();
            let p = GroqProvider::new(key, model.map(|s| s.to_string()));
            Ok(Arc::new(p))
        }
    }
}
