use thiserror::Error;

#[derive(Debug, Error)]
pub enum MaunsError {
    #[error("LLM provider error: {0}")]
    LlmProvider(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Agent error in {agent}: {message}")]
    Agent { agent: String, message: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Task execution failed: {0}")]
    Execution(String),

    #[error("Verification failed: {0}")]
    Verification(String),

    #[error("Filesystem error: {0}")]
    Filesystem(String),

    #[error("Path traversal attempt blocked: {0}")]
    PathTraversal(String),

    #[error("Path is outside workspace root: {path}")]
    OutsideWorkspace { path: String },

    #[error("Access to restricted path denied: {0}")]
    RestrictedPath(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(String),

    #[error("Dry-run mode is active; no writes will be performed")]
    DryRun,

    #[error("Git error: {0}")]
    Git(String),

    #[error("GitHub API error: {0}")]
    GitHub(String),

    #[error("User aborted the operation")]
    Aborted,

    #[error("Provider '{0}' is not valid; must be 'openai' or 'anthropic'")]
    InvalidProvider(String),

    // Phase 4
    #[error("Skill error in '{name}': {message}")]
    Skill { name: String, message: String },

    #[error("Skill '{0}' not found in registry")]
    SkillNotFound(String),

    #[error("Plugin error in '{name}': {message}")]
    Plugin { name: String, message: String },

    #[error("Action schema validation failed: {0}")]
    InvalidAction(String),

    #[error("Execution limit exceeded: {0}")]
    LimitExceeded(String),
}

pub type Result<T> = std::result::Result<T, MaunsError>;
