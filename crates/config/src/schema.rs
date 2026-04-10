use mauns_core::error::{MaunsError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MaunsConfig {
    pub provider: String,
    pub openai: OpenAiConfig,
    pub claude: ClaudeConfig,
    pub safety: SafetyConfig,
    pub logging: LoggingConfig,
    pub git: GitConfig,
    pub execution: ExecutionConfig,
}

impl Default for MaunsConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            openai: OpenAiConfig::default(),
            claude: ClaudeConfig::default(),
            safety: SafetyConfig::default(),
            logging: LoggingConfig::default(),
            git: GitConfig::default(),
            execution: ExecutionConfig::default(),
        }
    }
}

impl MaunsConfig {
    pub fn validate(&self) -> Result<()> {
        match self.provider.to_lowercase().as_str() {
            "openai" | "anthropic" => {}
            other => return Err(MaunsError::InvalidProvider(other.to_string())),
        }
        if self.execution.max_iterations == 0 {
            return Err(MaunsError::Config(
                "execution.max_iterations must be at least 1".to_string(),
            ));
        }
        if self.execution.max_iterations > 100 {
            return Err(MaunsError::Config(
                "execution.max_iterations must not exceed 100".to_string(),
            ));
        }
        if self.execution.max_retries > 10 {
            return Err(MaunsError::Config(
                "execution.max_retries must not exceed 10".to_string(),
            ));
        }
        if self.execution.context_window == 0 {
            return Err(MaunsError::Config(
                "execution.context_window must be at least 1".to_string(),
            ));
        }
        Ok(())
    }

    pub fn default_toml() -> &'static str {
        r#"# Mauns configuration
provider = "anthropic"   # openai | anthropic

[openai]
api_key = ""             # or set OPENAI_API_KEY env var

[claude]
api_key = ""             # or set CLAUDE_API_KEY env var

[safety]
dry_run              = false
confirm_before_write = false

[logging]
level = "info"           # error | warn | info | debug | trace

[git]
create_pr = true

[execution]
max_iterations = 20      # max agent loop iterations per run
max_retries    = 3       # retries per step on failure or bad output
context_window = 6       # recent steps kept in full context
"#
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenAiConfig {
    pub api_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClaudeConfig {
    pub api_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetyConfig {
    pub dry_run: bool,
    pub confirm_before_write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GitConfig {
    pub create_pr: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self { create_pr: true }
    }
}

/// Controls the iterative execution loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecutionConfig {
    /// Maximum number of agent iterations across all steps.
    pub max_iterations: usize,
    /// Maximum retries per step on failure or unparseable output.
    pub max_retries: usize,
    /// Number of recent step/skill outputs kept in full context.
    pub context_window: usize,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            max_retries: 3,
            context_window: 6,
        }
    }
}
