# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Email **security@mauns.sh** with:

1. A clear description of the vulnerability
2. Reproduction steps or a proof-of-concept
3. The potential impact (data exposure, privilege escalation, etc.)
4. Your contact information for follow-up

### What to expect

- **Acknowledgement** within 72 hours of receipt
- **Status update** within 7 days confirming whether the issue is accepted
- **Fix and coordinated disclosure** within 30 days for accepted reports

We follow responsible disclosure: we will notify you before publishing a fix and credit you in the release notes unless you prefer anonymity.

## Security Design

Mauns enforces several layers of protection by design:

- **PathGuard**: all file operations are confined to the workspace root. Path traversal attempts (`../`) are rejected before any I/O occurs.
- **.maunsignore**: user-defined exclusion rules are enforced at the guard layer and cannot be bypassed by agent actions.
- **Credential isolation**: API tokens are read from environment variables only. They are never written to config files, logged, or included in `Debug` output.
- **Protected branches**: the git layer will never commit directly to `main`, `master`, `production`, or `staging`.
- **No shell execution**: skill implementations cannot spawn child processes or run shell commands.
- **No unsafe Rust**: the entire codebase compiles without `unsafe` blocks.
