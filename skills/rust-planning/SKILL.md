---
name: rust-planning
description: Use for planning Rust development work before implementation: breaking down tasks, identifying files to inspect, sequencing safe changes, choosing validation commands, surfacing assumptions, and defining done criteria. Trigger when the user asks for a plan, roadmap, implementation plan, migration plan, or task breakdown.
---

# rust-planning

## Purpose

Create concrete, executable plans for Rust development tasks that reduce risk before coding starts.

## When to use this skill

Use this skill when the user asks to:

- Plan a Rust feature, bug fix, refactor, migration, or review.
- Break work into safe steps.
- Define validation strategy.
- Decide which skills or workflows should be combined.
- Produce an implementation plan without editing code yet.

## Inputs expected from user

Read or request context depending on the task:

- `Cargo.toml` for workspace, features, dependencies.
- `README.md` for expected behavior.
- `src/lib.rs`, `src/main.rs`, target modules.
- `tests/`, `examples/`, `benches/`.
- `AGENTS.md` for local rules.
- Existing issue, bug report, compiler output, or feature description.

## Assumptions

A plan should be specific enough that Codex can execute it. If critical context is missing, list assumptions and the first files or commands required to validate them.

## Workflow

1. Restate the goal in one sentence.
2. Identify task type: API design, implementation, refactor, tests, compiler fix, CLI, observability, production readiness.
3. List context to inspect before coding.
4. Identify constraints:
   - public API compatibility;
   - feature flags;
   - dependency policy;
   - async runtime;
   - performance expectations;
   - deployment or CLI behavior.
5. Break work into small ordered steps.
6. Mark risk points and decision points.
7. Define tests to add or update.
8. Define validation commands.
9. Define done criteria.
10. If the user asked to proceed after planning, execute the plan using `rust-plan-execution` and task-specific skills.

## Rust-specific rules

- Plan public API changes separately from internal implementation.
- Include no-default-features and all-features checks for library crates with features.
- Include docs checks when public APIs change.
- Include CLI command examples when CLI behavior changes.
- Include async cancellation, backpressure, and shutdown checks for async work.
- Include observability validation when logs, metrics, or tracing are involved.
- Do not plan dependency additions without justification and alternatives.
- Prefer migration steps that keep the crate compiling between commits.

## Commands to run

A plan should choose from these commands, not necessarily run them immediately:

```bash
cargo check
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo doc --all-features --no-deps
cargo test --doc --all-features
```

For targeted work:

```bash
cargo check -p <crate-name>
cargo test <test_name>
cargo run -- <cli-args>
```

## Quality checklist

- Goal and non-goals are clear.
- Required context files are listed.
- Steps are ordered and executable.
- Public API and compatibility risks are called out.
- Tests and validation commands are defined.
- Dependencies and feature flags are considered.
- Async, CLI, and observability risks are included when relevant.
- Done criteria are measurable.
- Open questions are limited to decisions that truly block work.

## Anti-patterns to avoid

- Producing a vague plan that says "implement feature" without file-level steps.
- Mixing design debate with execution when the user only asked for a plan.
- Ignoring tests and validation until the end.
- Planning broad rewrites when a narrow change is enough.
- Omitting compatibility and feature-flag checks for library crates.
- Treating dependency additions as free.

## Output format

Include:

- Goal.
- Assumptions.
- Context to inspect.
- Ordered implementation steps.
- Tests to add or update.
- Commands to run.
- Risks and decisions.
- Done criteria.

## Example user requests

- "Plan how to implement this Rust feature."
- "Give me a migration plan for this crate API."
- "Break this refactor into safe steps."
- "Plan debugging for these compiler errors."
