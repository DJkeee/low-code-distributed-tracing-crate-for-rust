---
name: rust-plan-execution
description: Use when executing an existing plan for Rust work, following ordered steps, updating progress, preventing scope creep, validating after each stage, and reporting deviations. Trigger when the user says follow this plan, execute the plan, continue from plan, implement step-by-step, or do not deviate.
---

# rust-plan-execution

## Purpose

Execute a Rust development plan in a controlled way: one step at a time, with validation, explicit deviations, and no scope creep.

## When to use this skill

Use this skill when the user asks to:

- Follow an existing plan.
- Execute implementation steps in order.
- Continue a planned refactor or migration.
- Keep changes limited to a predefined scope.
- Report progress against a checklist.

## Inputs expected from user

Required or discoverable context:

- The plan or task list to execute.
- `Cargo.toml`, `README.md`, `src/lib.rs`, `src/main.rs`, relevant modules.
- `tests/`, `examples/`, and `AGENTS.md`.
- Any previous command failures or baseline test failures.

## Assumptions

The plan is the source of truth. If repository reality contradicts the plan, stop that step, document the mismatch, and choose the smallest correction or ask the user when the decision is material.

## Workflow

1. Convert the plan into a visible checklist.
2. Confirm current repository state with `git status --short` before editing.
3. Inspect files required for the next step only.
4. Execute one step at a time.
5. After each meaningful edit, run the narrowest relevant check.
6. Mark completed steps and record deviations.
7. Do not expand scope unless required to make the planned change compile or pass tests.
8. If unexpected unrelated changes appear, stop and ask the user how to proceed.
9. Run final validation commands from the plan.
10. Report completed steps, skipped steps, deviations, and verification.

## Rust-specific rules

- Keep the crate compiling between steps when practical.
- Preserve public API unless the plan explicitly includes a breaking change.
- Do not add dependencies unless the plan permits them or the need is unavoidable and explained.
- Apply task-specific skills as needed:
  - `rust-api-design` for public API steps.
  - `rust-code-implementation` for feature steps.
  - `rust-test-design` for test steps.
  - `rust-debug-compiler-errors` for compiler failures.
  - `rust-refactor` for behavior-preserving cleanup.
  - `rust-cli-design` for CLI steps.
  - `rust-observability` for tracing/logging/metrics steps.
- Keep tests aligned with each step's behavior change.
- Do not silence clippy with `allow` unless the lint is wrong for a documented reason.

## Commands to run

Start with:

```bash
git status --short
cargo check
```

Use targeted checks during execution:

```bash
cargo test <test_name>
cargo check -p <crate-name>
cargo test -p <crate-name>
```

Final validation should usually include:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo check --no-default-features
```

Add docs validation for public APIs:

```bash
cargo doc --all-features --no-deps
cargo test --doc --all-features
```

## Quality checklist

- Plan steps are tracked and completed in order or deviations are documented.
- Scope remains limited to the plan.
- Repository state was checked before editing.
- Unexpected unrelated changes were not overwritten.
- Each step has relevant validation.
- Final commands were run or blockers were reported.
- Public API and feature flags remain consistent with the plan.
- Tests cover behavior introduced by executed steps.

## Anti-patterns to avoid

- Starting broad implementation without mapping it to plan steps.
- Rewriting architecture because a local step is inconvenient.
- Skipping validation until the end.
- Continuing after unexpected unrelated file changes appear.
- Reverting user changes to make the plan easier.
- Treating compiler fixes as permission to alter public behavior.
- Adding dependencies outside the plan without explicit justification.

## Output format

Include:

- Plan checklist status.
- Files changed.
- Deviations from plan.
- Validation commands and results.
- Remaining steps or blockers.

## Example user requests

- "Execute this Rust implementation plan step by step."
- "Follow the plan and do not change scope."
- "Continue from step 3."
- "Implement this migration plan with validation after each step."
