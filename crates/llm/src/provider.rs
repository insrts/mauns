use async_trait::async_trait;
use mauns_core::error::Result;

/// Options controlling LLM sampling behaviour.
#[derive(Debug, Clone)]
pub struct SamplingOptions {
    /// Temperature. 0.0 = deterministic, 1.0 = default creative.
    pub temperature: f32,
    /// Top-p nucleus sampling (0.0–1.0).
    pub top_p: f32,
}

impl SamplingOptions {
    /// Standard options: mild creativity suitable for agent tasks.
    pub fn standard() -> Self {
        Self {
            temperature: 0.2,
            top_p: 0.95,
        }
    }

    /// Fully deterministic: temperature 0, top_p 1.
    /// Produces stable, reproducible outputs.
    pub fn deterministic() -> Self {
        Self {
            temperature: 0.0,
            top_p: 1.0,
        }
    }
}

/// The central abstraction for all LLM backends.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a plain-text prompt and receive a plain-text response.
    async fn send_prompt(&self, input: &str) -> Result<String>;

    /// Send a prompt with explicit sampling options.
    /// Default implementation delegates to `send_prompt` (ignores options).
    async fn send_prompt_with_options(
        &self,
        input: &str,
        _opts: &SamplingOptions,
    ) -> Result<String> {
        self.send_prompt(input).await
    }

    /// Human-readable provider name.
    fn name(&self) -> &str;
}
