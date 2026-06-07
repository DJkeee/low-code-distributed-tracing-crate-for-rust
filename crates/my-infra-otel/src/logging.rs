#[cfg(feature = "json-logs")]
use std::{fmt, time::SystemTime};

#[cfg(all(feature = "json-logs", feature = "otlp"))]
use opentelemetry::trace::TraceContextExt;
#[cfg(feature = "json-logs")]
use serde_json::{Map, Number, Value};
#[cfg(feature = "json-logs")]
use tracing::{Event, Subscriber, field::Visit};
#[cfg(all(feature = "json-logs", feature = "otlp"))]
use tracing_opentelemetry::OpenTelemetrySpanExt;
#[cfg(feature = "json-logs")]
use tracing_subscriber::{
    fmt::{FmtContext, FormatEvent, FormatFields, format::Writer},
    registry::LookupSpan,
};

#[cfg(feature = "json-logs")]
#[derive(Debug, Clone)]
pub(crate) struct JsonTraceFormatter {
    service_name: String,
    service_version: Option<String>,
    environment: String,
}

#[cfg(feature = "json-logs")]
impl JsonTraceFormatter {
    pub(crate) fn new(config: &crate::TracingConfig) -> Self {
        Self {
            service_name: config.service_name.clone(),
            service_version: config.service_version.clone(),
            environment: config.environment.clone(),
        }
    }
}

#[cfg(feature = "json-logs")]
impl<S, N> FormatEvent<S, N> for JsonTraceFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut fields = Map::new();
        event.record(&mut JsonVisitor {
            fields: &mut fields,
        });

        let mut output = Map::new();
        output.insert("timestamp".to_owned(), Value::String(timestamp()));
        output.insert(
            "level".to_owned(),
            Value::String(event.metadata().level().to_string()),
        );
        output.insert(
            "target".to_owned(),
            Value::String(event.metadata().target().to_owned()),
        );
        output.insert(
            "service.name".to_owned(),
            Value::String(self.service_name.clone()),
        );
        output.insert(
            "deployment.environment".to_owned(),
            Value::String(self.environment.clone()),
        );

        if let Some(version) = &self.service_version {
            output.insert("service.version".to_owned(), Value::String(version.clone()));
        }

        #[cfg(feature = "otlp")]
        add_trace_context(&mut output);

        output.extend(fields);

        let line = serde_json::to_string(&Value::Object(output)).map_err(|_| fmt::Error)?;
        writer.write_str(&line)?;
        writer.write_char('\n')
    }
}

#[cfg(feature = "json-logs")]
struct JsonVisitor<'a> {
    fields: &'a mut Map<String, Value>,
}

#[cfg(feature = "json-logs")]
impl Visit for JsonVisitor<'_> {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if let Some(value) = Number::from_f64(value) {
            self.fields
                .insert(field.name().to_owned(), Value::Number(value));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_owned(), Value::Number(Number::from(value)));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_owned(), Value::Number(Number::from(value)));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_owned(), Value::Bool(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_owned(), Value::String(value.to_owned()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.fields
            .insert(field.name().to_owned(), Value::String(format!("{value:?}")));
    }
}

#[cfg(all(feature = "json-logs", feature = "otlp"))]
fn add_trace_context(output: &mut Map<String, Value>) {
    let context = tracing::Span::current().context();
    let span = context.span();
    let span_context = span.span_context();

    if span_context.is_valid() {
        output.insert(
            "trace_id".to_owned(),
            Value::String(span_context.trace_id().to_string()),
        );
        output.insert(
            "span_id".to_owned(),
            Value::String(span_context.span_id().to_string()),
        );
    }
}

#[cfg(feature = "json-logs")]
fn timestamp() -> String {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => format!("{}.{}", duration.as_secs(), duration.subsec_millis()),
        Err(_) => "0.000".to_owned(),
    }
}
