use std::time::{Duration, Instant};

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use my_infra_otel::{
    EventField, MyOtelTracingLayer, TracedHttpClient, TracingConfig, init_global_tracing,
    record_event,
};
use serde_json::{Value, json};
use tokio::time::sleep;
use tracing::Instrument;

#[derive(Clone)]
struct AppState {
    client: TracedHttpClient,
    service_b_url: String,
    service_c_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = tracing_config("service-a")?;
    let _guard = init_global_tracing(config)?;

    let layer = MyOtelTracingLayer::builder()
        .header_attr("x-user-id", "user.id")
        .header_attr("x-request-id", "request.id")
        .header_attr("x-tenant-id", "tenant.id")
        .build()?;

    let state = AppState {
        client: TracedHttpClient::new(
            reqwest::Client::builder()
                .timeout(Duration::from_secs(3))
                .build()?,
        ),
        service_b_url: std::env::var("SERVICE_B_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3001".to_owned()),
        service_c_url: std::env::var("SERVICE_C_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3003".to_owned()),
    };

    let app = Router::new()
        .route("/checkout", get(checkout))
        .route("/health", get(health))
        .with_state(state)
        .layer(layer);

    let bind = std::env::var("SERVICE_A_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "service-a listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn checkout(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let started_at = Instant::now();
    let checkout_id = checkout_id();

    record_event(
        "checkout.started",
        [
            EventField::string("operation.name", "checkout"),
            EventField::string("checkout.id", checkout_id.clone()),
            EventField::string("operation.result", "started"),
        ],
    );

    validate_cart(&checkout_id).await;
    authenticate_customer(&checkout_id).await;
    reserve_inventory(&checkout_id).await;
    let pricing = price_order(&checkout_id).await;
    let direct_risk = call_risk_engine(&state).await.map_err(internal_error)?;
    authorize_payment(&checkout_id, pricing.total_cents).await;
    let service_b = call_processing(&state).await.map_err(internal_error)?;
    build_response().await;

    let duration_ms = started_at.elapsed().as_secs_f64() * 1000.0;
    record_event(
        "checkout.completed",
        [
            EventField::string("operation.name", "checkout"),
            EventField::string("checkout.id", checkout_id.clone()),
            EventField::string("operation.result", "success"),
            EventField::i64("duration.ms", duration_ms.round() as i64),
        ],
    );

    Ok(Json(json!({
        "status": "ok",
        "checkout_id": checkout_id,
        "pricing": {
            "currency": pricing.currency,
            "total_cents": pricing.total_cents,
            "discount_cents": pricing.discount_cents,
        },
        "service_c_direct": direct_risk,
        "service_b": service_b,
        "duration_ms": duration_ms,
    })))
}

async fn validate_cart(checkout_id: &str) {
    let checkout_id = checkout_id.to_owned();
    async move {
        sleep(Duration::from_millis(45)).await;
        record_event(
            "checkout.cart.validated",
            [
                EventField::string("operation.name", "checkout.validate_cart"),
                EventField::string("checkout.id", checkout_id),
                EventField::i64("cart.item_count", 3),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "checkout.validate_cart",
        component = "checkout",
        operation.name = "checkout.validate_cart",
        step = "validate_cart",
    ))
    .await;
}

async fn authenticate_customer(checkout_id: &str) {
    let checkout_id = checkout_id.to_owned();
    async move {
        sleep(Duration::from_millis(30)).await;
        record_event(
            "checkout.customer.authenticated",
            [
                EventField::string("operation.name", "checkout.authenticate_customer"),
                EventField::string("checkout.id", checkout_id),
                EventField::string("auth.method", "demo-session"),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "checkout.authenticate_customer",
        component = "checkout",
        operation.name = "checkout.authenticate_customer",
        step = "authenticate_customer",
    ))
    .await;
}

async fn reserve_inventory(checkout_id: &str) {
    let checkout_id = checkout_id.to_owned();
    async move {
        sleep(Duration::from_millis(65)).await;
        record_event(
            "checkout.inventory.reserved",
            [
                EventField::string("operation.name", "checkout.reserve_inventory"),
                EventField::string("checkout.id", checkout_id),
                EventField::string("operation.result", "reserved"),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "checkout.reserve_inventory",
        component = "checkout",
        operation.name = "checkout.reserve_inventory",
        step = "reserve_inventory",
    ))
    .await;
}

#[derive(Debug)]
struct PricingQuote {
    currency: &'static str,
    total_cents: i64,
    discount_cents: i64,
}

async fn price_order(checkout_id: &str) -> PricingQuote {
    let checkout_id = checkout_id.to_owned();
    async move {
        sleep(Duration::from_millis(55)).await;
        let quote = PricingQuote {
            currency: "USD",
            total_cents: 12_450,
            discount_cents: 1_250,
        };
        record_event(
            "checkout.priced",
            [
                EventField::string("operation.name", "checkout.price_order"),
                EventField::string("checkout.id", checkout_id),
                EventField::i64("pricing.total_cents", quote.total_cents),
                EventField::i64("pricing.discount_cents", quote.discount_cents),
            ],
        );
        quote
    }
    .instrument(tracing::info_span!(
        "checkout.price_order",
        component = "checkout",
        operation.name = "checkout.price_order",
        step = "price_order",
    ))
    .await
}

async fn authorize_payment(checkout_id: &str, total_cents: i64) {
    let checkout_id = checkout_id.to_owned();
    async move {
        sleep(Duration::from_millis(75)).await;
        record_event(
            "checkout.payment.authorized",
            [
                EventField::string("operation.name", "checkout.authorize_payment"),
                EventField::string("checkout.id", checkout_id),
                EventField::i64("payment.amount_cents", total_cents),
                EventField::string("payment.provider", "demo-bank"),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "checkout.authorize_payment",
        component = "payment",
        operation.name = "checkout.authorize_payment",
        step = "authorize_payment",
    ))
    .await;
}

async fn call_risk_engine(state: &AppState) -> Result<Value, my_infra_otel::MyOtelError> {
    async {
        let response = state
            .client
            .get(format!("{}/quote-risk", state.service_c_url))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            record_event(
                "checkout.direct_risk.failed",
                [
                    EventField::string("operation.name", "checkout.call_risk_engine"),
                    EventField::string("downstream.service", "service-c"),
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
            "checkout.direct_risk.completed",
            [
                EventField::string("operation.name", "checkout.call_risk_engine"),
                EventField::string("downstream.service", "service-c"),
                EventField::i64("downstream.status_code", i64::from(status.as_u16())),
            ],
        );

        Ok(body)
    }
    .instrument(tracing::info_span!(
        "checkout.call_risk_engine",
        component = "checkout",
        operation.name = "checkout.call_risk_engine",
        step = "call_risk_engine",
        downstream.service = "service-c",
    ))
    .await
}

async fn call_processing(state: &AppState) -> Result<Value, my_infra_otel::MyOtelError> {
    async {
        let response = state
            .client
            .get(format!("{}/process", state.service_b_url))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            record_event(
                "checkout.processing.failed",
                [
                    EventField::string("operation.name", "checkout.call_processing"),
                    EventField::string("downstream.service", "service-b"),
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
            "checkout.processing.completed",
            [
                EventField::string("operation.name", "checkout.call_processing"),
                EventField::string("downstream.service", "service-b"),
                EventField::i64("downstream.status_code", i64::from(status.as_u16())),
            ],
        );

        Ok(body)
    }
    .instrument(tracing::info_span!(
        "checkout.call_processing",
        component = "checkout",
        operation.name = "checkout.call_processing",
        step = "call_processing",
        downstream.service = "service-b",
    ))
    .await
}

async fn build_response() {
    async {
        sleep(Duration::from_millis(20)).await;
        record_event(
            "checkout.response.built",
            [EventField::string(
                "operation.name",
                "checkout.build_response",
            )],
        );
    }
    .instrument(tracing::info_span!(
        "checkout.build_response",
        component = "checkout",
        operation.name = "checkout.build_response",
        step = "build_response",
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
            "error": "downstream request failed",
            "detail": err.to_string(),
        })),
    )
}

fn checkout_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();

    format!("checkout-{millis}")
}

fn tracing_config(service_name: &str) -> Result<TracingConfig, my_infra_otel::MyOtelError> {
    let builder = TracingConfig::builder(service_name);
    match std::env::var("OTLP_ENDPOINT") {
        Ok(endpoint) => builder.otlp_endpoint(endpoint).build(),
        Err(_) => builder.build(),
    }
}
