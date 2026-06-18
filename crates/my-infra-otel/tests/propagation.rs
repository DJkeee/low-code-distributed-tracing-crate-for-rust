#![cfg(all(feature = "otlp", feature = "reqwest-client"))]

use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::Duration,
};

use my_infra_otel::TracedHttpClient;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{Value, trace::Status};
use opentelemetry_sdk::testing::trace::InMemorySpanExporter;
use tracing::instrument::WithSubscriber;
use tracing_subscriber::layer::SubscriberExt;

#[tokio::test]
async fn traced_http_client_injects_traceparent() {
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    let exporter = InMemorySpanExporter::default();
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();
    let tracer = provider.tracer("client-injection-test");
    let subscriber =
        tracing_subscriber::registry().with(tracing_opentelemetry::layer().with_tracer(tracer));
    let (url, received_headers) = spawn_one_request_server();
    let client = TracedHttpClient::new(reqwest::Client::new());

    let response = tracing::subscriber::with_default(subscriber, || {
        async { client.get(url).send().await }.with_current_subscriber()
    })
    .await
    .expect("request succeeds");

    assert!(response.status().is_success());

    let headers = received_headers
        .recv_timeout(Duration::from_secs(1))
        .expect("server receives request headers");
    let traceparent = headers
        .lines()
        .find_map(|line| {
            line.strip_prefix("traceparent:")
                .or_else(|| line.strip_prefix("Traceparent:"))
                .map(str::trim)
        })
        .expect("traceparent header is injected");

    assert!(traceparent.starts_with("00-"));
    assert_eq!(traceparent.len(), 55);

    for result in provider.force_flush() {
        result.expect("flush test provider");
    }
}

#[tokio::test]
async fn traced_http_client_records_error_status_without_full_url() {
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    let exporter = InMemorySpanExporter::default();
    let exporter_assert = exporter.clone();
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();
    let tracer = provider.tracer("client-error-test");
    let subscriber =
        tracing_subscriber::registry().with(tracing_opentelemetry::layer().with_tracer(tracer));
    let (url, received_headers) = spawn_one_request_server_with_status("404 Not Found");
    let client = TracedHttpClient::new(reqwest::Client::new());

    let response = tracing::subscriber::with_default(subscriber, || {
        async { client.get(format!("{url}?token=secret")).send().await }.with_current_subscriber()
    })
    .await
    .expect("request succeeds");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let _headers = received_headers
        .recv_timeout(Duration::from_secs(1))
        .expect("server receives request headers");

    for result in provider.force_flush() {
        result.expect("flush test provider");
    }

    let spans = exporter_assert
        .get_finished_spans()
        .expect("finished spans available");
    let span = spans
        .iter()
        .find(|span| span.name == "http.client.request")
        .expect("client span exported");

    assert!(matches!(span.status, Status::Error { .. }));
    assert_eq!(
        string_attribute(span, "error.type"),
        Some("http.client_error")
    );
    assert_eq!(string_attribute(span, "url.path"), Some("/process"));
    assert!(string_attribute(span, "url.full").is_none());
}

fn spawn_one_request_server() -> (String, mpsc::Receiver<String>) {
    spawn_one_request_server_with_status("200 OK")
}

fn spawn_one_request_server_with_status(status: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let address = listener.local_addr().expect("read test server address");
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept one request");
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("set read timeout");

        let mut buffer = [0_u8; 4096];
        let bytes_read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..bytes_read]).into_owned();
        let headers = request
            .split_once("\r\n\r\n")
            .map(|(head, _)| head.to_owned())
            .unwrap_or(request);
        sender.send(headers).expect("send captured headers");

        stream
            .write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 2\r\n\r\nok").as_bytes())
            .expect("write response");
    });

    (format!("http://{address}/process"), receiver)
}

fn string_attribute<'a>(
    span: &'a opentelemetry_sdk::export::trace::SpanData,
    key: &str,
) -> Option<&'a str> {
    span.attributes
        .iter()
        .find(|attribute| attribute.key.as_str() == key)
        .and_then(|attribute| match &attribute.value {
            Value::String(value) => Some(value.as_str()),
            _ => None,
        })
}
