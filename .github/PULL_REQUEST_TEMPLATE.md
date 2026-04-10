## Description

<!-- What does this PR change and why? Link to the relevant issue if applicable. -->

Closes #

---

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that changes existing behavior)
- [ ] Refactor (no behavior change)
- [ ] Documentation update
- [ ] CI / tooling change

---

## Testing

<!-- Describe how you tested the change. Include relevant test names or commands. -->

```sh
cargo test --workspace
```

Specific tests added or modified:

- 

---

## Checklist

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes with no new warnings
- [ ] `cargo fmt --all -- --check` passes
- [ ] No `unwrap()`, `expect()`, `todo!()`, or `unimplemented!()` added in production paths
- [ ] No `unsafe` blocks added
- [ ] All file I/O goes through `PathGuard` (no direct `std::fs` calls in agents)
- [ ] No new global state or registries introduced
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] New public types and functions have doc comments

---

## Screenshots / Output

<!-- If the change affects CLI output, paste before/after examples. -->
