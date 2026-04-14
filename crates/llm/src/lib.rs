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
            ("claude-opus-4-5", "Claude Opus 4.5 — most capable"),
            ("claude-sonnet-4-5", "Claude Sonnet 4.5 — balanced"),
            ("claude-haiku-4-5-20251001", "Claude Haiku 4.5 — fast"),
            ("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet — legacy"),
        ],
        Groq => GROQ_MODELS.to_vec(),
    }
}
