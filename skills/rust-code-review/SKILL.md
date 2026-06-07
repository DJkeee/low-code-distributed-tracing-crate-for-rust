---
name: rust-code-review
description: Use for reviewing Rust code, diffs, pull requests, or proposed changes with focus on correctness, safety, ownership, lifetimes, async behavior, concurrency, error handling, tests, dependency hygiene, performance traps, and API ergonomics.
---

# rust-code-review

## Purpose

Review Rust changes as a production maintainer: identify concrete bugs, regressions, missing tests, unsafe assumptions, API hazards, and operational risks.

## When to use this skill

Use this skill when the user asks for:

- Code review, PR review, diff review, or risk assessment.
- Review of Rust correctness, async behavior, safety, API ergonomics, or test coverage.
- A maintainer-style pass before merging.

## Inputs expected from user

Prefer available repository context before asking questions:

- Git diff or PR branch.
- `Cargo.toml` for dependencies and features.
- `README.md`, `src/lib.rs`, `src/main.rs` for API expectations.
- Changed files and related tests.
- `tests/`, `examples/`, `benches/` when relevant.
- `AGENTS.md` for review conventions.

## Assumptions

Default to code-review mode: findings first, ordered by severity. Do not rewrite the code unless the user explicitly asks for fixes.

## Workflow

1. Inspect the diff and surrounding code, not only changed lines.
2. Identify behavior changes and classify them as intended, ambiguous, or likely regression.
3. Review ownership and borrowing for invalid moves, over-cloning, hidden lifetime coupling, and aliasing risks.
4. Review error paths before happy paths.
5. Review async and concurrency behavior:
   - cancellation safety;
   - lock scope across `.await`;
   - task leaks;
   - backpressure;
   - shutdown behavior;
   - `Send`/`Sync` assumptions.
6. Review API compatibility and ergonomics for public items.
7. Check dependency changes for necessity, features, MSRV impact, and transitive risk.
8. Check tests for happy path, edge cases, failure cases, and async determinism.
9. Run verification commands when feasible.
10. Report only actionable findings with file and line references.

## Rust-specific rules

- Treat `unsafe` as high-risk. Require a documented invariant and tests around safe wrappers.
- Flag `unwrap`, `expect`, `panic!`, indexing, and unchecked conversions in production paths unless justified by an invariant.
- Flag swallowed errors, broad `map_err(|_| ...)`, and lost source errors.
- Flag public API changes that break source compatibility or behavior without migration notes.
- Flag holding `Mutex`, `RwLock`, or borrowed references across `.await` unless proven safe.
- Flag unbounded channels, unbounded buffers, and spawned tasks without lifecycle ownership.
- Flag blocking I/O or CPU-heavy work inside async tasks without `spawn_blocking` or an explicit design reason.
- Flag `clone` used to silence ownership errors when borrowing or restructuring would be clearer.
- Flag unnecessary dependencies and broad feature activation.
- Check whether tests assert behavior rather than implementation details.

## Commands to run

Run applicable commands:

```bash
git diff --check
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo check --all-targets --all-features
cargo doc --all-features --no-deps
```

For async or concurrency-sensitive changes, also run targeted tests repeatedly if practical:

```bash
cargo test <test_name> -- --nocapture
cargo test <module_name>
```

## Quality checklist

- Findings are specific, reproducible, and tied to code references.
- Correctness and failure paths were reviewed.
- Ownership, lifetimes, and trait bounds are sound and not over-constrained.
- Async code is cancellation-safe and does not hold locks across `.await` unnecessarily.
- Error handling preserves context and source errors.
- Public API changes are compatible or clearly marked breaking.
- Dependency changes are justified.
- Tests cover happy path, edge cases, and failure cases.
- Performance traps such as repeated allocation, accidental quadratic behavior, and blocking async paths are considered.

## Anti-patterns to avoid

- Writing a generic approval without inspecting error paths.
- Reporting style-only nits while missing correctness issues.
- Suggesting broad rewrites during review unless the current design is unsafe or unmaintainable.
- Treating `cargo test` success as proof of correctness.
- Ignoring feature combinations and no-default-features builds.
- Ignoring operational behavior: logs, metrics, shutdown, and diagnostics.
- Recommending `Arc<Mutex<_>>` as a default fix without checking ownership design.

## Output format

Use this order:

1. Findings, ordered by severity, with file and line references.
2. Open questions or assumptions.
3. Verification performed and commands run.
4. Residual risks or missing tests.
5. Short summary only after findings.

If there are no findings, state that explicitly and mention remaining testing gaps.

## Example user requests

- "Review this Rust PR."
- "Check this diff for async bugs."
- "Do a maintainer review before merge."
- "Review error handling and API compatibility."
