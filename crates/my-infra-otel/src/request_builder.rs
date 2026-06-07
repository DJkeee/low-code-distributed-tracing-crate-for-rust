use std::time::Instant;

use serde::Serialize;
use tracing::Instrument;
#[cfg(feature = "otlp")]
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::error::Result;

#[derive(Debug)]
pub struct TracedRequestBuilder {
    inner: reqwest::RequestBuilder,
}

impl TracedRequestBuilder {
    pub(crate) fn new(inner: reqwest::RequestBuilder, _method: reqwest::Method) -> Self {
        Self { inner }
    }

    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        reqwest::header::HeaderName: TryFrom<K>,
        <reqwest::header::HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        reqwest::header::HeaderValue: TryFrom<V>,
        <reqwest::header::HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self {
            inner: self.inner.header(key, value),
        }
    }

    pub fn json<T: Serialize + ?Sized>(self, json: &T) -> Self {
        Self {
            inner: self.inner.json(json),
        }
    }

    pub fn body<T: Into<reqwest::Body>>(self, body: T) -> Self {
        Self {
            inner: self.inner.body(body),
        }
    }

    pub async fn send(self) -> Result<reqwest::Response> {
        let (client, request) = self.inner.build_split();
        let mut request = request?;
        let method = request.method().clone();
        let url = request.url().clone();
        let url_text = url.to_string();
        let host = url.host_str().unwrap_or_default().to_owned();
        let span = tracing::info_span!(
            "http.client.request",
            otel.kind = "client",
            http.request.method = %method,
            url.full = %url_text,
            server.address = %host,
            http.response.status_code = tracing::field::Empty,
            duration.ms = tracing::field::Empty,
            error.type = tracing::field::Empty,
        );

        #[cfg(feature = "otlp")]
        {
            span.set_attribute("http.request.method", method.to_string());
            span.set_attribute("url.full", url_text);
            span.set_attribute("server.address", host);
            let context = span.context();
            crate::propagation::inject_context(&context, request.headers_mut());
        }

        let started_at = Instant::now();
        let future = client.execute(request);

        match future.instrument(span.clone()).await {
            Ok(response) => {
                let status = response.status().as_u16();
                let duration_ms = started_at.elapsed().as_secs_f64() * 1000.0;
                span.record("http.response.status_code", i64::from(status));
                span.record("duration.ms", duration_ms);

                #[cfg(feature = "otlp")]
                {
                    span.set_attribute("http.response.status_code", i64::from(status));
                    span.set_attribute("duration.ms", duration_ms);
                }

                Ok(response)
            }
            Err(err) => {
                span.record("error.type", "reqwest");
                #[cfg(feature = "otlp")]
                span.set_attribute("error.type", "reqwest");
                Err(err.into())
            }
        }
    }
}
