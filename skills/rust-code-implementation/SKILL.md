---
name: rust-code-implementation
description: Use for implementing production-ready Rust features or fixes with repository context, minimal dependencies, sound ownership, typed errors, tests, formatting, clippy, and validation. Trigger when asked to write Rust code, implement a feature, fix a bug, add a module, or make production-ready changes.
---

# rust-code-implementation

## Purpose

Implement Rust changes that are correct, maintainable, tested, and aligned with the existing crate architecture.

## When to use this skill

Use this skill when the user asks to:

- Write Rust code.
- Implement a feature or bug fix.
- Add a module, function, trait implementation, or integration.
- Make code production-ready.
- Replace pseudocode with real Rust.

## Inputs expected from user

Read project context before coding:

- `Cargo.toml` for workspace layout, features, dependencies, edition, binaries.
- `README.md` for behavior and usage.
- `src/lib.rs`, `src/main.rs`, and relevant modules.
- `tests/`, `examples/`, `benches/` when relevant.
- `AGENTS.md` for repository-specific rules.

Useful user inputs:

- Required behavior and non-goals.
- Public API compatibility requirements.
- Error handling expectations.
- Runtime constraints for async code.
- Whether dependencies may be added.

## Assumptions

Do not add new dependencies, change public API, or introduce runtime requirements without a clear reason. Prefer incremental changes that compile and can be reviewed.

## Workflow

1. Inspect repository structure and relevant call sites.
2. Identify whether the change affects library API, CLI API, or internal API.
3. Define the smallest implementation plan.
4. Add or update tests before or alongside implementation.
5. Implement using existing patterns, naming, and module boundaries.
6. Use ownership and borrowing deliberately:
   - borrow inputs when ownership is unnecessary;
   - move values at clear boundaries;
   - avoid clones unless they express ownership needs.
7. Use typed errors in libraries and contextual errors at application boundaries.
8. Keep async runtime assumptions explicit.
9. Run focused validation, then broad validation.
10. Summarize behavior, files changed, and verification.

## Rust-specific rules

- Use `Result<T, E>` for recoverable errors.
- Do not use `unwrap` or `expect` in production code unless the invariant is impossible to violate and documented.
- Preserve source errors with `#[source]`, `thiserror`, or contextual wrapping when appropriate.
- Use `anyhow` only in binaries, examples, tests, or top-level application orchestration.
- Do not expose implementation details in public API.
- Keep feature flags additive and test relevant combinations.
- Prefer simple concrete types over premature traits and generics.
- Avoid global mutable state.
- Avoid blocking calls in async contexts unless isolated with the runtime's blocking mechanism.
- Do not hold locks across `.await` unless the lock type and design explicitly support it.
- Use structured `tracing` fields for production diagnostics when adding logs.
- Keep unsafe code out unless unavoidable; if used, document invariants and provide safe wrappers.

## Commands to run

Run focused checks during development:

```bash
cargo check
cargo test <test_name>
```

Run final validation:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo check --no-default-features
cargo doc --all-features --no-deps
```

Adjust commands for workspaces:

```bash
cargo check -p <crate-name>
cargo test -p <crate-name> --all-features
```

## Quality checklist

- Change solves the requested behavior with minimal scope.
- Public API changes are intentional and documented.
- Error handling is typed or contextual as appropriate.
- No production `unwrap`/`expect` without invariant comment.
- Ownership model is clear and avoids unnecessary clones.
- Async behavior handles cancellation, backpressure, and shutdown where relevant.
- Tests cover happy path, edge cases, and failure cases.
- Formatting, clippy, tests, and docs checks pass or failures are reported.
- Dependencies and feature flags are justified.
- Observability is structured and redacted when logs are added.

## Anti-patterns to avoid

- Implementing broad architecture changes for a narrow feature.
- Adding dependencies for trivial code.
- Using `clone`, `Arc<Mutex<_>>`, or `'static` to bypass design issues.
- Hiding errors with `ok()`, `unwrap_or_default()`, or string-only mapping.
- Mixing library and CLI concerns.
- Writing tests after declaring success but not running them.
- Adding logs that leak sensitive data.
- Ignoring no-default-features builds in library crates.

## Output format

Include:

- Implementation summary.
- Files changed.
- Public API or CLI impact.
- Tests added or changed.
- Commands run and results.
- Known limitations or follow-up work.

## Example user requests

- "Implement this Rust feature."
- "Write production-ready Rust code for this module."
- "Fix this bug and add tests."
- "Add a new crate API without breaking existing users."
