use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use askama::Template;
use axum::{
    Form, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use my_infra_otel::{EventField, TracedHttpClient, init_global_tracing, record_event};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::time::sleep;

mod tracing_setup;

const DEFAULT_BIND: &str = "127.0.0.1:3002";
const DEFAULT_CHECKOUT_API_URL: &str = "http://127.0.0.1:3000";
const DEFAULT_ORDER_PROCESSOR_URL: &str = "http://127.0.0.1:3001";
const DEFAULT_RISK_ENGINE_URL: &str = "http://127.0.0.1:3003";
const DEFAULT_JAEGER_URL: &str = "http://localhost:16686";
const TRACE_LOOKUP_ATTEMPTS: usize = 30;
const TRACE_LOOKUP_DELAY: Duration = Duration::from_millis(300);
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct AppState {
    traced_client: TracedHttpClient,
    http_client: reqwest::Client,
    checkout_api_url: String,
    order_processor_url: String,
    risk_engine_url: String,
    jaeger_url: String,
}

#[derive(Debug, Deserialize)]
struct CheckoutForm {
    user_id: String,
    tenant_id: String,
}

#[derive(Debug, Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    checkout_api_url: String,
    order_processor_url: String,
    risk_engine_url: String,
    jaeger_url: String,
    health: HealthView,
    result: Option<RunResult>,
}

#[derive(Debug, Clone)]
struct HealthView {
    probes: Vec<ProbeStatus>,
}

#[derive(Debug, Clone)]
struct ProbeStatus {
    label: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Clone)]
struct RunResult {
    request_id: String,
    user_id: String,
    tenant_id: String,
    response_status: String,
    response_body: String,
    trace: Option<TraceView>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct TraceView {
    trace_id: String,
    jaeger_url: String,
    span_count: usize,
    services: String,
    total_duration_ms: String,
    spans: Vec<SpanView>,
}

#[derive(Debug, Clone)]
struct SpanView {
    service: String,
    operation: String,
    span_id: String,
    parent_span_id: Option<String>,
    start_offset_ms: String,
    duration_ms: String,
    tags: Vec<TagView>,
}

#[derive(Debug, Clone)]
struct TagView {
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct JaegerSearchResponse {
    data: Vec<JaegerTrace>,
}

#[derive(Debug, Deserialize)]
struct JaegerTrace {
    #[serde(rename = "traceID")]
    trace_id: String,
    spans: Vec<JaegerSpan>,
    processes: HashMap<String, JaegerProcess>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JaegerSpan {
    #[serde(rename = "spanID")]
    span_id: String,
    operation_name: String,
    references: Vec<JaegerReference>,
    start_time: u64,
    duration: u64,
    tags: Vec<JaegerTag>,
    #[serde(rename = "processID")]
    process_id: String,
}

#[derive(Debug, Deserialize)]
struct JaegerReference {
    #[serde(rename = "spanID")]
    span_id: String,
    #[serde(rename = "refType")]
    ref_type: String,
}

#[derive(Debug, Deserialize)]
struct JaegerProcess {
    #[serde(rename = "serviceName")]
    service_name: String,
}

#[derive(Debug, Deserialize)]
struct JaegerTag {
    key: String,
    value: Value,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = tracing_setup::tracing_config("demo-ui")?;
    let _guard = init_global_tracing(config)?;
    let layer = tracing_setup::tracing_layer()?;

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    let checkout_api_url = env_or_legacy_default(
        "CHECKOUT_API_URL",
        "SERVICE_A_URL",
        DEFAULT_CHECKOUT_API_URL,
    );
    let order_processor_url = env_or_legacy_default(
        "ORDER_PROCESSOR_URL",
        "SERVICE_B_URL",
        DEFAULT_ORDER_PROCESSOR_URL,
    );
    let risk_engine_url =
        env_or_legacy_default("RISK_ENGINE_URL", "SERVICE_C_URL", DEFAULT_RISK_ENGINE_URL);
    let jaeger_url = env_or_default("JAEGER_URL", DEFAULT_JAEGER_URL);
    let bind = env_or_default("DEMO_UI_BIND", DEFAULT_BIND);

    let state = AppState {
        traced_client: TracedHttpClient::new(http_client.clone()),
        http_client,
        checkout_api_url,
        order_processor_url,
        risk_engine_url,
        jaeger_url,
    };

