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
