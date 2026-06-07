use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "json-logs")]
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "json-logs")]
use crate::LogFormat;
#[cfg(feature = "json-logs")]
use crate::logging::JsonTraceFormatter;
use crate::{TracingConfig, TracingGuard, error::MyOtelError, error::Result};
#[cfg(feature = "otlp")]
use opentelemetry::trace::TracerProvider as _;

static TRACING_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init_global_tracing(config: TracingConfig) -> Result<TracingGuard> {
    if TRACING_INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(MyOtelError::AlreadyInitialized);
    }

    match init_global_tracing_inner(&config) {
        Ok(guard) => Ok(guard),
        Err(err) => {
            TRACING_INITIALIZED.store(false, Ordering::Release);
            Err(err)
        }
    }
}

#[cfg(all(feature = "otlp", feature = "json-logs"))]
fn init_global_tracing_inner(config: &TracingConfig) -> Result<TracingGuard> {
    config.validate()?;

    let tracer_provider = crate::internal::otel::init_provider(config)?;
    opentelemetry::global::set_tracer_provider(tracer_provider.clone());

    let tracer = tracer_provider.tracer(config.service_name.clone());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let filter = EnvFilter::try_new(&config.log_filter)
        .map_err(|err| MyOtelError::SubscriberInit(err.to_string()))?;

    let init_result = match config.log_format {
        LogFormat::Json => tracing_subscriber::registry()
            .with(filter)
            .with(otel_layer)
            .with(tracing_subscriber::fmt::layer().event_format(JsonTraceFormatter::new(config)))
            .try_init(),
        LogFormat::Pretty => tracing_subscriber::registry()
            .with(filter)
            .with(otel_layer)
            .with(tracing_subscriber::fmt::layer())
            .try_init(),
    };

    if init_result.is_err() {
        let _ = tracer_provider.shutdown();
        return Err(MyOtelError::AlreadyInitialized);
    }

    Ok(TracingGuard::new(config, tracer_provider))
}

#[cfg(all(feature = "otlp", not(feature = "json-logs")))]
fn init_global_tracing_inner(config: &TracingConfig) -> Result<TracingGuard> {
    config.validate()?;

    let tracer_provider = crate::internal::otel::init_provider(config)?;
    opentelemetry::global::set_tracer_provider(tracer_provider.clone());

    let tracer = tracer_provider.tracer(config.service_name.clone());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    if tracing_subscriber::registry()
        .with(otel_layer)
        .try_init()
        .is_err()
    {
        let _ = tracer_provider.shutdown();
        return Err(MyOtelError::AlreadyInitialized);
    }

    Ok(TracingGuard::new(config, tracer_provider))
}

#[cfg(all(not(feature = "otlp"), feature = "json-logs"))]
fn init_global_tracing_inner(config: &TracingConfig) -> Result<TracingGuard> {
    config.validate()?;

    let filter = EnvFilter::try_new(&config.log_filter)
        .map_err(|err| MyOtelError::SubscriberInit(err.to_string()))?;

    let init_result = match config.log_format {
        LogFormat::Json => tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().event_format(JsonTraceFormatter::new(config)))
            .try_init(),
        LogFormat::Pretty => tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init(),
    };

    init_result.map_err(|_| MyOtelError::AlreadyInitialized)?;

    Ok(TracingGuard::new(config))
}

#[cfg(all(not(feature = "otlp"), not(feature = "json-logs")))]
fn init_global_tracing_inner(config: &TracingConfig) -> Result<TracingGuard> {
    config.validate()?;
    Ok(TracingGuard::new(config))
}