    let app = Router::new()
        .route("/", get(index).post(run_checkout))
        .route("/health", get(health))
        .with_state(state)
        .layer(layer);

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    let address = listener.local_addr()?;
    tracing::info!(%address, "demo UI listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index(State(state): State<AppState>) -> Result<Html<String>, UiError> {
    render_dashboard(&state, None).await
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, axum::Json(json!({ "status": "ok" })))
}

async fn run_checkout(
    State(state): State<AppState>,
    Form(form): Form<CheckoutForm>,
) -> Result<Html<String>, UiError> {
    let request_id = request_id();
    let user_id = empty_to_default(form.user_id, "42");
    let tenant_id = empty_to_default(form.tenant_id, "demo");

    record_event(
        "demo.checkout.requested",
        [
            EventField::string("request.id", request_id.clone()),
            EventField::string("operation.name", "demo.checkout"),
            EventField::string("operation.result", "started"),
        ],
    );

    let checkout_url = format!("{}/checkout", state.checkout_api_url);
    let response = state
        .traced_client
        .get(&checkout_url)
        .header("x-user-id", &user_id)
        .header("x-request-id", &request_id)
        .header("x-tenant-id", &tenant_id)
        .send()
        .await;

    let mut result = match response {
        Ok(response) => {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|err| format!("failed to read response body: {err}"));

            RunResult {
                request_id: request_id.clone(),
                user_id,
                tenant_id,
                response_status: status.to_string(),
                response_body: pretty_json(&body),
                trace: None,
                error: (!status.is_success())
                    .then(|| "checkout-api returned non-success status".to_owned()),
            }
        }
        Err(err) => RunResult {
            request_id: request_id.clone(),
            user_id,
            tenant_id,
            response_status: "request failed".to_owned(),
            response_body: "".to_owned(),
            trace: None,
            error: Some(err.to_string()),
        },
    };

    if result.error.is_none() {
        match wait_for_trace(&state, &request_id).await {
            Ok(Some(trace)) => {
                record_event(
                    "demo.checkout.trace_found",
                    [
                        EventField::string("request.id", request_id),
                        EventField::string("trace.id", trace.trace_id.clone()),
                        EventField::i64("span.count", trace.span_count as i64),
                    ],
                );
                result.trace = Some(trace);
            }
            Ok(None) => {
                result.error = Some(
                    "Trace was not found in Jaeger yet. Refresh Jaeger or run the request again."
                        .to_owned(),
                );
            }
            Err(err) => {
                result.error = Some(format!("Failed to query Jaeger: {err}"));
            }
        }
    }

