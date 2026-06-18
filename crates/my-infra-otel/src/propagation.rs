#[cfg(feature = "otlp")]
use http::HeaderMap;

#[cfg(feature = "otlp")]
pub(crate) fn inject_context(context: &opentelemetry::Context, headers: &mut HeaderMap) {
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(context, &mut HeaderInjector(headers));
    });
}

#[cfg(feature = "otlp")]
pub(crate) fn extract_context(headers: &HeaderMap) -> opentelemetry::Context {
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(headers))
    })
}

#[cfg(feature = "otlp")]
struct HeaderExtractor<'a>(&'a HeaderMap);

#[cfg(feature = "otlp")]
struct HeaderInjector<'a>(&'a mut HeaderMap);

#[cfg(feature = "otlp")]
impl opentelemetry::propagation::Injector for HeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        let Ok(header_name) = http::HeaderName::from_bytes(key.as_bytes()) else {
            return;
        };
        let Ok(header_value) = http::HeaderValue::from_str(&value) else {
            return;
        };
        self.0.insert(header_name, header_value);
    }
}

#[cfg(feature = "otlp")]
impl opentelemetry::propagation::Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(http::HeaderName::as_str).collect()
    }
}

#[cfg(all(test, feature = "otlp"))]
mod tests {
    use super::*;
    use opentelemetry::{
        Context,
        trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState},
    };

    #[test]
    fn injects_and_extracts_w3c_traceparent() {
        opentelemetry::global::set_text_map_propagator(
            opentelemetry_sdk::propagation::TraceContextPropagator::new(),
        );

        let trace_id =
            TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736").expect("valid trace id");
        let span_id = SpanId::from_hex("00f067aa0ba902b7").expect("valid span id");
        let span_context = SpanContext::new(
            trace_id,
            span_id,
            TraceFlags::SAMPLED,
            true,
            TraceState::default(),
        );
        let context = Context::new().with_remote_span_context(span_context);
        let mut headers = HeaderMap::new();

        inject_context(&context, &mut headers);

        assert_eq!(
            headers
                .get("traceparent")
                .and_then(|value| value.to_str().ok()),
            Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
        );

        let extracted = extract_context(&headers);
        let extracted_span = extracted.span();
        let extracted_context = extracted_span.span_context();

        assert_eq!(extracted_context.trace_id(), trace_id);
        assert_eq!(extracted_context.span_id(), span_id);
        assert!(extracted_context.is_remote());
        assert!(extracted_context.is_sampled());
    }
}
