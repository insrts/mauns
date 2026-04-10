# Mauns

Mauns is a deterministic AI agent CLI written in Rust. It breaks tasks into structured plans, executes them step-by-step using LLM-backed agents, and verifies results — all with strict filesystem safety, `.maunsignore` support, and Git-aware change tracking.

- Supports OpenAI and Anthropic (Claude)
- Iterative execution loop with reflection and self-correction
- Safe by default: path guard, workspace confinement, blocklisted files
- Structured plans with dependency awareness
- Git integration with branch creation and optional PR creation

---

## Installing Mauns

### npm

```sh
npm install -g mauns
```

### pnpm

```sh
pnpm add -g mauns
```

### bun

```sh
bun add -g mauns
```

### Docker

```sh
docker pull ghcr.io/mauns/mauns:latest
docker run --rm -it \
  -e CLAUDE_API_KEY=$CLAUDE_API_KEY \
  -v $(pwd):/workspace \
  ghcr.io/mauns/mauns:latest \
  run "your task here"
```

### Helm

```sh
helm repo add mauns https://mauns.github.io/charts
helm repo update
helm install mauns mauns/mauns \
  --set env.CLAUDE_API_KEY=$CLAUDE_API_KEY
```

---

## Getting Started

Set your API key:

```sh
export CLAUDE_API_KEY=sk-ant-...
# or
export OPENAI_API_KEY=sk-...
```

Initialize a config file:

```sh
mauns config init
```

Run a task:

```sh
mauns run "add input validation to the login function"
```

Run in dry-run mode to preview changes without writing files:

```sh
mauns run "refactor the database module" --dry-run
```

Run in deterministic mode for reproducible outputs:

```sh
mauns run "write unit tests for the auth module" --deterministic
```

Test mode (dry-run + no git + no confirmation):

```sh
mauns run "add error handling to all API endpoints" --test
```

---

## Donations

Mauns is free and open-source. If it saves you time, consider sponsoring the project:

- GitHub Sponsors: [github.com/sponsors/mauns](https://github.com/sponsors/mauns)

---

## Reporting a Vulnerability

Do not open a public issue for security vulnerabilities.

Email **security@mauns.sh** with:
- A description of the vulnerability
- Steps to reproduce
- Potential impact

We will respond within 72 hours. See [SECURITY.md](./SECURITY.md) for the full policy.

---

## Contributors

Contributions are welcome. Please read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening a pull request.

<a href="https://github.com/mauns/mauns/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=mauns/mauns" />
</a>

---

## Troubleshooting

**`CLAUDE_API_KEY is not set`**
Export your API key before running: `export CLAUDE_API_KEY=sk-ant-...`

**`path traversal attempt blocked`**
The requested path contains `..` components. Use paths relative to your project root.

**`path is excluded by .maunsignore`**
Add an exception to `.maunsignore` using `!pattern` syntax.

**`skill calls exceeded limit`**
The agent hit the 50-call safety limit. Break the task into smaller pieces.

**`max_iterations reached`**
Increase the iteration limit: `mauns run "..." --max-iterations 40`

**`icu_normalizer build error on Rust < 1.80`**
Upgrade your Rust toolchain: `rustup update stable`

---

## License

MIT — see [LICENSE](./LICENSE)
