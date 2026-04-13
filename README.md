# Mauns

Mauns is a deterministic AI agent CLI written in Rust. Run `mauns` to enter an interactive agent session — type a task, and Mauns breaks it into a structured plan, executes it step-by-step using LLM-backed agents, and verifies the result.

- Supports **OpenAI**, **Anthropic** (Claude), and **Groq**
- Interactive session mode with slash commands (`/models`, `/plan`, `/diff`, `/status`, and more)
- Iterative execution loop with reflection and self-correction
- Safe by default: path guard, workspace confinement, `.maunsignore` support
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
  ghcr.io/mauns/mauns:latest
```

---

## Getting Started

Set your API key:

```sh
export CLAUDE_API_KEY=sk-ant-...
# or
export OPENAI_API_KEY=sk-...
# or
export GROQ_API_KEY=gsk_...
```

Initialize a config file (optional):

```sh
mauns config-init
```

Start the agent session:

```sh
mauns
```

You will see a session screen similar to this:

```
 ╭────────────────────────────────────────╮
 │  >_ Mauns  (v0.1.1)                           │
 │                                               │
 │  provider: anthropic  /models to change.      │
 │  model:    (default)                          │
 │  directory: ~/projects/myapp                  │
 ╰────────────────────────────────────────╯

  Tip: Type a task and press Enter to run it.
       Use /help to see all available commands.

>
```

Type a task and press Enter:

```
> add input validation to the login function
```

### Session Commands

| Command | Description |
|---|---|
| `/help` | List all commands |
| `/config [key value]` | View or set config |
| `/models [provider] [model]` | Switch provider or model |
| `/plan` | Show the last plan |
| `/status` | Show session status, tokens, run count |
| `/history [n]` | Show last N tasks |
| `/diff` | Show colored diffs from the last run |
| `/files` | List changed files |
| `/tokens` | Show token usage |
| `/dry-run` | Toggle dry-run mode |
| `/vibe` | Toggle vibe mode (faster) |
| `/deterministic` | Toggle temperature=0 |
| `/reset` | Clear session state |
| `/workspace` | Show working directory |
| `/clear` | Clear the screen |
| `/exit` | Exit the session |

### Switching Providers

```
/models groq llama-3.3-70b-versatile
```

```
/models openai gpt-4o
```

```
/models anthropic claude-sonnet-4-5
```

---

## Donations

Mauns is free and open-source. If it saves you time, consider sponsoring the project:

- GitHub Sponsors: [github.com/sponsors/mauns](https://github.com/sponsors/mauns)

---

## Reporting a Vulnerability

Do not open a public issue for security vulnerabilities.

Email **security@mauns.sh** with a description, reproduction steps, and potential impact. We will respond within 72 hours. See [SECURITY.md](./SECURITY.md).

---

## Contributors

Contributions are welcome. Please read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening a pull request.

<a href="https://github.com/mauns/mauns/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=mauns/mauns" />
</a>

---

## Troubleshooting

**`Configuration error: CLAUDE_API_KEY is not set`**
Export your API key before running: `export CLAUDE_API_KEY=sk-ant-...`

**`Unknown provider 'xyz'`**
Use `/models` inside the session to see valid providers and switch.

**`path traversal attempt blocked`**
The agent tried to access a path containing `..`. This is a safety block.

**`path is excluded by .maunsignore`**
Add a negation rule to `.maunsignore`: `!path/to/allow`

**`skill calls exceeded limit`**
Break the task into smaller pieces or use `/config max_iterations 40`.

**`icu_normalizer build error on Rust < 1.80`**
Upgrade your Rust toolchain: `rustup update stable`

---

## Supoort

If you need any help you can join our [Discord server](https://discord.gg/wuWVhP8eUD)

## License

MIT — see [LICENSE](./LICENSE)
