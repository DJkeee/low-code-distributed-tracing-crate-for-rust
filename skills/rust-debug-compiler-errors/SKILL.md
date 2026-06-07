---
name: rust-debug-compiler-errors
description: Use for fixing Rust compiler errors including borrow checker failures, lifetime errors, trait bounds, generic constraints, async lifetime issues, moved values, type inference errors, Send/Sync issues, and minimal compile-fix strategies.
---

# rust-debug-compiler-errors

## Purpose

Fix Rust compiler errors with minimal, behavior-preserving changes that address the actual ownership, lifetime, trait, or type issue instead of masking it.

## When to use this skill

Use this skill when the user provides or asks about:

- Borrow checker errors.
- Lifetime errors.
- Trait bound or generic constraint failures.
- Async lifetime or `Send` errors.
- Moved value, partial move, or type inference errors.
- A failing `cargo check`, `cargo build`, `cargo test`, or `cargo clippy` output.

## Inputs expected from user

Prefer running compiler commands and reading code locally:

- Full compiler error output.
- `Cargo.toml` for features and dependencies.
- The files and functions mentioned by diagnostics.
- Related trait definitions and impls.
- `src/lib.rs`, `src/main.rs`, tests that trigger the compile failure.
- `AGENTS.md` for local constraints.

## Assumptions

Use minimal fixes first. Do not redesign architecture, add broad clones, or add `'static` bounds unless the compiler error proves the value must outlive the current scope.

## Workflow

1. Run the narrowest command that reproduces the error.
2. Read the first compiler error and relevant notes; later errors may be cascading.
3. Map the diagnostic to the underlying category:
   - ownership move;
   - borrow overlap;
   - lifetime escape;
   - missing trait bound;
   - type inference ambiguity;
   - async `Send` or `'static` requirement;
   - feature-gated missing item.
4. Inspect surrounding code and call sites before editing.
5. Choose the least invasive fix:
   - shorten borrow scope;
   - reorder operations;
   - borrow instead of move;
   - return owned data instead of invalid references;
   - add precise trait bounds;
   - introduce named lifetime only when elision cannot express the relationship;
   - split async work to avoid holding borrows across `.await`.
6. Apply one conceptual fix at a time.
7. Re-run the reproducing command.
8. If new errors appear, distinguish real follow-up errors from cascades.
9. Run formatting, clippy, and tests after compile succeeds.

## Rust-specific rules

- Prefer narrowing scopes over cloning.
- Prefer changing function signatures from owned to borrowed only when callers do not need ownership transfer.
- Prefer returning owned values when returning references would expose invalid or complex lifetimes.
- Do not add `'static` to silence lifetime errors unless data is truly owned by a spawned task, global, or long-lived runtime boundary.
- For `tokio::spawn`, move owned `Send + 'static` values into the task or use scoped task patterns where available.
- Do not hold `MutexGuard`, `RwLockGuard`, or non-`Send` values across `.await` unless runtime and guard semantics allow it.
- Add trait bounds at the narrowest function, impl, or type where the bound is needed.
- Avoid blanket trait bounds on public types if only one method needs the bound.
- Use turbofish or local type annotations for inference errors when they document intent.
- Check feature flags before assuming an item is missing.
- Preserve error behavior while fixing compile errors.

## Commands to run

Start narrow:

```bash
cargo check
cargo check -p <crate-name>
cargo test <test_name> --no-run
```

Then validate broadly:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo check --no-default-features
```

For feature-specific failures:

```bash
cargo check --features <feature-name>
cargo test --features <feature-name>
```

## Quality checklist

- The first real compiler error is addressed, not only a downstream symptom.
- Fix is minimal and behavior-preserving.
- No unnecessary `clone`, `Arc`, `Mutex`, `Box`, or `'static` was introduced.
- Trait bounds are precise and placed at the narrowest useful scope.
- Lifetimes express a real relationship and are not decorative.
- Async fixes respect cancellation, task lifetime, and `Send` requirements.
- Public API compatibility is preserved unless explicitly approved.
- Compiler, clippy, and tests pass or remaining failures are documented.

## Anti-patterns to avoid

- Adding `.clone()` everywhere to satisfy the borrow checker.
- Adding `'static` bounds without understanding the lifetime boundary.
- Boxing futures or trait objects only to hide type errors.
- Replacing typed errors with `anyhow` to avoid generic constraints in a library.
- Changing public API unnecessarily to fix an internal compile error.
- Ignoring the compiler's notes and help messages.
- Fixing later cascading errors before the first root error.

## Output format

Include:

- Root cause category.
- Minimal fix applied.
- Files changed.
- Commands run and results.
- Any remaining compiler or clippy issues.

## Example user requests

- "Fix this borrow checker error."
- "Cargo check fails with a lifetime error."
- "Resolve this trait bound error without redesigning the API."
- "Fix async Send errors in this task spawning code."
