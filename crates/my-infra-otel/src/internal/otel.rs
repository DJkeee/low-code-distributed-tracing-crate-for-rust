use crate::{MyOtelError, TracingConfig, error::Result};

pub(crate) fn init_provider(
    config: &TracingConfig,
) -> Result<opentelemetry_sdk::trace::TracerProvider> {
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::{Resource, trace::Sampler};

    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    let mut resource_attrs = vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("deployment.environment", config.environment.clone()),
    ];

    if let Some(version) = &config.service_version {
        resource_attrs.push(KeyValue::new("service.version", version.clone()));
    }

    resource_attrs.extend(
        config
            .resource_attrs
            .iter()
            .map(|(key, value)| KeyValue::new(key.clone(), value.clone())),
    );

    let pipeline = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint(config.otlp_endpoint.clone())
                .with_timeout(config.export_timeout),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(Sampler::AlwaysOn)
                .with_resource(Resource::new(resource_attrs)),
        );

    if tokio::runtime::Handle::try_current().is_ok() {
        pipeline
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .map_err(|err| MyOtelError::ExporterInit(err.to_string()))
    } else {
        pipeline
            .install_simple()
            .map_err(|err| MyOtelError::ExporterInit(err.to_string()))
    }
}
