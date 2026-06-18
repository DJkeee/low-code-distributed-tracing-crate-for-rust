# my-infra-otel

`my-infra-otel` is a lightweight Rust SDK for standardized distributed tracing in Axum/Tower services.

It does not replace OpenTelemetry. It provides a small golden-path layer over `tracing`, `tracing-subscriber`, `tracing-opentelemetry`, OTLP/HTTP, Tower middleware, and `reqwest` propagation.

## What It Provides

- One-call tracing initialization through `TracingConfig` and `init_global_tracing`.
- JSON stdout logs with `trace_id` and `span_id` when logs are emitted inside an active traced span.
- OTLP/HTTP trace export to an OpenTelemetry Collector.
- Tower middleware for inbound HTTP server spans.
- W3C `traceparent` extraction and injection.
- Policy-based header capture for request spans.
- `TracedHttpClient` wrapper for outbound `reqwest` calls.
- `record_event` helper for structured business events.
- Explicit `TracingGuard::shutdown()` for exporter flush on shutdown.

## MVP Limitations

- Metrics are not included.
- OTLP log export is not included; logs go to stdout.
- Baggage propagation is not included.
- Actix-web and Tonic middleware are not included.
- Detached `tokio::spawn` tasks need explicit context propagation by the application.
- Jaeger event logs may show dynamic business fields as `event.fields`; stdout JSON flattens them as top-level fields.

## Quick Start

```rust,no_run
use axum::{Router, routing::get};
use my_infra_otel::{MyOtelTracingLayer, TracingConfig, init_global_tracing};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = TracingConfig::builder("checkout-api").build()?;
    let guard = init_global_tracing(config)?;

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .layer(MyOtelTracingLayer::new());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    guard.shutdown()?;
    Ok(())
}
```

## Production-Like Config

```rust,no_run
use my_infra_otel::{TracingConfig, init_global_tracing};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let config = TracingConfig::builder("checkout-service")
    .environment("prod")
    .version("1.2.3")
    .otlp_endpoint("http://otel-collector:4318/v1/traces")
    .resource_attr("team", "platform")
    .resource_attr("datacenter", "dc-1")
    .log_filter("info,checkout_service=debug,my_infra_otel=debug")
    .build()?;

let guard = init_global_tracing(config)?;
# guard.shutdown()?;
# Ok(())
# }
```

## Header Capture Policy

```rust,no_run
use my_infra_otel::{HeaderCapturePolicy, HeaderValueMode, MyOtelTracingLayer};

# fn build_layer() -> Result<MyOtelTracingLayer, my_infra_otel::MyOtelError> {
let headers = HeaderCapturePolicy::builder()
    .standard_request_ids()
    .gateway_headers()
    .header("x-tenant-id", "tenant.id")
    .header_with("x-user-id", "user.id", HeaderValueMode::Redacted)
    .max_value_len(128)
    .build()?;

let layer = MyOtelTracingLayer::builder()
    .headers(headers)
    .build()?;
# Ok(layer)
# }
```

Only allowlisted headers are copied into the request span. Sensitive headers such as `authorization`, `cookie`, and `x-api-key` are rejected by the policy builder. Custom headers use truncated values by default; use `Redacted` or `Present` for higher-risk identifiers.

## Outbound HTTP Propagation

```rust,no_run
use my_infra_otel::TracedHttpClient;

# async fn call_downstream() -> Result<(), my_infra_otel::MyOtelError> {
let client = TracedHttpClient::new(reqwest::Client::new());

let response = client
    .get("http://127.0.0.1:3001/process")
    .send()
    .await?;
# let _ = response;
# Ok(())
# }
```

`TracedHttpClient` creates a client span and injects W3C `traceparent` into outgoing headers.

## Business Events

```rust
use my_infra_otel::{EventField, record_event};

record_event(
    "checkout.started",
    [
        EventField::string("operation.name", "checkout"),
        EventField::string("operation.result", "started"),
    ],
);
```

When called inside a traced request span, stdout JSON logs include `trace_id`, `span_id`, `event.name`, and the provided business fields.

## Local Demo

Requirements:

- Rust stable toolchain.
- Docker with `docker compose`.
- Free ports: `3000`, `3001`, `3002`, `3003`, `4318`, `16686`.

Start the full demo stack:

```bash
docker compose -f infra/docker-compose.yml up -d
```

This starts Jaeger, OpenTelemetry Collector, `checkout-api`, `order-processor`, `risk-engine`, and `demo-ui`.

Open:

```text
http://127.0.0.1:3002
```

For local manual runs, start only Collector and Jaeger:

```bash
docker compose -f infra/docker-compose.yml up -d jaeger otel-collector
```

Run risk-engine:

