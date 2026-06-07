---
name: rust-cli-design
description: Use for designing or implementing Rust CLI behavior, clap argument parsing, config precedence, stdout/stderr contracts, exit codes, machine-readable output, human-readable errors, logging verbosity, dry-run behavior, shell completions, and command UX.
---

# rust-cli-design

## Purpose

Design Rust CLIs that are predictable for humans, scriptable for automation, explicit about errors, and maintainable as the command surface grows.

## When to use this skill

Use this skill when the user asks to:

- Design a CLI in Rust.
- Add or change clap commands, flags, config, or output behavior.
- Improve CLI error UX.
- Add JSON output, dry-run, verbosity, or shell completion.
- Review CLI behavior for production use.

## Inputs expected from user

Read project context first:

- `Cargo.toml` for CLI dependencies and binary targets.
- `README.md` for documented commands.
- `src/main.rs`, `src/bin/`, and CLI modules.
- `src/lib.rs` if CLI wraps a library API.
- `tests/` for command tests.
- `examples/` for expected workflows.
- `AGENTS.md` for local conventions.

Useful user inputs:

- Target users: humans, scripts, CI, operators.
- Required output formats.
- Config file and environment variable expectations.
- Backward compatibility requirements.

## Assumptions

If the CLI is already public, preserve command names, flags, output contracts, and exit codes unless the user approves a breaking change.

## Workflow

1. Separate CLI parsing from business logic. Keep reusable logic in library or internal modules.
2. Define command hierarchy and names before implementation.
3. Define config precedence explicitly, usually:
   - command-line flags;
   - environment variables;
   - config file;
   - defaults.
4. Define stdout/stderr contract:
   - stdout for requested data;
   - stderr for diagnostics, progress, warnings, and errors.
5. Define exit codes:
   - `0` success;
   - `1` general failure;
   - `2` usage/config error when appropriate;
   - document additional codes if used.
6. Support machine-readable output only when its schema is stable and tested.
7. Add `--verbose`/`--quiet` behavior that maps cleanly to logging/tracing levels.
8. Add `--dry-run` only if it avoids side effects reliably and reports planned actions.
9. Add shell completion support if the command surface is non-trivial and `clap_complete` is acceptable.
10. Test parsing, output streams, exit codes, and failure UX.

## Rust-specific rules

- Use `clap` derive or builder consistently with the existing codebase.
- Keep parsing structs close to CLI modules; convert them into domain config types explicitly.
- Do not let `clap` types leak into library APIs.
- Use `std::process::ExitCode` or a consistent application error-to-exit-code mapping.
- Use `anyhow` in the binary boundary if useful; convert domain errors into user-friendly messages.
- Library code should still use typed errors where callers need inspection.
- Do not print errors with `println!`; use stderr.
- Avoid `unwrap`/`expect` in CLI production paths; return errors with context.
- Redact secrets in errors, logs, and dry-run output.
- Keep JSON output stable and avoid mixing logs into stdout when JSON mode is active.
- Test CLI with `assert_cmd`, `trycmd`, or existing project conventions if dependencies are allowed.

## Commands to run

Run validation:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo run -- --help
```

For each command changed, run examples such as:

```bash
cargo run -- <command> --help
cargo run -- <command> --dry-run
cargo run -- <command> --output json
```

If no-default-features matters:

```bash
cargo check --no-default-features
```

## Quality checklist

- CLI parsing is separated from domain logic.
- Config precedence is documented and tested.
- stdout contains machine-consumable or requested output only.
- stderr contains diagnostics and errors.
- Exit codes are intentional and tested.
- Human-readable errors include actionable context.
- Machine-readable output has stable schema and no logs mixed into stdout.
- Verbosity flags control logging consistently.
- Dry-run avoids side effects and reports planned changes.
- Help text is useful and examples are current.
- Public CLI changes are backward-compatible or clearly marked breaking.

## Anti-patterns to avoid

- Printing diagnostics to stdout.
- Mixing JSON output with logs or progress bars.
- Panicking on invalid user input.
- Encoding config precedence implicitly across multiple modules.
- Letting CLI argument structs become core domain models.
- Returning success exit code after partial failure without clear reporting.
- Adding flags without tests or help text.
- Logging secrets from environment variables or config files.

## Output format

Include:

- CLI behavior summary.
- Commands, flags, config precedence, and output contract.
- Exit code behavior.
- Files changed.
- Tests and manual commands run.
- Compatibility notes.

## Example user requests

- "Design a CLI for this crate using clap."
- "Add JSON output and keep logs on stderr."
- "Review CLI UX and exit codes."
- "Add dry-run and config precedence tests."
