use axum::{Json, Router, routing::get};
use my_infra_otel::{
    EventField, MyOtelTracingLayer, TracingConfig, init_global_tracing, record_event,
};
use serde_json::{Value, json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = TracingConfig::builder("service-b").build()?;
    let _guard = init_global_tracing(config)?;

    let app = Router::new()
        .route("/process", get(process))
        .route("/health", get(health))
        .layer(MyOtelTracingLayer::new());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn process() -> Json<Value> {
    record_event(
        "process.started",
        [EventField::string("operation.result", "success")],
    );
    Json(json!({ "status": "processed" }))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
