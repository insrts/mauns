# Changelog

All notable changes to Mauns are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

---

## [0.1.1] — 2026-04-13

### Added

- **Agent Session Mode** — running `mauns` with no arguments now enters a fully interactive REPL.
  The session renders a Codex-style splash screen showing the provider, model, and workspace directory.
- **Groq provider** — `provider = "groq"` in `mauns.toml` or `export GROQ_API_KEY=gsk_...`.
  Supports six models: `llama-3.3-70b-versatile`, `llama-3.1-70b-versatile`, `llama-3.1-8b-instant`,
  `mixtral-8x7b-32768`, `gemma2-9b-it`, and `llama3-groq-70b-8192-tool-use-preview`.
- **Model selection** — `model = ""` in config or `MAUNS_MODEL` env var overrides the provider default.
  Changed live in session with `/models <provider> [model]`.
- **`build_provider_with_model()`** — LLM registry function that accepts an explicit model override.
- **`models_for_provider()`** — returns the curated model list for any provider.
- **`crates/session`** — new crate containing the full session runtime:
  - `SessionState` — mutable runtime state (provider, model, mode, history, reports)
  - `SessionMode` — `Interactive`, `Running`, `DryRun`, `Vibe`
  - `CommandHistory` — persistent history backed by `~/.mauns_history` (max 500 entries)
  - `display` module — ANSI-colored splash, prompt, diff rendering, step indicators
  - `SessionRunner` — async REPL loop with live progress reporting
  - `SessionProgressReporter` — implements `ProgressReporter` for real-time step output

- **Slash commands** (all available inside the session):
  - `/help` — list all commands
  - `/config [key value]` — view or set execution config live
  - `/models [provider] [model]` — list or switch provider and model
  - `/plan` — display the last generated plan with dependencies
  - `/status` — show provider, model, mode, run count, last verdict, token usage
  - `/history [n]` — show last N task inputs (default 10)
  - `/diff` — show colored unified diffs from the last run
  - `/files` — list files changed in the last run
  - `/tokens` — show prompt/completion/total token usage for the last run and session total
  - `/dry-run` — toggle dry-run mode (no disk writes)
  - `/vibe` — toggle vibe mode (single iteration, faster)
  - `/deterministic` — toggle deterministic mode (temperature=0)
  - `/reset` — clear session state, keep config
  - `/workspace` — show current working directory
  - `/clear` — clear the terminal screen
  - `/exit` / `/quit` / `/q` — exit the session

### Changed

- `apps/cli/src/main.rs` — completely replaced. `mauns` with no args enters session mode.
  Only three non-session args remain: `config-init`, `--version`, `--help`.
- `crates/config/src/schema.rs` — added `model: String` field and `[groq]` section.
  `effective_model()` returns `None` when model is empty (use provider default).
- `crates/config/src/loader.rs` — merges `model` and `groq.api_key`; reads `MAUNS_MODEL`
  and `GROQ_API_KEY` environment variables.
- `crates/llm/src/registry.rs` — `ProviderKind` gains `Groq` variant; `ProviderKind::all()`
  returns all three providers for enumeration.
- `Cargo.toml` (workspace) — added `crates/session` and `crossterm = "0.27"`.
- `apps/cli/Cargo.toml` — bumped to `0.1.1`, replaced most deps with `mauns-session`.

### Removed

- All CLI subcommand flags (`run`, `config-edit`, `--dry-run`, `--vibe`, `--test`, etc.)
  are removed from the binary interface. Equivalent functionality is available as
  session slash commands (`/dry-run`, `/vibe`, `/config`, `/status`).

---

## [0.1.0] — 2026-04-11

### Added

- Core agent pipeline: Planner → Executor → Verifier
- Structured planning with dependency-aware step ordering (`id`, `task`, `depends_on`)
- Iterative execution loop with configurable `max_iterations` (default 20)
- Explicit reflection step after each iteration to improve decision making
- Self-correction: retry on failure with loop detection after 3 identical errors
- Token usage tracking (prompt + completion) with optional hard limit (`--max-tokens`)
- Ctrl+C interrupt support with graceful partial result return
- `ExecContext`: structured rolling context with compression, key-output memory, and error history
- Git integration: automatic branch creation (`mauns/<slug>-<timestamp>`), commit, push
- GitHub PR creation via `GITHUB_TOKEN` environment variable
- `.maunsignore` support with gitignore-compatible glob syntax
- `PathGuard`: workspace confinement, path traversal prevention, size limit (1 MiB), always-skip dirs
- `SkillSet` with O(1) internal HashMap dispatch; builder API via `.with_skill()`
- Built-in skills: `file_read`, `file_write`, `dir_list`
- Project detection: Rust, JavaScript, TypeScript, Python, Go
- Deterministic mode: temperature=0, top_p=1 on all LLM calls
- Vibe mode: single iteration per step, reduced friction
- `mauns.toml` configuration with `[execution]`, `[safety]`, `[git]`, `[logging]`
- OpenAI and Anthropic (Claude) provider support
- `DeterministicProvider` wrapper enforcing fixed sampling on all calls
- `mauns-sdk` crate for embedding Mauns as a library
- Complete open-source release: MIT license, CI, release automation, Docker, Helm chart stub

[0.1.1]: https://github.com/mauns/mauns/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/mauns/mauns/releases/tag/v0.1.0
