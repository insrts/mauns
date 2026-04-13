use std::sync::Arc;

use mauns_core::error::{MaunsError, Result};

use crate::{
    anthropic::AnthropicProvider,
    groq::GroqProvider,
    openai::OpenAiProvider,
    provider::LlmProvider,
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
            ProviderKind::OpenAi    => "openai",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Groq      => "groq",
        }
    }

    pub fn all() -> &'static [ProviderKind] {
        &[ProviderKind::OpenAi, ProviderKind::Anthropic, ProviderKind::Groq]
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
            "openai"               => Ok(ProviderKind::OpenAi),
            "anthropic" | "claude" => Ok(ProviderKind::Anthropic),
            "groq"                 => Ok(ProviderKind::Groq),
            other => Err(MaunsError::InvalidProvider(other.to_string())),
        }
    }
}

/// Build a provider, optionally overriding the model.
pub fn build_provider(kind: &ProviderKind) -> Result<Arc<dyn LlmProvider>> {
    build_provider_with_model(kind, None)
}

/// Build a provider with an explicit model override.
pub fn build_provider_with_model(
    kind:  &ProviderKind,
    model: Option<&str>,
) -> Result<Arc<dyn LlmProvider>> {
    match kind {
        ProviderKind::OpenAi => {
            let key = std::env::var("OPENAI_API_KEY").map_err(|_| {
                MaunsError::Config("OPENAI_API_KEY is not set".to_string())
            })?;
            let p = OpenAiProvider::new(key);
            let p = if let Some(m) = model { p.with_model(m) } else { p };
            Ok(Arc::new(p))
        }
        ProviderKind::Anthropic => {
            let key = std::env::var("CLAUDE_API_KEY").map_err(|_| {
                MaunsError::Config("CLAUDE_API_KEY is not set".to_string())
            })?;
            let p = AnthropicProvider::new(key);
            let p = if let Some(m) = model { p.with_model(m) } else { p };
            Ok(Arc::new(p))
        }
        ProviderKind::Groq => {
            let key = std::env::var("GROQ_API_KEY").map_err(|_| {
                MaunsError::Config("GROQ_API_KEY is not set".to_string())
            })?;
            let p = GroqProvider::new(key);
            let p = if let Some(m) = model { p.with_model(m) } else { p };
            Ok(Arc::new(p))
        }
    }
}
