use axum::{Json, Router, routing::get};
use my_infra_otel::{
    EventField, MyOtelTracingLayer, TracingConfig, init_global_tracing, record_event,
};
use serde_json::{Value, json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = TracingConfig::builder("service-a").build()?;
    let _guard = init_global_tracing(config)?;

    let layer = MyOtelTracingLayer::builder()
        .header_attr("x-user-id", "user.id")
        .header_attr("x-request-id", "request.id")
        .header_attr("x-tenant-id", "tenant.id")
        .build()?;

    let app = Router::new()
        .route("/checkout", get(checkout))
        .route("/health", get(health))
        .layer(layer);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn checkout() -> Json<Value> {
    record_event(
        "checkout.started",
        [EventField::string("operation.result", "success")],
    );
    Json(json!({ "status": "ok" }))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
