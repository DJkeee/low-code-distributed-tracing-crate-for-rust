---
name: rust-observability
description: Use for adding or reviewing Rust observability: tracing, structured logs, spans, metrics, OpenTelemetry compatibility, request or trace IDs, redaction, cardinality control, production diagnostics, and logging configuration.
---

# rust-observability

## Purpose

Add production-grade observability to Rust code with structured tracing, useful metrics, safe redaction, bounded cardinality, and diagnostics that help operate the system.

## When to use this skill

Use this skill when the user asks to:

- Add logs, tracing, spans, metrics, or OpenTelemetry support.
- Improve production diagnostics.
- Add request IDs, trace IDs, or correlation fields.
- Review logging for security, cardinality, or operational usefulness.
- Instrument async Rust services, clients, queues, or CLI commands.

## Inputs expected from user

Read project context first:

- `Cargo.toml` for observability dependencies and features.
- `README.md` for operational expectations.
- `src/lib.rs`, `src/main.rs`, service/client modules.
- Existing logging/tracing initialization.
- `tests/` for behavior and diagnostics tests.
- `AGENTS.md` for local conventions.

Useful user inputs:

- Runtime environment and log collector.
- Required metric backend.
- Trace propagation requirements.
- Data redaction rules.

## Assumptions

Do not add OpenTelemetry, metrics exporters, or logging dependencies unless they are already present or the user approves them. Instrumentation must not expose secrets or create unbounded-cardinality labels.

## Workflow

1. Identify operational questions the instrumentation must answer.
2. Locate request/task boundaries and long-running operations.
3. Add spans around meaningful units of work, not every helper function.
4. Use structured fields instead of formatted strings for IDs, counts, durations, and statuses.
5. Propagate request ID or trace context through async boundaries where relevant.
6. Add metrics for rates, errors, latency, queue depth, retries, and saturation where useful.
7. Keep metric labels low-cardinality.
8. Redact secrets and user-sensitive data before logging.
9. Ensure initialization is owned by binaries/apps, not libraries, unless the crate is explicitly an app framework.
10. Test or manually verify logs/metrics for representative success and failure paths.

## Rust-specific rules

- Prefer `tracing` macros: `trace!`, `debug!`, `info!`, `warn!`, `error!` with structured fields.
- Use `#[instrument]` selectively; skip large or sensitive arguments with `skip(...)`.
- Avoid logging secrets, tokens, full payloads, PII, or unbounded user input.
- Use fields such as `request_id`, `trace_id`, `span_id`, `operation`, `status`, `error.kind`, `retry_count`, and `duration_ms`.
- Record errors with source context, not only `error = %err` if classification is available.
- Do not initialize global subscribers in library code except in tests or examples.
- For libraries, emit spans/events and let applications configure subscribers/exporters.
- For async code, ensure spans are entered correctly using `.instrument(span)` or `#[instrument]` rather than holding guards across `.await` incorrectly.
- Avoid high-cardinality metric labels such as user ID, request ID, path with raw IDs, SQL text, or error messages.
- Use histograms for latency and sizes; counters for totals; gauges for current depth or saturation.
- Make observability feature-gated if it introduces optional dependencies.

## Commands to run

Run validation:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo check --no-default-features
```

For examples or binaries:

```bash
RUST_LOG=debug cargo run -- <args>
RUST_LOG=trace cargo test <test_name> -- --nocapture
```

If docs or feature flags changed:

```bash
cargo doc --all-features --no-deps
cargo check --features <observability-feature>
```

## Quality checklist

- Spans match operational boundaries.
- Events use structured fields, not only formatted text.
- Request ID or trace context is propagated where needed.
- Errors include classification and useful context.
- Metrics answer concrete operational questions.
- Metric labels are bounded-cardinality.
- Secrets and PII are redacted.
- Library code does not own global subscriber initialization.
- Async instrumentation preserves spans across `.await`.
- Feature flags and dependencies are justified.
- Tests or manual verification cover success and failure diagnostics.

## Anti-patterns to avoid

- Adding noisy logs to every function.
- Logging raw request/response bodies by default.
- Logging secrets, tokens, passwords, API keys, cookies, or authorization headers.
- Using request IDs, user IDs, or error messages as metric labels.
- Initializing `tracing_subscriber` inside a reusable library.
- Formatting all log data into strings instead of structured fields.
- Holding span guards across `.await` incorrectly.
- Adding OpenTelemetry exporters without a deployment requirement.

## Output format

Include:

- Observability goals addressed.
- Spans, events, metrics, and fields added.
- Redaction and cardinality decisions.
- Files changed.
- Commands run and any manual diagnostics checked.
- Remaining production diagnostics gaps.

## Example user requests

- "Add tracing to this async pipeline."
- "Improve production logging and metrics."
- "Review observability for cardinality and secrets."
- "Add request IDs and structured logs."
