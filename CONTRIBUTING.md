# Contributing to Mauns

Thank you for your interest in contributing. This guide covers everything you need to get started.

---

## Development Setup

### Prerequisites

- Rust stable (≥ 1.80): install via [rustup](https://rustup.rs)
- Git

### Clone and build

```sh
git clone https://github.com/mauns/mauns.git
cd mauns
cargo build --release
```

### Run the binary

```sh
export CLAUDE_API_KEY=sk-ant-...
cargo run --release --bin mauns -- run "your task here"
```

### Run tests

```sh
cargo test --workspace
```

### Lint and format

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

---

## Coding Rules

- **No `unsafe` Rust.** The entire codebase compiles without unsafe blocks.
- **No `unwrap()` or `expect()` in production code.** Use `?` and `map_err`.
- **No placeholder code.** Every function must have a complete implementation.
- **All file I/O must go through `PathGuard`.** Direct `std::fs` calls in agents are forbidden.
- **No global registries.** Skills are added via `SkillSet::with_skill()`.
- **Test coverage.** New behavior must include unit tests in a `#[cfg(test)]` module within the source file.
- **Error messages must be specific.** Include the path, value, or context that caused the error.

---

## Project Structure

```
mauns/
  crates/
    core/        — shared types, errors, project detection
    llm/         — LLM provider trait and implementations
    agents/      — planner, executor, verifier, pipeline
    filesystem/  — PathGuard, .maunsignore, diff engine
    skills/      — AgentSkill trait, SkillSet, built-in skills
    git/         — git safety and operations
    github/      — GitHub API client and PR creation
    config/      — mauns.toml schema and loader
    sdk/         — embeddable Mauns builder
  apps/
    cli/         — binary entry point
```

---

## Pull Request Guidelines

- One logical change per PR.
- All tests must pass: `cargo test --workspace`
- No new Clippy warnings: `cargo clippy --workspace --all-targets -- -D warnings`
- Code must be formatted: `cargo fmt --all -- --check`
- Update `CHANGELOG.md` under `[Unreleased]` with a summary of changes.
- Reference any related issues in the PR description.
- PRs that break safety rules (PathGuard bypass, unsafe blocks, shell execution from skills) will not be merged regardless of other quality.

---

## Reporting Issues

Use the GitHub issue tracker. For security vulnerabilities, see [SECURITY.md](./SECURITY.md).
