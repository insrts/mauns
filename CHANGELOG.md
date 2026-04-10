# Changelog

All notable changes to Mauns are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

---

## [0.1.0] — 2026-01-01

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
- Deterministic mode (`--deterministic`): temperature=0, top_p=1 on all LLM calls
- Vibe mode (`--vibe`): single iteration per step, reduced friction
- Test mode (`--test`): dry-run + no-git + no-confirm in one flag
- `mauns config init` and `mauns config edit` commands
- `mauns.toml` configuration with `[execution]`, `[safety]`, `[git]`, `[logging]`
- OpenAI and Anthropic (Claude) provider support
- `DeterministicProvider` wrapper enforcing fixed sampling on all calls
- `mauns-sdk` crate for embedding Mauns as a library
- Complete open-source release: MIT license, CI, release automation, Docker, Helm chart stub

[Unreleased]: https://github.com/mauns/mauns/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/mauns/mauns/releases/tag/v0.1.0
