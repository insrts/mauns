use std::path::PathBuf;

use mauns_core::error::{MaunsError, Result};
use tracing::debug;

use crate::schema::MaunsConfig;

pub fn load_config() -> Result<MaunsConfig> {
    let mut config = MaunsConfig::default();

    if let Some(home) = home_dir() {
        let p = home.join(".mauns.toml");
        if p.exists() {
            debug!(config = "loader", path = %p.display(), "loading home config");
            let text = std::fs::read_to_string(&p)
                .map_err(|e| MaunsError::Config(format!("cannot read '{}': {e}", p.display())))?;
            merge_toml(&mut config, &text)?;
        }
    }

    let project = PathBuf::from("mauns.toml");
    if project.exists() {
        debug!(config = "loader", "loading project mauns.toml");
        let text = std::fs::read_to_string(&project)
            .map_err(|e| MaunsError::Config(format!("cannot read mauns.toml: {e}")))?;
        merge_toml(&mut config, &text)?;
    }

    Ok(config)
}

fn merge_toml(base: &mut MaunsConfig, text: &str) -> Result<()> {
    let parsed: MaunsConfig =
        toml::from_str(text).map_err(|e| MaunsError::TomlParse(e.to_string()))?;

    if !parsed.provider.is_empty() {
        base.provider = parsed.provider;
    }
    if !parsed.model.is_empty() {
        base.model = parsed.model;
    }
    if !parsed.openai.api_key.is_empty() {
        base.openai.api_key = parsed.openai.api_key;
    }
    if !parsed.claude.api_key.is_empty() {
        base.claude.api_key = parsed.claude.api_key;
    }
    if !parsed.groq.api_key.is_empty() {
        base.groq.api_key = parsed.groq.api_key;
    }
    if !parsed.git.github_token.is_empty() {
        base.git.github_token = parsed.git.github_token;
    }
    base.safety = parsed.safety;
    base.logging = parsed.logging;
    base.git.create_pr = parsed.git.create_pr;
    base.execution = parsed.execution;
    Ok(())
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}
