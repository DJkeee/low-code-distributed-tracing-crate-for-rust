use std::time::{Duration, Instant};

use axum::{Json, Router, http::StatusCode, routing::get};
use my_infra_otel::{EventField, init_global_tracing, record_event};
use serde_json::{Value, json};
use tokio::time::sleep;
use tracing::Instrument;

mod tracing_setup;

const COMPUTE_ITERATIONS: u64 = 180_000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = tracing_setup::tracing_config("risk-engine")?;
    let _guard = init_global_tracing(config)?;
    let layer = tracing_setup::tracing_layer()?;

    let app = Router::new()
        .route("/quote-risk", get(quote_risk))
        .route("/health", get(health))
        .layer(layer);

    let bind = env_or_default("RISK_ENGINE_BIND", "SERVICE_C_BIND", "127.0.0.1:3003");
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "risk-engine listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn quote_risk() -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let started_at = Instant::now();

    record_event(
        "risk.started",
        [EventField::string("operation.name", "quote-risk")],
    );

    load_risk_model().await;
    prepare_risk_features().await;
    let risk_score = compute_raw_risk_score().await.map_err(internal_error)?;
    let calibrated_score = calibrate_risk_score(risk_score).await;
    explain_risk_decision(calibrated_score).await;
    persist_risk_decision(calibrated_score).await;

    let decision = if calibrated_score >= 70 {
        "manual_review"
    } else {
        "approved"
    };
    let duration_ms = started_at.elapsed().as_secs_f64() * 1000.0;

    record_event(
        "risk.completed",
        [
            EventField::string("operation.name", "quote-risk"),
            EventField::string("operation.result", decision),
            EventField::i64("risk.raw_score", i64::from(risk_score)),
            EventField::i64("risk.score", i64::from(calibrated_score)),
            EventField::i64("compute.iterations", COMPUTE_ITERATIONS as i64),
        ],
    );

    Ok(Json(json!({
        "status": "risk_evaluated",
        "decision": decision,
        "raw_risk_score": risk_score,
        "risk_score": calibrated_score,
        "compute_iterations": COMPUTE_ITERATIONS,
        "duration_ms": duration_ms,
    })))
}

async fn load_risk_model() {
    async {
        sleep(Duration::from_millis(35)).await;
        record_event(
            "risk.model.loaded",
            [
                EventField::string("operation.name", "quote-risk.load_risk_model"),
                EventField::string("model.name", "demo-risk-v1"),
                EventField::string("model.version", "2026-06-12"),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "quote-risk.load_risk_model",
        component = "risk-engine",
        operation.name = "quote-risk.load_risk_model",
        step = "load_risk_model",
    ))
    .await;
}

async fn prepare_risk_features() {
    async {
        sleep(Duration::from_millis(40)).await;
        record_event(
            "risk.features.prepared",
            [
                EventField::string("operation.name", "quote-risk.prepare_risk_features"),
                EventField::i64("feature.count", 12),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "quote-risk.prepare_risk_features",
        component = "risk-engine",
        operation.name = "quote-risk.prepare_risk_features",
        step = "prepare_risk_features",
    ))
    .await;
}

async fn compute_raw_risk_score() -> Result<u8, tokio::task::JoinError> {
    async {
        let score = tokio::task::spawn_blocking(cpu_risk_score).await?;
        record_event(
            "risk.score.computed",
            [
                EventField::string("operation.name", "quote-risk.compute_raw_risk_score"),
                EventField::i64("risk.score", i64::from(score)),
                EventField::i64("compute.iterations", COMPUTE_ITERATIONS as i64),
            ],
        );
        Ok(score)
    }
    .instrument(tracing::info_span!(
        "quote-risk.compute_raw_risk_score",
        component = "risk-engine",
        operation.name = "quote-risk.compute_raw_risk_score",
        step = "compute_raw_risk_score",
    ))
    .await
}

async fn calibrate_risk_score(score: u8) -> u8 {
    async move {
        sleep(Duration::from_millis(30)).await;
        let calibrated = score.saturating_add(7).min(99);
        record_event(
            "risk.score.calibrated",
            [
                EventField::string("operation.name", "quote-risk.calibrate_risk_score"),
                EventField::i64("risk.raw_score", i64::from(score)),
                EventField::i64("risk.score", i64::from(calibrated)),
            ],
        );
        calibrated
    }
    .instrument(tracing::info_span!(
        "quote-risk.calibrate_risk_score",
        component = "risk-engine",
        operation.name = "quote-risk.calibrate_risk_score",
        step = "calibrate_risk_score",
    ))
    .await
}

async fn explain_risk_decision(score: u8) {
    async move {
        sleep(Duration::from_millis(20)).await;
        let factor = if score >= 70 {
            "velocity"
        } else {
            "low_amount"
        };
        record_event(
            "risk.explanation.generated",
            [
                EventField::string("operation.name", "quote-risk.explain_risk_decision"),
                EventField::string("risk.top_factor", factor),
                EventField::i64("risk.score", i64::from(score)),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "quote-risk.explain_risk_decision",
        component = "risk-engine",
        operation.name = "quote-risk.explain_risk_decision",
        step = "explain_risk_decision",
    ))
    .await;
}

async fn persist_risk_decision(score: u8) {
    async move {
        sleep(Duration::from_millis(25)).await;
        record_event(
            "risk.decision.persisted",
            [
                EventField::string("operation.name", "quote-risk.persist_risk_decision"),
                EventField::i64("risk.score", i64::from(score)),
            ],
        );
    }
    .instrument(tracing::info_span!(
        "quote-risk.persist_risk_decision",
        component = "risk-engine",
        operation.name = "quote-risk.persist_risk_decision",
        step = "persist_risk_decision",
    ))
    .await;
}

fn cpu_risk_score() -> u8 {
    let mut acc = 0_u64;
    for i in 0..COMPUTE_ITERATIONS {
        acc = acc.wrapping_add((i ^ (acc.rotate_left(7))) % 97);
    }

    ((acc % 100) as u8).max(1)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

fn internal_error(err: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    record_event(
        "risk.failed",
        [
            EventField::string("operation.name", "quote-risk"),
            EventField::string("error.kind", "compute_join_error"),
        ],
    );

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "error": "risk computation failed",
            "detail": err.to_string(),
        })),
    )
}

fn env_or_default(primary: &str, legacy: &str, default: &str) -> String {
    std::env::var(primary)
        .or_else(|_| std::env::var(legacy))
        .unwrap_or_else(|_| default.to_owned())
}
