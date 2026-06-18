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
        add_trace_context(_ctx, &mut output);

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
        if field.name() == "event.fields" {
            merge_event_fields(self.fields, value);
            return;
        }

        self.fields
            .insert(field.name().to_owned(), Value::String(value.to_owned()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "event.fields" {
            let value = format!("{value:?}");
            if let Ok(decoded) = serde_json::from_str::<String>(&value) {
                merge_event_fields(self.fields, &decoded);
            } else {
                merge_event_fields(self.fields, &value);
            }
            return;
        }

        self.fields
            .insert(field.name().to_owned(), Value::String(format!("{value:?}")));
    }
}

#[cfg(feature = "json-logs")]
fn merge_event_fields(output: &mut Map<String, Value>, value: &str) {
    if let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(value) {
        output.extend(fields);
    }
}

#[cfg(all(feature = "json-logs", feature = "otlp"))]
fn add_trace_context<S, N>(ctx: &FmtContext<'_, S, N>, output: &mut Map<String, Value>)
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    if let Some(span) = ctx.lookup_current() {
        let extensions = span.extensions();

        if let Some(data) = extensions.get::<tracing_opentelemetry::OtelData>() {
            let parent_span = data.parent_cx.span();
            let parent_context = parent_span.span_context();
            let trace_id = data
                .builder
                .trace_id
                .or_else(|| parent_context.is_valid().then(|| parent_context.trace_id()));

            if let (Some(trace_id), Some(span_id)) = (trace_id, data.builder.span_id) {
                output.insert("trace_id".to_owned(), Value::String(trace_id.to_string()));
                output.insert("span_id".to_owned(), Value::String(span_id.to_string()));
                return;
            }
        }
    }

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

#[cfg(all(test, feature = "json-logs", feature = "otlp"))]
mod tests {
    use super::*;
    use std::{
        io::{self, Write},
        sync::{Arc, Mutex},
    };

    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry::{
        Context,
        trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState},
    };
    use opentelemetry_sdk::testing::trace::InMemorySpanExporter;
    use tracing_opentelemetry::OpenTelemetrySpanExt;
    use tracing_subscriber::{fmt::writer::MakeWriter, layer::SubscriberExt};

    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);

    struct BufferWriter(Arc<Mutex<Vec<u8>>>);

    impl Buffer {
        fn lines(&self) -> Vec<Value> {
            let bytes = self.0.lock().expect("buffer lock").clone();
            let text = String::from_utf8(bytes).expect("valid utf8 log output");

            text.lines()
                .map(|line| serde_json::from_str(line).expect("valid json log line"))
                .collect()
        }
    }

    impl Write for BufferWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().expect("buffer lock").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Buffer {
        type Writer = BufferWriter;

        fn make_writer(&'a self) -> Self::Writer {
            BufferWriter(self.0.clone())
        }
    }

    #[test]
    fn json_formatter_adds_trace_context_and_flattens_business_fields() {
        let exporter = InMemorySpanExporter::default();
        let provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_simple_exporter(exporter)
            .build();
        let tracer = provider.tracer("log-correlation-test");
        let config = crate::TracingConfig::builder("log-correlation-test")
            .build()
            .expect("valid config");
        let output = Buffer::default();
        let subscriber = tracing_subscriber::registry()
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .with(
                tracing_subscriber::fmt::layer()
                    .event_format(JsonTraceFormatter::new(&config))
                    .with_writer(output.clone()),
            );

        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("request");
            span.set_parent(remote_parent_context());
            let _guard = span.enter();

            crate::record_event(
                "checkout.started",
                [
                    crate::EventField::string("operation.name", "checkout"),
                    crate::EventField::bool("operation.ok", true),
                ],
            );
        });

        let lines = output.lines();
        let event = lines.first().expect("one log event");

        assert_eq!(event["service.name"], "log-correlation-test");
        assert_eq!(event["event.name"], "checkout.started");
        assert_eq!(event["operation.name"], "checkout");
        assert_eq!(event["operation.ok"], true);
        assert!(
            event["trace_id"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(
            event["span_id"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );

        for result in provider.force_flush() {
            result.expect("flush test provider");
        }
    }

    #[test]
    fn json_formatter_omits_trace_context_outside_active_span() {
        let config = crate::TracingConfig::builder("outside-span-test")
            .build()
            .expect("valid config");
        let output = Buffer::default();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .event_format(JsonTraceFormatter::new(&config))
                .with_writer(output.clone()),
        );

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("service starting");
        });

        let lines = output.lines();
        let event = lines.first().expect("one log event");

        assert!(event.get("trace_id").is_none());
        assert!(event.get("span_id").is_none());
    }

    fn remote_parent_context() -> Context {
        let span_context = SpanContext::new(
            TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736").expect("valid trace id"),
            SpanId::from_hex("00f067aa0ba902b7").expect("valid span id"),
            TraceFlags::SAMPLED,
            true,
            TraceState::default(),
        );

        Context::new().with_remote_span_context(span_context)
    }
}
