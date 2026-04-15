use async_trait::async_trait;
use mauns_core::error::{MaunsError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::provider::{LlmProvider, SamplingOptions};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_MODEL: &str = "gpt-4o";

#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>, model: Option<impl Into<String>>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model
                .map(|m| m.into())
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            client: Client::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    async fn call(&self, input: &str, opts: &SamplingOptions) -> Result<String> {
        debug!(provider = "openai", model = %self.model, temperature = opts.temperature);

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
            .post(OPENAI_API_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| MaunsError::LlmProvider(format!("openai request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(MaunsError::LlmProvider(format!(
                "openai returned {status}: {text}"
            )));
        }

        let parsed: ChatResponse = response
            .json()
            .await
            .map_err(|e| MaunsError::LlmProvider(format!("openai parse error: {e}")))?;

        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| MaunsError::LlmProvider("openai returned no choices".to_string()))
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

#[async_trait]
impl LlmProvider for OpenAiProvider {
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
        "openai"
    }
}
