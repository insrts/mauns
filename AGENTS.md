# Mauns Architecture & Agent Guide

This document describes the internal architecture of Mauns and explains how to extend it with new skills.

---

## Architecture Overview

Mauns is a Cargo workspace with the following crates:

| Crate              | Responsibility                                              |
|--------------------|-------------------------------------------------------------|
| `crates/core`      | Shared types, error variants, project detection             |
| `crates/llm`       | LLM provider trait, OpenAI + Anthropic implementations      |
| `crates/agents`    | Planner, Executor, Verifier, Pipeline, context management   |
| `crates/filesystem`| PathGuard, `.maunsignore`, diff engine, change tracker      |
| `crates/skills`    | `AgentSkill` trait, `SkillSet` builder, built-in skills     |
| `crates/git`       | Repository lifecycle, branch safety, commit, push           |
| `crates/github`    | GitHub REST client, PR creation                             |
| `crates/config`    | `mauns.toml` schema, loader, validation                     |
| `crates/sdk`       | `Mauns` builder for embedding Mauns in other projects       |
| `apps/cli`         | Binary entry point (`mauns` command)                        |

---

## How Agents Work

### Planner

Receives the raw task string and produces a structured JSON plan:

```json
{
  "steps": [
    { "id": 1, "task": "Read existing implementation", "depends_on": [] },
    { "id": 2, "task": "Write new function", "depends_on": [1] }
  ]
}
```

Steps include dependency declarations. `Plan::execution_order()` performs a topological sort before the executor begins.

### Executor

Runs each step in dependency order inside an iterative loop:

1. Sends an action prompt to the LLM (iteration N / max).
2. Parses the response as one or more `AgentAction` JSON objects per line.
3. Dispatches `Skill` actions through `SkillSet::dispatch()` — O(1) HashMap lookup.
4. Feeds skill results back into the rolling `ExecContext`.
5. After each iteration, sends a reflection prompt (unless vibe mode).
6. Loops until `Done` is received, the step limit is hit, or an interrupt occurs.

Valid action schemas:
```json
{"type":"skill","name":"file_read","input":{"path":"src/main.rs"}}
{"type":"note","message":"Starting refactor"}
{"type":"done","summary":"Wrote the new function and updated imports"}
```

### Verifier

Receives the full `ExecutionOutput` and evaluates three criteria:
- Is the task fully complete?
- Is the output correct and coherent?
- Are there errors, omissions, or contradictions?

Returns `passed: bool`, `feedback: String`, and `retry_suggested: bool`.

---

## Adding a New Skill

1. Create `crates/skills/src/builtin/my_skill.rs`.
2. Implement `AgentSkill`:

```rust
use async_trait::async_trait;
use mauns_core::{error::Result, types::{SkillInput, SkillOutput}};
use crate::skill::AgentSkill;

pub struct MySkill;

#[async_trait]
impl AgentSkill for MySkill {
    fn name(&self) -> &str { "my_skill" }
    fn description(&self) -> &str {
        "Does X. Input: {\"param\": \"<value>\"}. Output: {\"result\": \"<value>\"}."
    }
    async fn execute(&self, input: SkillInput) -> Result<SkillOutput> {
        let param = input.params.get("param")
            .and_then(|v| v.as_str())
            .ok_or_else(|| mauns_core::error::MaunsError::InvalidAction(
                "my_skill requires 'param'".into()
            ))?;
        Ok(SkillOutput::ok(serde_json::json!({ "result": param })))
    }
}
```

3. Export from `crates/skills/src/builtin/mod.rs`.
4. Add to `default_skillset()` or pass via `Pipeline::new(..., extra_skills)` / `Mauns::with_skill(...)`.

---

## Constraints

- **No registries.** Skills are stored in a `Vec` + internal `HashMap` inside `SkillSet`. There is no global registry.
- **No shell execution.** Skills must not spawn processes or use `std::process::Command`.
- **No token access.** Skills cannot read API keys from env or config.
- **PathGuard is mandatory.** All file operations must go through `PathGuard::validate()` or `validate_for_read()`.
- **`.maunsignore` is enforced.** Paths matched by `.maunsignore` are blocked at the guard layer before any skill sees them.
- **Protected branches.** The git layer will never commit directly to `main`, `master`, `production`, or `staging`.
- **No unsafe Rust.** The entire codebase compiles without any `unsafe` blocks.
- **User constraints from AGENTS.md are advisory.** They are injected into prompts but cannot override the above safety rules.
