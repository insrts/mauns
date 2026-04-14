//! Groq provider — uses the OpenAI-compatible chat completions endpoint
//! at api.groq.com.  Groq's inference is significantly faster than hosted
//! models, making it well-suited for agent inner loops.

use async_trait::async_trait;
use mauns_core::error::{MaunsError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::provider::{LlmProvider, SamplingOptions};

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const DEFAULT_MODEL: &str = "llama-3.3-70b-versatile";

/// All Groq models available for selection in the session.
pub const GROQ_MODELS: &[(&str, &str)] = &[
    ("llama-3.3-70b-versatile", "Llama 3.3 70B — balanced, fast"),
    ("llama-3.1-70b-versatile", "Llama 3.1 70B — stable"),
    ("llama-3.1-8b-instant", "Llama 3.1 8B — very fast"),
    ("mixtral-8x7b-32768", "Mixtral 8x7B — 32k context"),
    ("gemma2-9b-it", "Gemma 2 9B — compact"),
    (
        "llama3-groq-70b-8192-tool-use-preview",
        "Llama 3 70B — tool use",
    ),
];

#[derive(Debug, Clone)]
pub struct GroqProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl GroqProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
            client: Client::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    async fn call(&self, input: &str, opts: &SamplingOptions) -> Result<String> {
        debug!(provider = "groq", model = %self.model, temperature = opts.temperature);

        let body = ChatRequest {
            model: &self.model,
            messages: vec![ChatMessage {
                role: "user",
                content: input,
            }],
            temperature: opts.temperature,
            top_p: opts.top_p,
        };

        let response = self
            .client
            .post(GROQ_API_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| MaunsError::LlmProvider(format!("groq request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(MaunsError::LlmProvider(format!("groq {status}: {text}")));
        }

        let parsed: ChatResponse = response
            .json()
            .await
            .map_err(|e| MaunsError::LlmProvider(format!("groq parse error: {e}")))?;

        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| MaunsError::LlmProvider("groq returned no choices".to_string()))
    }
}

#[async_trait]
impl LlmProvider for GroqProvider {
    async fn send_prompt(&self, input: &str) -> Result<String> {
        self.call(input, &SamplingOptions::standard()).await
    }

    async fn send_prompt_with_options(
        &self,
        input: &str,
        opts: &SamplingOptions,
    ) -> Result<String> {
        self.call(input, opts).await
    }

    fn name(&self) -> &str {
        "groq"
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    top_p: f32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}
