//! FileWriteSkill — write content to a workspace file.
//!
//! Input params:
//!   { "path": "<relative path>", "content": "<text to write>" }
//!
//! Output data:
//!   { "written": true }
//!
//! Writes are blocked in dry-run mode.

use std::sync::Arc;

use async_trait::async_trait;
use mauns_core::{
    error::{MaunsError, Result},
    types::{SkillInput, SkillOutput},
};
use mauns_filesystem::PathGuard;
use tracing::{debug, warn};

use crate::skill::AgentSkill;

pub struct FileWriteSkill {
    guard:   Arc<PathGuard>,
    dry_run: bool,
}

impl FileWriteSkill {
    pub fn new(guard: Arc<PathGuard>) -> Self {
        Self { guard, dry_run: false }
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
}

#[async_trait]
impl AgentSkill for FileWriteSkill {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write text content to a file inside the workspace. \
         Creates the file (and parent directories) if they do not exist. \
         Input: { \"path\": \"<relative path>\", \"content\": \"<text>\" }. \
         Output: { \"written\": true }."
    }

    async fn execute(&self, input: SkillInput) -> Result<SkillOutput> {
        let (path, content) = extract_params(&input)?;
        debug!(skill = "file_write", path = %path);

        let safe = self.guard.validate(&path)?;

        if self.dry_run {
            warn!(skill = "file_write", path = %path, "dry-run: write suppressed");
            return Ok(SkillOutput::ok(serde_json::json!({
                "written": false,
                "dry_run": true
            })));
        }

        if let Some(parent) = safe.as_path().parent() {
            std::fs::create_dir_all(parent).map_err(|e| MaunsError::Skill {
                name:    "file_write".to_string(),
                message: format!("cannot create directories for '{}': {e}", path),
            })?;
        }

        std::fs::write(safe.as_path(), &content).map_err(|e| MaunsError::Skill {
            name:    "file_write".to_string(),
            message: format!("cannot write '{}': {e}", path),
        })?;

        Ok(SkillOutput::ok(serde_json::json!({ "written": true })))
    }
}

fn extract_params(input: &SkillInput) -> Result<(String, String)> {
    let path = input
        .params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| MaunsError::InvalidAction(
            "file_write requires a 'path' string parameter".to_string(),
        ))?
        .to_string();

    let content = input
        .params
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| MaunsError::InvalidAction(
            "file_write requires a 'content' string parameter".to_string(),
        ))?
        .to_string();

    Ok((path, content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn make_guard() -> Arc<PathGuard> {
        Arc::new(PathGuard::new(env::current_dir().unwrap()).unwrap())
    }

    #[tokio::test]
    async fn rejects_missing_content() {
        let skill = FileWriteSkill::new(make_guard());
        let input = SkillInput { params: serde_json::json!({ "path": "x.txt" }) };
        let err = skill.execute(input).await.unwrap_err();
        assert!(matches!(err, MaunsError::InvalidAction(_)));
    }

    #[tokio::test]
    async fn dry_run_does_not_write() {
        let skill = FileWriteSkill::new(make_guard()).with_dry_run(true);
        let input = SkillInput {
            params: serde_json::json!({ "path": "dry_test.txt", "content": "hello" }),
        };
        let out = skill.execute(input).await.unwrap();
        assert!(out.success);
        assert_eq!(out.data["dry_run"], true);
        assert!(!std::path::Path::new("dry_test.txt").exists());
    }
}
