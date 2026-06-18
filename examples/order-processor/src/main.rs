use std::time::{Duration, Instant};

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use my_infra_otel::{EventField, TracedHttpClient, init_global_tracing, record_event};
use serde_json::{Value, json};
use tokio::time::sleep;
use tracing::Instrument;

mod tracing_setup;

#[derive(Clone)]
struct AppState {
    client: TracedHttpClient,
    risk_engine_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = tracing_setup::tracing_config("order-processor")?;
    let _guard = init_global_tracing(config)?;
    let layer = tracing_setup::tracing_layer()?;

    let state = AppState {
        client: TracedHttpClient::new(
            reqwest::Client::builder()
                .timeout(Duration::from_secs(3))
                .build()?,
        ),
        risk_engine_url: env_or_default(
            "RISK_ENGINE_URL",
            "SERVICE_C_URL",
            "http://127.0.0.1:3003",
        ),
    };

    let app = Router::new()
        .route("/process", get(process_order))
        .route("/health", get(health))
        .with_state(state)
        .layer(layer);

    let bind = env_or_default("ORDER_PROCESSOR_BIND", "SERVICE_B_BIND", "127.0.0.1:3001");
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "order-processor listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn process_order(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let started_at = Instant::now();

    record_event(
        "process.started",
        [EventField::string("operation.name", "process")],
    );

    normalize_request().await;
    enrich_order().await;
    let risk = request_risk_quote(&state).await.map_err(internal_error)?;
    allocate_shipping().await;
    write_ledger().await;
    finalize_processing().await;

    let duration_ms = started_at.elapsed().as_secs_f64() * 1000.0;
    record_event(
        "process.completed",
        [
            EventField::string("operation.name", "process"),
            EventField::string("operation.result", "success"),
            EventField::i64("duration.ms", duration_ms.round() as i64),
        ],
    );

    Ok(Json(json!({
        "status": "processed",
        "service_c": risk,
        "duration_ms": duration_ms,
    })))
}

async fn normalize_request() {
    async {
        sleep(Duration::from_millis(35)).await;
        record_event(
            "process.normalized",
            [
                EventField::string("operation.name", "process.normalize_request"),
                EventField::string("operation.result", "success"),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "process.normalize_request",
        component = "processor",
        operation.name = "process.normalize_request",
        step = "normalize_request",
    ))
    .await;
}

async fn enrich_order() {
    async {
        sleep(Duration::from_millis(45)).await;
        record_event(
            "process.order.enriched",
            [
                EventField::string("operation.name", "process.enrich_order"),
                EventField::i64("enrichment.rule_count", 8),
                EventField::string("operation.result", "success"),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "process.enrich_order",
        component = "processor",
        operation.name = "process.enrich_order",
        step = "enrich_order",
    ))
    .await;
}

async fn request_risk_quote(state: &AppState) -> Result<Value, my_infra_otel::MyOtelError> {
    async {
        let response = state
            .client
            .get(format!("{}/quote-risk", state.risk_engine_url))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            record_event(
                "process.risk.failed",
                [
                    EventField::string("operation.name", "process.request_risk_quote"),
                    EventField::string("downstream.service", "risk-engine"),
                    EventField::i64("downstream.status_code", i64::from(status.as_u16())),
                ],
            );
            return match response.error_for_status() {
                Ok(_) => Ok(Value::Null),
                Err(err) => Err(err.into()),
            };
        }
        let body = response.json::<Value>().await?;

        record_event(
            "process.risk.completed",
            [
                EventField::string("operation.name", "process.request_risk_quote"),
                EventField::string("downstream.service", "risk-engine"),
                EventField::i64("downstream.status_code", i64::from(status.as_u16())),
            ],
        );

        Ok(body)
    }
    .instrument(tracing::info_span!(
        "process.request_risk_quote",
        component = "processor",
        operation.name = "process.request_risk_quote",
        step = "request_risk_quote",
        downstream.service = "risk-engine",
    ))
    .await
}

async fn allocate_shipping() {
    async {
        sleep(Duration::from_millis(50)).await;
        record_event(
            "process.shipping.allocated",
            [
                EventField::string("operation.name", "process.allocate_shipping"),
                EventField::string("shipping.carrier", "demo-express"),
                EventField::i64("shipping.eta_days", 2),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "process.allocate_shipping",
        component = "shipping",
        operation.name = "process.allocate_shipping",
        step = "allocate_shipping",
    ))
    .await;
}

async fn write_ledger() {
    async {
        sleep(Duration::from_millis(40)).await;
        record_event(
            "process.ledger.written",
            [
                EventField::string("operation.name", "process.write_ledger"),
                EventField::string("ledger.partition", "orders-2026"),
                EventField::string("operation.result", "committed"),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "process.write_ledger",
        component = "ledger",
        operation.name = "process.write_ledger",
        step = "write_ledger",
    ))
    .await;
}

async fn finalize_processing() {
    async {
        sleep(Duration::from_millis(30)).await;
        record_event(
            "process.finalized",
            [
                EventField::string("operation.name", "process.finalize"),
                EventField::string("operation.result", "success"),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "process.finalize",
        component = "processor",
        operation.name = "process.finalize",
        step = "finalize",
    ))
    .await;
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

fn internal_error(err: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({
            "error": "risk engine request failed",
            "detail": err.to_string(),
        })),
    )
}

fn env_or_default(primary: &str, legacy: &str, default: &str) -> String {
    std::env::var(primary)
        .or_else(|_| std::env::var(legacy))
        .unwrap_or_else(|_| default.to_owned())
}
