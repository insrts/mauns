//! `DeterministicProvider` wraps any `LlmProvider` and enforces
//! `SamplingOptions::deterministic()` (temperature=0, top_p=1) on every call.
//!
//! Used when `--deterministic` is set. Guarantees reproducible outputs
//! regardless of provider defaults.

use std::sync::Arc;

use async_trait::async_trait;
use mauns_core::error::Result;

use crate::provider::{LlmProvider, SamplingOptions};

/// Wraps any provider and forces deterministic sampling on every call.
pub struct DeterministicProvider {
    inner: Arc<dyn LlmProvider>,
    opts:  SamplingOptions,
}

impl DeterministicProvider {
    pub fn new(inner: Arc<dyn LlmProvider>) -> Self {
        Self {
            inner,
            opts: SamplingOptions::deterministic(),
        }
    }
}

#[async_trait]
impl LlmProvider for DeterministicProvider {
    async fn send_prompt(&self, input: &str) -> Result<String> {
        self.inner.send_prompt_with_options(input, &self.opts).await
    }

    async fn send_prompt_with_options(
        &self,
        input: &str,
        _opts: &SamplingOptions,
    ) -> Result<String> {
        // Caller-supplied options are ignored — deterministic always wins.
        self.inner.send_prompt_with_options(input, &self.opts).await
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}
