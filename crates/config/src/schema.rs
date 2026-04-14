use mauns_core::error::{MaunsError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MaunsConfig {
    pub provider: String,
    /// Optional model override. Empty means use provider default.
    pub model: String,
    pub openai: OpenAiConfig,
    pub claude: ClaudeConfig,
    pub groq: GroqConfig,
    pub safety: SafetyConfig,
    pub logging: LoggingConfig,
    pub git: GitConfig,
    pub execution: ExecutionConfig,
}

impl Default for MaunsConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            model: String::new(),
            openai: OpenAiConfig::default(),
            claude: ClaudeConfig::default(),
            groq: GroqConfig::default(),
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
            "openai" => {
                if self.openai.api_key.is_empty() {
                    return Err(MaunsError::Config("openai.api_key is required".to_string()));
                }
            }
            "anthropic" => {
                if self.claude.api_key.is_empty() {
                    return Err(MaunsError::Config("claude.api_key is required".to_string()));
                }
            }
            "groq" => {
                if self.groq.api_key.is_empty() {
                    return Err(MaunsError::Config("groq.api_key is required".to_string()));
                }
            }
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

    /// Effective model: explicit override > provider default.
    pub fn effective_model(&self) -> Option<&str> {
        if self.model.is_empty() {
            None
        } else {
            Some(&self.model)
        }
    }

    pub fn default_toml() -> &'static str {
        r#"# Mauns configuration
provider = "anthropic"   # openai | anthropic | groq
model    = ""            # leave empty to use provider default

[openai]
api_key = ""

[claude]
api_key = ""

[groq]
api_key = ""

[safety]
dry_run              = false
confirm_before_write = false

[logging]
level = "info"

[git]
create_pr    = true
github_token = ""

[execution]
max_iterations = 20
max_retries    = 3
context_window = 6
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
pub struct GroqConfig {
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
    pub github_token: String,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            create_pr: true,
            github_token: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecutionConfig {
    pub max_iterations: usize,
    pub max_retries: usize,
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