```bash
RUST_LOG=info cargo run -p risk-engine
```

Run order-processor:

```bash
RISK_ENGINE_URL=http://127.0.0.1:3003 RUST_LOG=info cargo run -p order-processor
```

Run checkout-api in another terminal:

```bash
ORDER_PROCESSOR_URL=http://127.0.0.1:3001 RISK_ENGINE_URL=http://127.0.0.1:3003 RUST_LOG=info cargo run -p checkout-api
```

Run the demo UI in a third terminal:

```bash
RUST_LOG=info cargo run -p demo-ui
```

The UI can also be configured with:

```bash
DEMO_UI_BIND=127.0.0.1:3002 \
CHECKOUT_API_URL=http://127.0.0.1:3000 \
ORDER_PROCESSOR_URL=http://127.0.0.1:3001 \
RISK_ENGINE_URL=http://127.0.0.1:3003 \
JAEGER_URL=http://localhost:16686 \
RUST_LOG=info cargo run -p demo-ui
```

Call the demo flow:

```bash
curl \
  -H "X-User-ID: 42" \
  -H "X-Request-ID: req-123" \
  -H "X-Tenant-ID: demo" \
  http://127.0.0.1:3000/checkout
```

Expected response:

```json
{"checkout_id":"checkout-...","duration_ms":123.4,"pricing":{"currency":"USD","discount_cents":1250,"total_cents":12450},"service_b":{"duration_ms":90.1,"service_c":{"compute_iterations":180000,"decision":"approved","duration_ms":70.2,"raw_risk_score":42,"risk_score":49,"status":"risk_evaluated"},"status":"processed"},"service_c_direct":{"compute_iterations":180000,"decision":"approved","duration_ms":70.2,"raw_risk_score":42,"risk_score":49,"status":"risk_evaluated"},"status":"ok"}
```

Expected trace shape:

```text
checkout-api /checkout
  checkout.validate_cart
  checkout.authenticate_customer
  checkout.reserve_inventory
  checkout.price_order
  checkout.request_direct_risk_quote
    checkout-api -> risk-engine client span
      risk-engine /quote-risk
        quote-risk.load_risk_model
        quote-risk.prepare_risk_features
        quote-risk.compute_raw_risk_score
        quote-risk.calibrate_risk_score
        quote-risk.explain_risk_decision
        quote-risk.persist_risk_decision
  checkout.authorize_payment
  checkout.request_order_processing
    checkout-api -> order-processor client span
      order-processor /process
        process.normalize_request
        process.enrich_order
        process.request_risk_quote
          order-processor -> risk-engine client span
            risk-engine /quote-risk
        process.allocate_shipping
        process.write_ledger
        process.finalize
```

Open Jaeger UI:

```text
http://localhost:16686
```

Select `checkout-api` and inspect the latest trace. The `checkout-api` server span should contain configured header capture attributes such as `user.id`, `request.id`, and `tenant.id`.

The demo UI performs this lookup automatically after pressing `Run checkout trace`. It displays:

- the matched `trace_id`;
- all spans returned by Jaeger;
- service name, operation, span id, parent span id, start offset, duration;
- all span tags, including `request.id`, `user.id`, `tenant.id`, `operation.name`, `step`, and `downstream.service`;
- a direct link to the trace in Jaeger.

## Validation Commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo check -p my-infra-otel --no-default-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
```

## Troubleshooting

### No `trace_id` In Stdout Logs

The log must be emitted inside an active traced span. For HTTP requests, ensure the route is wrapped with `MyOtelTracingLayer` and the log/event is emitted inside the handler future.

### No Traces In Jaeger

Check Collector logs:

```bash
docker compose -f infra/docker-compose.yml logs --tail=100 otel-collector
```

Verify the service OTLP endpoint. The local default is:

```text
http://localhost:4318/v1/traces
```

### `/checkout` Returns `502`

Ensure `order-processor` is running on:

```text
http://127.0.0.1:3001
```

If `order-processor` runs elsewhere, start `checkout-api` with `ORDER_PROCESSOR_URL`.

Also ensure `risk-engine` is running on:

```text
http://127.0.0.1:3003
```

If `risk-engine` runs elsewhere, start `checkout-api`, `order-processor`, and `demo-ui` with `RISK_ENGINE_URL`.

### Double Initialization

`init_global_tracing` installs a global subscriber. Calling it more than once in the same process returns `MyOtelError::AlreadyInitialized`.

### Detached Tokio Tasks

Trace context is not automatically guaranteed across detached tasks. Capture and propagate context explicitly when spawning independent work.

## Future Work

- Automated live smoke tests for Collector and Jaeger.
- Metrics support.
- Baggage support.
- Additional framework middleware.
- Environment/file config helpers.
