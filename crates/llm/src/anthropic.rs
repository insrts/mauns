use async_trait::async_trait;
use mauns_core::error::{MaunsError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::provider::{LlmProvider, SamplingOptions};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL:     &str = "claude-sonnet-4-20250514";
const MAX_TOKENS:        u32  = 4096;

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    api_key: String,
    model:   String,
    client:  Client,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model:   DEFAULT_MODEL.to_string(),
            client:  Client::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    async fn call(&self, input: &str, opts: &SamplingOptions) -> Result<String> {
        debug!(provider = "anthropic", model = %self.model, temperature = opts.temperature);

        let body = MessagesRequest {
            model:       &self.model,
            max_tokens:  MAX_TOKENS,
            temperature: opts.temperature,
            top_p:       opts.top_p,
            messages:    vec![Message { role: "user", content: input }],
        };

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await
            .map_err(|e| MaunsError::LlmProvider(format!("anthropic request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(MaunsError::LlmProvider(format!("anthropic returned {status}: {text}")));
        }

        let parsed: MessagesResponse = response
            .json()
            .await
            .map_err(|e| MaunsError::LlmProvider(format!("anthropic parse error: {e}")))?;

        parsed
            .content
            .into_iter()
            .find(|b| b.block_type == "text")
            .and_then(|b| b.text)
            .ok_or_else(|| MaunsError::LlmProvider("anthropic returned no text block".to_string()))
    }
}

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model:       &'a str,
    max_tokens:  u32,
    temperature: f32,
    top_p:       f32,
    messages:    Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role:    &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text:       Option<String>,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn send_prompt(&self, input: &str) -> Result<String> {
        self.call(input, &SamplingOptions::standard()).await
    }

    async fn send_prompt_with_options(&self, input: &str, opts: &SamplingOptions) -> Result<String> {
        self.call(input, opts).await
    }

    fn name(&self) -> &str { "anthropic" }
}
