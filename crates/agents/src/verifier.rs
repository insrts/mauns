//! Verifier agent — validates execution completeness and output correctness.
//!
//! Returns `retry_suggested = true` when the task is incomplete but
//! the pipeline could recover by re-running the executor.

use std::sync::Arc;

use mauns_core::{
    error::{MaunsError, Result},
    types::{ExecutionOutput, VerificationReport},
};
use mauns_llm::provider::LlmProvider;
use tracing::{info, warn};

pub struct Verifier {
    provider: Arc<dyn LlmProvider>,
}

impl Verifier {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    pub async fn verify(&self, output: &ExecutionOutput) -> Result<VerificationReport> {
        info!(agent = "verifier", steps = output.results.len(), "verifying execution output");

        let steps_text: String = output
            .results
            .iter()
            .map(|r| {
                format!(
                    "Step {} (retries: {}): {}\nOutput: {}",
                    r.step.id, r.retries_used, r.step.task, r.output
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            "You are a strict quality verifier. Evaluate whether the execution output \
             correctly and completely addresses the original task.\n\
             \n\
             Check ALL of the following:\n\
             1. Is the task fully complete? (not just started or partially done)\n\
             2. Is the output correct and coherent?\n\
             3. Are there obvious errors, omissions, or contradictions?\n\
             \n\
             Respond ONLY with a JSON object with exactly these keys:\n\
             - \"passed\": boolean — true only if ALL checks above pass\n\
             - \"feedback\": string — 1-3 sentences explaining the verdict\n\
             - \"retry_suggested\": boolean — true if a retry might fix the issues\n\
             \n\
             No markdown fences, no prose outside the JSON.\n\n\
             Original task: {task}\n\n\
             Execution summary: {summary}\n\n\
             Step outputs:\n{steps_text}",
            task    = output.task,
            summary = output.summary,
        );

        let raw = self.provider.send_prompt(&prompt).await.map_err(|e| {
            MaunsError::Agent {
                agent:   "verifier".to_string(),
                message: format!("LLM call failed: {e}"),
            }
        })?;

        let parsed: VerificationJson =
            serde_json::from_str(raw.trim()).map_err(|e| MaunsError::Agent {
                agent:   "verifier".to_string(),
                message: format!("JSON parse failed: {e}\nraw: {raw}"),
            })?;

        let report = VerificationReport {
            passed:          parsed.passed,
            feedback:        parsed.feedback,
            retry_suggested: parsed.retry_suggested,
        };

        if report.passed {
            info!(agent = "verifier", "verification PASSED");
        } else {
            warn!(
                agent          = "verifier",
                retry          = report.retry_suggested,
                feedback       = %report.feedback,
                "verification FAILED"
            );
        }

        Ok(report)
    }
}

#[derive(serde::Deserialize)]
struct VerificationJson {
    passed:          bool,
    feedback:        String,
    #[serde(default)]
    retry_suggested: bool,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_output(task: &str, summary: &str) -> ExecutionOutput {
        ExecutionOutput {
            task:          task.to_string(),
            results:       vec![],
            summary:       summary.to_string(),
            iterations:    1,
            total_retries: 0,
            token_usage:   mauns_core::types::TokenUsage::default(),
        }
    }

    #[test]
    fn verification_json_parses_with_retry() {
        let json = r#"{"passed":false,"feedback":"incomplete","retry_suggested":true}"#;
        let v: VerificationJson = serde_json::from_str(json).unwrap();
        assert!(!v.passed);
        assert!(v.retry_suggested);
    }

    #[test]
    fn verification_json_defaults_retry_to_false() {
        let json = r#"{"passed":true,"feedback":"ok"}"#;
        let v: VerificationJson = serde_json::from_str(json).unwrap();
        assert!(v.passed);
        assert!(!v.retry_suggested);
    }

    #[test]
    fn make_output_helper_works() {
        let o = make_output("fix bug", "fixed");
        assert_eq!(o.task, "fix bug");
        assert_eq!(o.results.len(), 0);
    }
}
