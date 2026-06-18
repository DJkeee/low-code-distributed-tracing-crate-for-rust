use my_infra_otel::{
    HeaderCapturePolicy, HeaderValueMode, MyOtelError, MyOtelTracingLayer, TracingConfig,
};

pub fn tracing_config(service_name: &str) -> Result<TracingConfig, MyOtelError> {
    let builder = TracingConfig::builder(service_name);
    match std::env::var("OTLP_ENDPOINT") {
        Ok(endpoint) => builder.otlp_endpoint(endpoint).build(),
        Err(_) => builder.build(),
    }
}

pub fn tracing_layer() -> Result<MyOtelTracingLayer, MyOtelError> {
    let headers = HeaderCapturePolicy::builder()
        .standard_request_ids()
        .gateway_headers()
        .header("x-tenant-id", "tenant.id")
        .header_with("x-user-id", "user.id", HeaderValueMode::Redacted)
        .max_value_len(128)
        .build()?;

    MyOtelTracingLayer::builder().headers(headers).build()
}
