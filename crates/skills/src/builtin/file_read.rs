use std::sync::Arc;

use async_trait::async_trait;
use mauns_core::{
    error::{MaunsError, Result},
    types::{SkillInput, SkillOutput},
};
use mauns_filesystem::PathGuard;
use tracing::debug;

use crate::skill::AgentSkill;

pub struct FileReadSkill {
    guard: Arc<PathGuard>,
}

impl FileReadSkill {
    pub fn new(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }
}

#[async_trait]
impl AgentSkill for FileReadSkill {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the text content of a file inside the workspace (max 1 MiB). \
         Input: {\"path\": \"<relative path>\"}. \
         Output: {\"content\": \"<file text>\"}."
    }

    async fn execute(&self, input: SkillInput) -> Result<SkillOutput> {
        let path = input
            .params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                MaunsError::InvalidAction(
                    "file_read requires a 'path' string parameter".to_string(),
                )
            })?
            .to_string();

        debug!(skill = "file_read", path = %path);

        // validate_for_read enforces size limit as well as all guard rules.
        let safe = self.guard.validate_for_read(&path)?;

        let content = std::fs::read_to_string(safe.as_path()).map_err(|e| MaunsError::Skill {
            name: "file_read".to_string(),
            message: format!("cannot read '{}': {e}", path),
        })?;

        Ok(SkillOutput::ok_msg(
            serde_json::json!({ "content": content }),
            format!("read {} bytes", content.len()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn guard() -> Arc<PathGuard> {
        Arc::new(PathGuard::new(env::current_dir().unwrap()).unwrap())
    }

    #[tokio::test]
    async fn rejects_missing_path_param() {
        let s = FileReadSkill::new(guard());
        let e = s
            .execute(SkillInput {
                params: serde_json::json!({}),
            })
            .await
            .unwrap_err();
        assert!(matches!(e, MaunsError::InvalidAction(_)));
    }

    #[tokio::test]
    async fn rejects_traversal() {
        let s = FileReadSkill::new(guard());
        let e = s
            .execute(SkillInput {
                params: serde_json::json!({ "path": "../../etc/passwd" }),
            })
            .await
            .unwrap_err();
        assert!(matches!(e, MaunsError::PathTraversal(_)));
    }
}
