---
name: rust-refactor
description: Use for safe Rust refactoring that preserves behavior, minimizes diff size, improves readability, removes duplication, simplifies types, and avoids unnecessary architectural changes. Trigger on refactor, cleanup, simplify, deduplicate, restructure, or migrate without behavior changes.
---

# rust-refactor

## Purpose

Refactor Rust code safely while preserving behavior and keeping the diff focused, reviewable, and test-backed.

## When to use this skill

Use this skill when the user asks to:

- Refactor, simplify, clean up, deduplicate, or restructure Rust code.
- Improve readability without changing behavior.
- Migrate internals while keeping public API stable.
- Split modules or extract helpers.

## Inputs expected from user

Read project context first:

- `Cargo.toml` for crate layout and feature constraints.
- `README.md` for documented behavior.
- `src/lib.rs`, `src/main.rs`, and target modules.
- `tests/`, `examples/`, and snapshots if present.
- `AGENTS.md` for local coding rules.

Useful user inputs:

- Whether public API changes are allowed.
- Refactor scope and files to avoid.
- Performance or compatibility constraints.

## Assumptions

Unless the user says otherwise, preserve behavior and public API. Do not introduce dependencies or architecture changes just to make code look cleaner.

## Workflow

1. Establish current behavior from tests, docs, examples, and call sites.
2. Run or identify baseline tests before editing when feasible.
3. Define the smallest safe refactor step.
4. Make mechanical changes first: renames, extraction, module movement, duplication removal.
5. Keep public API stable; if a public rename is requested, add deprecation or migration notes when appropriate.
6. Prefer local helper functions over new traits or generics unless multiple call sites need abstraction.
7. Simplify types only when it reduces caller burden or internal complexity.
8. Avoid mixing refactor with behavior changes. If a bug is found, separate the fix or clearly report it.
9. Run formatting, clippy, and tests after changes.
10. Summarize preserved behavior and any intentional differences.

## Rust-specific rules

- Prefer borrowing over cloning when ownership is not needed.
- Do not fight the borrow checker by adding broad `clone`, `Arc`, `Mutex`, or `'static` bounds without design justification.
- Use `pub(crate)` for extracted helpers unless they are intentionally public.
- Keep module boundaries aligned with domain concepts, not just file size.
- Preserve feature-gated behavior and no-default-features builds.
- Avoid introducing trait objects or generic parameters unless they remove real duplication or enable necessary testing.
- Keep error types and variants stable for public APIs.
- Do not replace explicit errors with `anyhow` in library code.
- Do not add macros unless they remove substantial repetitive code and remain readable.

## Commands to run

Run before and after when feasible:

```bash
cargo test --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo check --all-targets --all-features
```

If public API or docs are affected:

```bash
cargo doc --all-features --no-deps
cargo test --doc --all-features
```

If features exist:

```bash
cargo check --no-default-features
cargo test --no-default-features
```

## Quality checklist

- Behavior is preserved or intentional changes are explicitly listed.
- Diff is minimal and scoped to the requested refactor.
- Public API is unchanged unless approved.
- Tests pass before and after or baseline failures are documented.
- Duplicated logic is removed without hiding important domain differences.
- Types are simpler for callers and maintainers.
- Error behavior and messages remain compatible unless intentionally changed.
- Feature-gated code still compiles.
- No unnecessary dependencies were added.
- No broad architecture rewrite was introduced.

## Anti-patterns to avoid

- Combining refactor, feature work, and bug fixes in one unreviewable diff.
- Replacing straightforward code with over-generic abstractions.
- Adding `clone`, `Arc`, `Mutex`, or `'static` to bypass ownership design.
- Moving public types between modules without preserving re-exports.
- Changing error variants or messages relied on by tests or users without noting it.
- Deleting tests because they make refactoring harder.
- Running formatters over unrelated files when the project does not expect it.

## Output format

Include:

- Refactor scope.
- Behavior preservation statement.
- Files changed.
- Public API impact.
- Tests and commands run.
- Any baseline failures or follow-up risks.

## Example user requests

- "Refactor this module without changing behavior."
- "Remove duplication in the parser."
- "Simplify these generic types."
- "Split this file into modules but keep the public API stable."
