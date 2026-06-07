---
name: rust-api-design
description: Use for designing or changing public Rust library APIs, backend component boundaries, crate structure, error models, feature flags, semver-compatible interfaces, and examples. Trigger when the task mentions public API, crate API, library design, module boundaries, builders, traits, feature flags, semver, or API ergonomics.
---

# rust-api-design

## Purpose

Design production-ready Rust APIs that are stable, ergonomic, documented, testable, and semver-aware.

## When to use this skill

Use this skill when the user asks to:

- Design a public API for a Rust crate, library, SDK, or backend component.
- Add or change exported structs, traits, functions, modules, feature flags, or error types.
- Review API ergonomics before implementation.
- Preserve backward compatibility while evolving an API.
- Split internal implementation from public API.

## Inputs expected from user

Ask for missing context only when it blocks the design. Prefer reading project files first:

- `Cargo.toml` for crate type, features, dependencies, MSRV hints, workspace layout.
- `README.md` for intended user-facing behavior.
- `src/lib.rs` and module files for public exports.
- `src/main.rs` only if the crate exposes both CLI and library APIs.
- `examples/` for expected usage patterns.
- `tests/` for compatibility expectations.
- `AGENTS.md` for repository-specific constraints.

Useful user inputs:

- Target users of the API.
- Stability expectations and semver constraints.
- Required sync/async behavior.
- Error handling expectations.
- Whether breaking changes are allowed.

## Assumptions

If the user does not explicitly allow breaking changes, preserve backward compatibility. If the crate has no documented MSRV, do not introduce language features or dependencies that unnecessarily raise the MSRV.

## Workflow

1. Inspect `Cargo.toml`, `README.md`, `src/lib.rs`, existing modules, `examples/`, and tests.
2. Classify every proposed item as public API, crate-private API, or internal implementation.
3. Define the smallest public surface that supports the use case.
4. Prefer stable concrete types for common paths and traits only where extension points are required.
5. Decide constructor strategy:
   - Use `new` for simple required fields.
   - Use a builder for optional settings, validation, or future-compatible configuration.
   - Keep builder defaults explicit and documented.
6. Define an error model before coding:
   - Library APIs should return typed errors that callers can inspect.
   - Applications may use `anyhow` at top-level boundaries.
   - Avoid stringly typed errors for public library APIs.
7. Define feature flags only for optional dependencies, runtime integrations, or clearly separable capabilities.
8. Add crate-level docs with one minimal working example.
9. Add examples for main user flows and feature-gated behavior.
10. Validate API names, ownership, borrowing, lifetimes, and async boundaries with real call-site examples.
11. Run formatting, linting, tests, and docs checks.

## Rust-specific rules

- Keep public exports centralized and intentional in `src/lib.rs`.
- Use `pub(crate)` for implementation shared inside the crate.
- Do not expose internal modules because tests or examples need them; prefer public behavior tests or crate-private unit tests.
- Prefer borrowing inputs as `&str`, `&Path`, `&[T]`, or `impl AsRef<_>` when the API does not need ownership.
- Prefer owned return values when returning borrowed values would leak internal lifetimes or make callers fight the borrow checker.
- Avoid unnecessary lifetime parameters in public API; use elision where possible.
- Avoid exposing generic parameters unless they materially improve usability or performance.
- Prefer `impl Trait` in arguments for flexibility; use named generics when bounds must be reused.
- Do not expose dependency-specific types in public API unless integration with that dependency is the purpose of the feature.
- Use `#[non_exhaustive]` on public enums or structs when future variants or fields are likely.
- Avoid public fields unless direct construction and mutation are part of the API contract.
- Use `thiserror` for typed library errors when a dependency is justified; otherwise implement `std::error::Error` directly.
- Use `anyhow` only in binaries, examples, tests, or application composition layers.
- Do not use `unwrap` or `expect` in library code except for invariants that cannot fail; document the invariant near the call.
- Feature names must be additive and documented. Avoid mutually exclusive features unless unavoidable.
- Public async APIs must state runtime assumptions. Avoid hard-coding a runtime unless required.
- Public APIs must have docs for behavior, errors, panics, cancellation, and feature flags where relevant.

## Commands to run

Run applicable commands from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo doc --all-features --no-deps
cargo check --all-targets --all-features
```

If feature combinations are important, run targeted checks:

```bash
cargo check --no-default-features
cargo check --features <feature-name>
cargo test --features <feature-name>
```

## Quality checklist

- Public/private boundaries are explicit and minimal.
- Public exports are intentional and documented.
- API examples compile and represent real usage.
- Backward compatibility is preserved unless the user approved a breaking change.
- Error types are inspectable and do not hide recoverable details.
- Constructors validate invalid states early.
- Builder APIs have documented defaults.
- Feature flags are additive, documented, and tested.
- The API does not force unnecessary clones, allocations, or lifetime parameters.
- Async APIs document runtime, cancellation, and shutdown behavior where relevant.
- Crate-level documentation explains purpose, quick start, features, and error model.
- Tests cover public behavior, not internal implementation details only.

## Anti-patterns to avoid

- Adding public exports because implementation files need sharing.
- Breaking semver compatibility without explicit user approval.
- Exposing internal dependency types accidentally.
- Using `Box<dyn Error>` or `anyhow::Error` in public library APIs without a clear boundary reason.
- Adding feature flags that change existing behavior silently.
- Designing trait-heavy APIs before concrete use cases exist.
- Returning references tied to internal temporary values.
- Using `String` everywhere instead of accepting `&str` or `impl Into<String>` deliberately.
- Panicking on user input in library code.
- Adding dependencies for trivial helpers.

## Output format

When responding after using this skill, include:

- API design summary.
- Public API changes with file references.
- Compatibility notes, including breaking changes if any.
- Error model and feature flag decisions.
- Tests, docs, and commands run.
- Open questions only if required decisions remain.

## Example user requests

- "Design a public API for this tracing crate."
- "Refactor `src/lib.rs` exports without breaking users."
- "Add a builder for client configuration."
- "Review this crate API for semver and ergonomics."