    render_dashboard(&state, Some(result)).await
}

async fn render_dashboard(
    state: &AppState,
    result: Option<RunResult>,
) -> Result<Html<String>, UiError> {
    let health = probe_health(state).await;
    let template = DashboardTemplate {
        checkout_api_url: state.checkout_api_url.clone(),
        order_processor_url: state.order_processor_url.clone(),
        risk_engine_url: state.risk_engine_url.clone(),
        jaeger_url: state.jaeger_url.clone(),
        health,
        result,
    };

    template
        .render()
        .map(Html)
        .map_err(|err| UiError::new(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

async fn probe_health(state: &AppState) -> HealthView {
    HealthView {
        probes: vec![
            probe_json(
                &state.http_client,
                "checkout-api",
                &format!("{}/health", state.checkout_api_url),
            )
            .await,
            probe_json(
                &state.http_client,
                "order-processor",
                &format!("{}/health", state.order_processor_url),
            )
            .await,
            probe_json(
                &state.http_client,
                "risk-engine",
                &format!("{}/health", state.risk_engine_url),
            )
            .await,
            probe_json(
                &state.http_client,
                "jaeger",
                &format!("{}/api/services", state.jaeger_url),
            )
            .await,
        ],
    }
}

async fn probe_json(client: &reqwest::Client, label: &str, url: &str) -> ProbeStatus {
    match client.get(url).send().await {
        Ok(response) if response.status().is_success() => ProbeStatus {
            label: label.to_owned(),
            ok: true,
            detail: response.status().to_string(),
        },
        Ok(response) => ProbeStatus {
            label: label.to_owned(),
            ok: false,
            detail: response.status().to_string(),
        },
        Err(err) => ProbeStatus {
            label: label.to_owned(),
            ok: false,
            detail: err.to_string(),
        },
    }
}

async fn wait_for_trace(
    state: &AppState,
    request_id: &str,
) -> Result<Option<TraceView>, reqwest::Error> {
    for _ in 0..TRACE_LOOKUP_ATTEMPTS {
        if let Some(trace) = find_trace(state, request_id).await? {
            return Ok(Some(trace));
        }

        sleep(TRACE_LOOKUP_DELAY).await;
    }

    Ok(None)
}

async fn find_trace(
    state: &AppState,
    request_id: &str,
) -> Result<Option<TraceView>, reqwest::Error> {
    let response = state
        .http_client
        .get(format!("{}/api/traces", state.jaeger_url))
        .query(&[
            ("service", "checkout-api"),
            ("lookback", "1h"),
            ("limit", "100"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<JaegerSearchResponse>()
        .await?;

    Ok(response
        .data
        .into_iter()
        .find(|trace| trace_has_request_id(trace, request_id))
        .map(|trace| trace.into_view(&state.jaeger_url)))
}

fn trace_has_request_id(trace: &JaegerTrace, request_id: &str) -> bool {
    trace.spans.iter().any(|span| {
        span.tags
            .iter()
            .any(|tag| tag.key == "request.id" && tag_value_text(&tag.value) == request_id)
    })
}

impl JaegerTrace {
    fn into_view(self, jaeger_url: &str) -> TraceView {
        let trace_id = self.trace_id;
        let span_count = self.spans.len();
        let min_start = self
            .spans
            .iter()
            .map(|span| span.start_time)
            .min()
            .unwrap_or(0);
        let max_end = self
            .spans
            .iter()
            .map(|span| span.start_time.saturating_add(span.duration))
            .max()
            .unwrap_or(min_start);
        let mut spans = self
            .spans
            .into_iter()
            .map(|span| span.into_view(min_start, &self.processes))
            .collect::<Vec<_>>();
        spans.sort_by(|left, right| {
            left.start_offset_ms
                .cmp(&right.start_offset_ms)
                .then_with(|| left.service.cmp(&right.service))
        });

        let mut services = spans
            .iter()
            .map(|span| span.service.clone())
            .collect::<Vec<_>>();
        services.sort();
        services.dedup();

        TraceView {
            trace_id: trace_id.clone(),
            jaeger_url: format!("{}/trace/{trace_id}", jaeger_url.trim_end_matches('/')),
            span_count,
            services: services.join(" -> "),
            total_duration_ms: micros_to_ms(max_end.saturating_sub(min_start)),
            spans,
        }
    }
}

impl JaegerSpan {
    fn into_view(self, min_start: u64, processes: &HashMap<String, JaegerProcess>) -> SpanView {
        let service = processes
            .get(&self.process_id)
            .map(|process| process.service_name.clone())
            .unwrap_or_else(|| self.process_id.clone());
        let parent_span_id = self
            .references
            .iter()
            .find(|reference| reference.ref_type == "CHILD_OF")
            .map(|reference| reference.span_id.clone());
        let mut tags = self
            .tags
            .into_iter()
            .map(|tag| TagView {
                key: tag.key,
                value: tag_value_text(&tag.value),
            })
            .collect::<Vec<_>>();
        tags.sort_by(|left, right| left.key.cmp(&right.key));

        SpanView {
            service,
            operation: self.operation_name,
            span_id: self.span_id,
            parent_span_id,
            start_offset_ms: micros_to_ms(self.start_time.saturating_sub(min_start)),
            duration_ms: micros_to_ms(self.duration),
            tags,
        }
    }
}

#[derive(Debug)]
struct UiError {
    status: StatusCode,
    message: String,
}

impl UiError {
    fn new(status: StatusCode, message: String) -> Self {
        Self { status, message }
    }
}

impl IntoResponse for UiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn env_or_legacy_default(primary: &str, legacy: &str, default: &str) -> String {
    std::env::var(primary)
        .or_else(|_| std::env::var(legacy))
        .unwrap_or_else(|_| default.to_owned())
}

fn request_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let sequence = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("demo-ui-{millis}-{sequence}")
}

fn empty_to_default(value: String, default: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn pretty_json(value: &str) -> String {
    serde_json::from_str::<Value>(value)
        .and_then(|value| serde_json::to_string_pretty(&value))
        .unwrap_or_else(|_| value.to_owned())
}

fn tag_value_text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => "null".to_owned(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn micros_to_ms(value: u64) -> String {
    format!("{:.2}", value as f64 / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_jaeger_trace_response_with_uppercase_id_fields() {
        let response = serde_json::from_str::<JaegerSearchResponse>(
            r#"{
              "data": [
                {
                  "traceID": "e50dbc10763b9ea631f324b5cb6587e5",
                  "spans": [
                    {
                      "traceID": "e50dbc10763b9ea631f324b5cb6587e5",
                      "spanID": "06006aeec8524aa5",
                      "operationName": "http.request",
                      "references": [
                        {
                          "refType": "CHILD_OF",
                          "traceID": "e50dbc10763b9ea631f324b5cb6587e5",
                          "spanID": "11116aeec8524aa5"
                        }
                      ],
                      "startTime": 1781814422881212,
                      "duration": 126,
                      "tags": [
                        {
                          "key": "request.id",
                          "type": "string",
                          "value": "demo-ui-1-0"
                        }
                      ],
                      "processID": "p1",
                      "warnings": null
                    }
                  ],
                  "processes": {
                    "p1": {
                      "serviceName": "checkout-api",
                      "tags": []
                    }
                  },
                  "warnings": null
                }
              ],
              "total": 1,
              "limit": 1,
              "offset": 0,
              "errors": null
            }"#,
        )
        .expect("valid Jaeger search response");

        let trace = response.data.into_iter().next().expect("one trace");
        assert_eq!(trace.trace_id, "e50dbc10763b9ea631f324b5cb6587e5");
        assert!(trace_has_request_id(&trace, "demo-ui-1-0"));

        let view = trace.into_view("http://localhost:16686");
        assert_eq!(view.spans[0].span_id, "06006aeec8524aa5");
        assert_eq!(
            view.spans[0].parent_span_id.as_deref(),
            Some("11116aeec8524aa5")
        );
        assert_eq!(view.spans[0].service, "checkout-api");
    }
}
