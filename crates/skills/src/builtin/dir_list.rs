//! DirListSkill — list entries of a workspace directory.
//!
//! Input params:
//!   { "path": "<relative path>" }
//!
//! Output data:
//!   { "entries": ["file1.rs", "lib.rs", ...] }

use std::sync::Arc;

use async_trait::async_trait;
use mauns_core::{
    error::{MaunsError, Result},
    types::{SkillInput, SkillOutput},
};
use mauns_filesystem::PathGuard;
use tracing::debug;

use crate::skill::AgentSkill;

pub struct DirListSkill {
    guard: Arc<PathGuard>,
}

impl DirListSkill {
    pub fn new(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }
}

#[async_trait]
impl AgentSkill for DirListSkill {
    fn name(&self) -> &str {
        "dir_list"
    }

    fn description(&self) -> &str {
        "List directory entries inside the workspace. \
         Input: { \"path\": \"<relative path>\" }. \
         Output: { \"entries\": [\"<name>\", ...] }."
    }

    async fn execute(&self, input: SkillInput) -> Result<SkillOutput> {
        let path = input
            .params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MaunsError::InvalidAction(
                "dir_list requires a 'path' string parameter".to_string(),
            ))?
            .to_string();

        debug!(skill = "dir_list", path = %path);

        let safe = self.guard.validate(&path)?;

        let read_dir = std::fs::read_dir(safe.as_path()).map_err(|e| MaunsError::Skill {
            name:    "dir_list".to_string(),
            message: format!("cannot read directory '{}': {e}", path),
        })?;

        let mut entries: Vec<String> = Vec::new();
        for entry in read_dir {
            let entry = entry.map_err(|e| MaunsError::Skill {
                name:    "dir_list".to_string(),
                message: format!("directory entry error: {e}"),
            })?;
            entries.push(entry.file_name().to_string_lossy().into_owned());
        }
        entries.sort();

        Ok(SkillOutput::ok(serde_json::json!({ "entries": entries })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn make_guard() -> Arc<PathGuard> {
        Arc::new(PathGuard::new(env::current_dir().unwrap()).unwrap())
    }

    #[tokio::test]
    async fn lists_current_directory() {
        let skill = DirListSkill::new(make_guard());
        let input = SkillInput { params: serde_json::json!({ "path": "." }) };
        let out = skill.execute(input).await.unwrap();
        assert!(out.success);
        let entries = out.data["entries"].as_array().unwrap();
        assert!(!entries.is_empty());
    }

    #[tokio::test]
    async fn rejects_traversal() {
        let skill = DirListSkill::new(make_guard());
        let input = SkillInput { params: serde_json::json!({ "path": "../.." }) };
        let err = skill.execute(input).await.unwrap_err();
        assert!(matches!(err, MaunsError::PathTraversal(_)));
    }
}
