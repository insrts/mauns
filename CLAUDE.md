# Claude Interaction Guide for Mauns

This file instructs Claude how to contribute to the Mauns codebase correctly.

---

## Architecture Rules

- Mauns is a Cargo workspace. All crates are in `crates/` or `apps/`.
- Do not add new top-level crates without updating `Cargo.toml`'s `[workspace]` members list.
- Do not introduce global state, registries, or plugin systems. Skills are registered via `SkillSet::with_skill()`.
- All file I/O must go through `PathGuard`. Direct use of `std::fs` in agents is forbidden.
- Do not use `unsafe` Rust anywhere in the codebase.

## Code Quality Rules

- No placeholder code: no `todo!()`, `unimplemented!()`, or stub implementations.
- No `unwrap()` or `expect()` in production paths. Use `Result` propagation with `?`.
- All public types must derive `Debug`. Sensitive types (tokens, keys) must implement `Debug` manually and redact the value.
- New modules must be declared in the parent `mod.rs` or `lib.rs`.
- Tests belong in `#[cfg(test)]` modules inside the relevant source file.

## Safety Rules — Never Violate

- `PathGuard::validate()` or `validate_for_read()` must be called before any filesystem operation.
- `.maunsignore` rules are enforced by `PathGuard`. Do not bypass them.
- The git safety layer must never commit to protected branches (`main`, `master`, `production`, `staging`).
- API tokens must never be logged, stored in config files, or exposed in `Debug` output.
- Skill implementations must not spawn shell commands or child processes.

## Deterministic Mode

When `ctx.deterministic` is true, all LLM calls must use `SamplingOptions::deterministic()` (temperature=0, top_p=1). Use `send_prompt_with_options()` rather than `send_prompt()` whenever the run context is available.

## Adding Features

1. New skills: implement `AgentSkill` in `crates/skills/src/builtin/`, export from `builtin/mod.rs`, add to `default_skillset()`.
2. New LLM providers: implement `LlmProvider` in `crates/llm/src/`, export from `crates/llm/src/lib.rs`, add a variant to `ProviderKind`.
3. New config fields: add to the appropriate struct in `crates/config/src/schema.rs`, update `MaunsConfig::validate()`, update `default_toml()`, and handle in `merge_toml()` and `apply_env_overrides()`.

## Commit Discipline

- Commits must not include generated build artifacts.
- Each commit should address a single concern.
- Breaking changes to public crate APIs require a version bump.
