//! Structured planner agent.
//!
//! Produces a dependency-aware plan where each step has an id, a task
//! description, and an explicit list of step ids it depends on.
//! The executor uses `Plan::execution_order()` to honour dependencies.

use std::sync::Arc;

use mauns_core::{
    error::{MaunsError, Result},
    types::{Plan, RunContext, Step},
};
use mauns_llm::provider::LlmProvider;
use tracing::info;

pub struct Planner {
    provider: Arc<dyn LlmProvider>,
}

impl Planner {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    pub async fn plan(&self, task: &str, ctx: &RunContext) -> Result<Plan> {
        info!(agent = "planner", "producing structured plan");

        let policy_section = if ctx.agents_policy.raw.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nBehavioral constraints (AGENTS.md — MUST respect; \
                 MUST NOT override system safety rules):\n{}",
                ctx.agents_policy.raw
            )
        };

        let prefs_section = if ctx.mauns_prefs.raw.is_empty() {
            String::new()
        } else {
            format!("\n\nUser preferences (MAUNS.md):\n{}", ctx.mauns_prefs.raw)
        };

        let project_section = if ctx.project.context_hint.is_empty() {
            String::new()
        } else {
            format!("\n\nProject context: {}", ctx.project.context_hint)
        };

        let prompt = format!(
            "You are an expert task planner. Decompose the task into the minimum set of \
             discrete, actionable steps needed to complete it correctly.\n\
             \n\
             Rules:\n\
             - No redundant or overlapping steps.\n\
             - Order steps logically; use depends_on to express prerequisites.\n\
             - Keep step tasks concise and specific.\n\
             - Each step must be independently executable.\n\
             \n\
             Output ONLY a JSON object with a single key \"steps\" containing an array.\n\
             Each element must have exactly these keys:\n\
               \"id\"         — integer, 1-based, unique\n\
               \"task\"       — string, what must be done\n\
               \"depends_on\" — array of ids that must complete first (empty if none)\n\
             \n\
             Example:\n\
             {{\"steps\":[{{\"id\":1,\"task\":\"Read existing code\",\"depends_on\":[]}},\
             {{\"id\":2,\"task\":\"Write new function\",\"depends_on\":[1]}}]}}\n\
             \n\
             No prose, no markdown fences, only the raw JSON object.\
             {policy_section}{prefs_section}{project_section}\n\n\
             Task: {task}"
        );

        let raw = self.provider.send_prompt(&prompt).await?;

        let parsed = parse_plan_json(raw.trim()).map_err(|e| MaunsError::Agent {
            agent:   "planner".to_string(),
            message: format!("plan parse failed: {e}\nraw: {raw}"),
        })?;

        if parsed.is_empty() {
            return Err(MaunsError::Agent {
                agent:   "planner".to_string(),
                message: "planner returned zero steps".to_string(),
            });
        }

        validate_plan(&parsed).map_err(|e| MaunsError::Agent {
            agent:   "planner".to_string(),
            message: e,
        })?;

        info!(agent = "planner", step_count = parsed.len(), "plan produced");

        Ok(Plan { task: task.to_string(), steps: parsed })
    }
}

// ---------------------------------------------------------------------------
// JSON parsing
// ---------------------------------------------------------------------------

fn parse_plan_json(text: &str) -> std::result::Result<Vec<Step>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("invalid JSON: {e}"))?;

    let arr = value
        .get("steps")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing 'steps' array".to_string())?;

    let mut steps = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        let id = item
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| format!("step[{i}] missing integer 'id'"))?
            as usize;

        let task = item
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("step[{i}] missing string 'task'"))?
            .to_string();

        if task.trim().is_empty() {
            return Err(format!("step[{i}] has empty 'task'"));
        }

        let depends_on = item
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_u64().map(|n| n as usize))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        steps.push(Step { id, task, depends_on });
    }

    Ok(steps)
}

// ---------------------------------------------------------------------------
// Plan validation
// ---------------------------------------------------------------------------

fn validate_plan(steps: &[Step]) -> std::result::Result<(), String> {
    use std::collections::HashSet;

    // All ids must be unique.
    let mut seen_ids: HashSet<usize> = HashSet::new();
    for s in steps {
        if !seen_ids.insert(s.id) {
            return Err(format!("duplicate step id {}", s.id));
        }
    }

    // All depends_on references must refer to existing ids.
    for s in steps {
        for dep in &s.depends_on {
            if !seen_ids.contains(dep) {
                return Err(format!(
                    "step {} depends on id {} which does not exist",
                    s.id, dep
                ));
            }
            if *dep == s.id {
                return Err(format!("step {} depends on itself", s.id));
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_plan() {
        let json = r#"{"steps":[
            {"id":1,"task":"Read files","depends_on":[]},
            {"id":2,"task":"Write output","depends_on":[1]}
        ]}"#;
        let steps = parse_plan_json(json).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].id, 1);
        assert_eq!(steps[1].depends_on, vec![1]);
    }

    #[test]
    fn parse_rejects_missing_steps_key() {
        let json = r#"[{"id":1,"task":"foo","depends_on":[]}]"#;
        assert!(parse_plan_json(json).is_err());
    }

    #[test]
    fn parse_rejects_empty_task() {
        let json = r#"{"steps":[{"id":1,"task":"","depends_on":[]}]}"#;
        assert!(parse_plan_json(json).is_err());
    }

    #[test]
    fn validate_rejects_duplicate_ids() {
        let steps = vec![
            Step { id: 1, task: "a".into(), depends_on: vec![] },
            Step { id: 1, task: "b".into(), depends_on: vec![] },
        ];
        assert!(validate_plan(&steps).is_err());
    }

    #[test]
    fn validate_rejects_unknown_dependency() {
        let steps = vec![
            Step { id: 1, task: "a".into(), depends_on: vec![99] },
        ];
        assert!(validate_plan(&steps).is_err());
    }

    #[test]
    fn validate_rejects_self_dependency() {
        let steps = vec![
            Step { id: 1, task: "a".into(), depends_on: vec![1] },
        ];
        assert!(validate_plan(&steps).is_err());
    }

    #[test]
    fn validate_accepts_valid_plan() {
        let steps = vec![
            Step { id: 1, task: "a".into(), depends_on: vec![] },
            Step { id: 2, task: "b".into(), depends_on: vec![1] },
        ];
        assert!(validate_plan(&steps).is_ok());
    }
}
