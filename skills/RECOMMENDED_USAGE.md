# Recommended usage

## Which skill to use for which task

- `rust-planning`: use before non-trivial Rust work when the next steps, risks, or validation strategy are unclear.
- `rust-plan-execution`: use when a plan already exists and changes must be made step by step without scope creep.
- `rust-code-implementation`: use for writing production-ready Rust code, implementing features, and fixing bugs.
- `rust-api-design`: use for public library APIs, backend component boundaries, builders, feature flags, error models, and semver decisions.
- `rust-code-review`: use for PR/diff review and maintainer-style risk assessment.
- `rust-refactor`: use for behavior-preserving cleanup, deduplication, module restructuring, and type simplification.
- `rust-test-design`: use for unit tests, integration tests, async tests, property tests, fixtures, fakes, and regression coverage.
- `rust-debug-compiler-errors`: use for borrow checker, lifetime, trait bound, generic, type inference, and async `Send`/`'static` compiler errors.
- `rust-cli-design`: use for Rust CLIs, clap, config precedence, stdout/stderr, exit codes, JSON output, verbosity, dry-run, and shell completion.
- `rust-observability`: use for `tracing`, structured logs, spans, metrics, trace/request IDs, redaction, OpenTelemetry compatibility, and production diagnostics.

## How to combine skills

- Feature work: `rust-planning` -> `rust-api-design` if public API changes -> `rust-code-implementation` -> `rust-test-design` -> `rust-code-review`.
- Bug fix: `rust-debug-compiler-errors` or `rust-code-implementation` -> `rust-test-design` for regression tests -> `rust-code-review`.
- Refactor: `rust-planning` for risky refactors -> `rust-refactor` -> `rust-test-design` -> `rust-code-review`.
- CLI change: `rust-cli-design` -> `rust-code-implementation` -> `rust-test-design` -> `rust-code-review`.
- Observability change: `rust-observability` -> `rust-code-implementation` -> `rust-test-design` for diagnostics checks -> `rust-code-review`.
- Existing plan execution: `rust-plan-execution` as the controller, invoking task-specific skills only for individual steps.

## Minimal starter set

Use these first for most Rust repositories:

- `rust-planning`
- `rust-code-implementation`
- `rust-test-design`
- `rust-debug-compiler-errors`
- `rust-code-review`

Add these when the project needs them:

- `rust-api-design` for crates and stable backend interfaces.
- `rust-cli-design` for binaries and operator tooling.
- `rust-observability` for services, async workers, distributed systems, and production diagnostics.
- `rust-refactor` when behavior must stay stable while structure changes.
- `rust-plan-execution` when a plan must be followed exactly.

## Improvements for later

- Add `references/` files with project-specific API stability policy, MSRV policy, dependency policy, and observability conventions.
- Add command wrappers under `scripts/` for standard validation profiles such as quick check, full CI, feature matrix, and docs.
- Add crate templates for common patterns: typed errors, builders, CLI parsing, async shutdown, and tracing setup.
- Add organization-specific review severity definitions and merge gates.
- Add examples of accepted output for each skill based on real repository tasks.
