---
name: rust-test-design
description: Use for designing or writing Rust tests: unit tests, integration tests, async tests, property-based tests, fixtures, fakes, deterministic tests, edge cases, failure cases, and coverage for public behavior.
---

# rust-test-design

## Purpose

Design and implement reliable Rust tests that prove behavior across happy paths, edge cases, failure paths, and async/concurrency scenarios.

## When to use this skill

Use this skill when the user asks to:

- Add tests for Rust code.
- Improve test coverage.
- Design unit, integration, property-based, or async tests.
- Add fixtures, fakes, or deterministic test infrastructure.
- Reproduce bugs with failing tests before fixing them.

## Inputs expected from user

Read project context first:

- `Cargo.toml` for test dependencies and feature flags.
- `README.md` for expected behavior.
- `src/lib.rs`, `src/main.rs`, and target modules.
- `tests/`, existing test helpers, fixtures, snapshots.
- `examples/` for public usage.
- `AGENTS.md` for test conventions.

Useful user inputs:

- Behavior to verify.
- Bug reproduction steps.
- Accepted test dependencies.
- Whether slow or external integration tests are allowed.

## Assumptions

Tests should be deterministic and local by default. Do not add network, clock, filesystem, or process dependencies unless the behavior requires them and they are isolated.

## Workflow

1. Identify observable behavior and invariants before writing tests.
2. Choose test level:
   - Unit tests for small pure logic and private edge cases.
   - Integration tests for public API and crate behavior.
   - Doc tests for public examples.
   - Property tests for broad input spaces and invariants.
3. Add a failing regression test first when fixing a bug.
4. Cover happy path, edge cases, and failure cases.
5. For async code, control time, cancellation, and task lifecycle explicitly.
6. Use fakes or in-memory implementations instead of mocks when they better model behavior.
7. Keep fixtures minimal, named by behavior, and close to tests unless reused widely.
8. Avoid relying on test order, wall-clock timing, external services, or random seeds without control.
9. Run focused tests first, then full test suite.
10. Document any intentionally untested behavior and why.

## Rust-specific rules

- Put crate-level integration tests under `tests/` for public API behavior.
- Put module-private tests in `#[cfg(test)] mod tests` when they need private access.
- Use `#[tokio::test]`, `#[async_std::test]`, or runtime-specific testing only when that runtime is already used or approved.
- Use `tokio::time::pause` and controlled advancement for time-dependent Tokio tests when available.
- Avoid sleeping in tests. Prefer channels, barriers, notifications, or fake clocks.
- Use `tempfile` for isolated filesystem tests if the dependency already exists or is justified.
- Use `proptest` or `quickcheck` only when property coverage is valuable and dependency policy allows it.
- Assert specific error variants for library APIs, not just `is_err()`.
- Assert stdout, stderr, and exit codes for CLI behavior.
- Avoid `unwrap` in test setup if a clearer failure message is useful; `expect` with context is acceptable in tests.
- Keep tests independent and parallel-safe.

## Commands to run

Run targeted tests first:

```bash
cargo test <test_name>
cargo test <module_name>
```

Run full validation:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo test --doc --all-features
```

For flaky or async-sensitive tests:

```bash
cargo test <test_name> -- --nocapture
cargo test <test_name> -- --test-threads=1
```

## Quality checklist

- Tests verify behavior observable by users or maintainers.
- Happy path, edge cases, and failure cases are covered.
- Regression tests fail without the bug fix.
- Async tests do not depend on arbitrary sleeps.
- Tests clean up files, tasks, and global state.
- Test names describe behavior, not implementation mechanics.
- Assertions check meaningful outputs, error variants, logs, metrics, or side effects.
- Feature-gated behavior is tested under relevant features.
- No external service is required unless explicitly marked and isolated.
- New test dependencies are justified.

## Anti-patterns to avoid

- Writing tests that only execute code without assertions.
- Testing private implementation when public behavior is sufficient.
- Using sleeps to hope async work has completed.
- Using random data without fixed seeds or shrinking support.
- Ignoring failure paths because they are harder to set up.
- Over-mocking so tests no longer represent real behavior.
- Adding brittle snapshot tests for unstable formatting.
- Making tests depend on current time, machine locale, user home directory, or test order.

## Output format

Include:

- Test strategy.
- Tests added or changed with file references.
- Cases covered: happy path, edge cases, failure cases.
- Commands run.
- Known gaps or intentionally deferred tests.

## Example user requests

- "Add tests for this Rust module."
- "Write a regression test for this borrow checker fix."
- "Add async tests without flakiness."
- "Improve coverage for error handling."
