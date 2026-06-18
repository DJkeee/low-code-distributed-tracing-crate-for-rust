# my-infra-otel

`my-infra-otel` is a lightweight Rust SDK for standardized distributed tracing in Axum/Tower services.

It does not replace OpenTelemetry. It provides a small golden-path layer over `tracing`, `tracing-subscriber`, `tracing-opentelemetry`, OTLP/HTTP, Tower middleware, and `reqwest` propagation.

## What It Provides

- One-call tracing initialization through `TracingConfig` and `init_global_tracing`.
- JSON stdout logs with `trace_id` and `span_id` when logs are emitted inside an active traced span.
- OTLP/HTTP trace export to an OpenTelemetry Collector.
- Tower middleware for inbound HTTP server spans.
- W3C `traceparent` extraction and injection.
- Configurable header labels copied into request spans.
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
    let config = TracingConfig::builder("service-a").build()?;
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

## Header Labels

```rust,no_run
use my_infra_otel::MyOtelTracingLayer;

# fn build_layer() -> Result<MyOtelTracingLayer, my_infra_otel::MyOtelError> {
let layer = MyOtelTracingLayer::builder()
    .header_attr("x-user-id", "user.id")
    .header_attr("x-request-id", "request.id")
    .header_attr("x-tenant-id", "tenant.id")
    .build()?;
# Ok(layer)
# }
```

Configured headers are copied into the request span when present. Missing configured headers are ignored.

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

This starts Jaeger, OpenTelemetry Collector, `service-a`, `service-b`, `service-c`, and `demo-ui`.

Open:

```text
http://127.0.0.1:3002
```

For local manual runs, start only Collector and Jaeger:

```bash
docker compose -f infra/docker-compose.yml up -d jaeger otel-collector
```

Run service C:

```bash
RUST_LOG=info cargo run -p service-c
```

Run service B:

```bash
SERVICE_C_URL=http://127.0.0.1:3003 RUST_LOG=info cargo run -p service-b
```

Run service A in another terminal:

```bash
SERVICE_B_URL=http://127.0.0.1:3001 SERVICE_C_URL=http://127.0.0.1:3003 RUST_LOG=info cargo run -p service-a
```

Run the demo UI in a third terminal:

```bash
RUST_LOG=info cargo run -p demo-ui
```

The UI can also be configured with:

```bash
DEMO_UI_BIND=127.0.0.1:3002 \
SERVICE_A_URL=http://127.0.0.1:3000 \
SERVICE_B_URL=http://127.0.0.1:3001 \
SERVICE_C_URL=http://127.0.0.1:3003 \
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
service-a /checkout
  checkout.validate_cart
  checkout.authenticate_customer
  checkout.reserve_inventory
  checkout.price_order
  checkout.call_risk_engine
    service-a -> service-c client span
      service-c /quote-risk
        quote-risk.load_model
        quote-risk.prepare_features
        quote-risk.compute_score
        quote-risk.calibrate_score
        quote-risk.explain_decision
        quote-risk.persist_decision
  checkout.authorize_payment
  checkout.call_processing
    service-a -> service-b client span
      service-b /process
        process.normalize_request
        process.enrich_order
        process.call_risk_engine
          service-b -> service-c client span
            service-c /quote-risk
        process.allocate_shipping
        process.write_ledger
        process.finalize
```

Open Jaeger UI:

```text
http://localhost:16686
```

Select `service-a` and inspect the latest trace. The `service-a` server span should contain configured header labels such as `user.id`, `request.id`, and `tenant.id`.

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

Ensure `service-b` is running on:

```text
http://127.0.0.1:3001
```

If `service-b` runs elsewhere, start `service-a` with `SERVICE_B_URL`.

Also ensure `service-c` is running on:

```text
http://127.0.0.1:3003
```

If `service-c` runs elsewhere, start `service-a`, `service-b`, and `demo-ui` with `SERVICE_C_URL`.

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
