pub mod anthropic;
pub mod deterministic;
pub mod groq;
pub mod openai;
pub mod provider;
pub mod registry;

pub use deterministic::DeterministicProvider;
pub use groq::{GroqProvider, GROQ_MODELS};
pub use provider::{LlmProvider, SamplingOptions};
pub use registry::{build_provider, build_provider_with_model, ProviderKind};

/// All available models for a given provider kind.
pub fn models_for_provider(kind: &ProviderKind) -> Vec<(&'static str, &'static str)> {
    use registry::ProviderKind::*;
    match kind {
        OpenAi => vec![
            ("gpt-4o", "GPT-4o — flagship multimodal"),
            ("gpt-4o-mini", "GPT-4o Mini — fast and cheap"),
            ("gpt-4-turbo", "GPT-4 Turbo — 128k context"),
            ("gpt-3.5-turbo", "GPT-3.5 Turbo — fastest/cheapest"),
        ],
        Anthropic => vec![
            ("claude-3-7-sonnet-20250219", "Claude 3.7 Sonnet — flagship"),
            ("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet — stable"),
            ("claude-3-5-haiku-20241022", "Claude 3.5 Haiku — fast"),
            ("claude-3-opus-20240229", "Claude 3 Opus — legacy"),
        ],
        Groq => GROQ_MODELS.to_vec(),
    }
}
