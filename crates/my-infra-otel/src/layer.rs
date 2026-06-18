use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

#[cfg(feature = "otlp")]
use crate::header_capture::HeaderValueCapture;
use crate::{HeaderAttr, HeaderCapturePolicy, error::Result};
use tracing::Instrument;
#[cfg(feature = "otlp")]
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug, Clone)]
pub struct MyOtelTracingLayer {
    header_attrs: Vec<HeaderAttr>,
    header_capture_policy: HeaderCapturePolicy,
    capture_route: bool,
    capture_user_agent: bool,
}

#[derive(Debug, Clone)]
pub struct MyOtelTracingLayerBuilder {
    header_attrs: Vec<(String, String)>,
    header_capture_policy: Option<HeaderCapturePolicy>,
    capture_route: bool,
    capture_user_agent: bool,
}

#[derive(Debug, Clone)]
pub struct MyOtelTracingService<S> {
    inner: S,
    header_capture_policy: HeaderCapturePolicy,
    capture_route: bool,
    capture_user_agent: bool,
}

impl MyOtelTracingLayer {
    pub fn new() -> Self {
        Self {
            header_attrs: Vec::new(),
            header_capture_policy: HeaderCapturePolicy::empty(),
            capture_route: true,
            capture_user_agent: false,
        }
    }

    pub fn builder() -> MyOtelTracingLayerBuilder {
        MyOtelTracingLayerBuilder {
            header_attrs: Vec::new(),
            header_capture_policy: None,
            capture_route: true,
            capture_user_agent: false,
        }
    }

    pub fn header_attrs(&self) -> &[HeaderAttr] {
        &self.header_attrs
    }

    pub fn header_capture_policy(&self) -> &HeaderCapturePolicy {
        &self.header_capture_policy
    }
}

impl Default for MyOtelTracingLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MyOtelTracingLayerBuilder {
    pub fn header_attr(mut self, header_name: impl AsRef<str>, attr_key: impl AsRef<str>) -> Self {
        self.header_attrs.push((
            header_name.as_ref().to_owned(),
            attr_key.as_ref().to_owned(),
        ));
        self
    }

    pub fn headers(mut self, policy: HeaderCapturePolicy) -> Self {
        self.header_capture_policy = Some(policy);
        self
    }

    pub fn capture_route(mut self, enabled: bool) -> Self {
        self.capture_route = enabled;
        self
    }

    pub fn capture_user_agent(mut self, enabled: bool) -> Self {
        self.capture_user_agent = enabled;
        self
    }

    pub fn build(self) -> Result<MyOtelTracingLayer> {
        let header_capture_policy =
            merge_header_capture_policy(self.header_capture_policy, self.header_attrs)?;
        let header_attrs = header_capture_policy
            .rules()
            .iter()
            .map(|rule| HeaderAttr::new(rule.header_name().as_str(), rule.attr_key().as_str()))
            .collect::<Result<Vec<_>>>()?;

        Ok(MyOtelTracingLayer {
            header_attrs,
            header_capture_policy,
            capture_route: self.capture_route,
            capture_user_agent: self.capture_user_agent,
        })
    }
}

fn merge_header_capture_policy(
    policy: Option<HeaderCapturePolicy>,
    header_attrs: Vec<(String, String)>,
) -> Result<HeaderCapturePolicy> {
    let mut builder = HeaderCapturePolicy::builder();

    if let Some(policy) = policy {
        builder = builder
            .max_value_len(policy.max_value_len())
            .max_captured_headers(policy.max_captured_headers())
            .non_utf8(policy.non_utf8());

        for rule in policy.rules() {
            builder = builder.header_with(
                rule.header_name().as_str(),
                rule.attr_key().as_str(),
                *rule.mode(),
            );
        }
    }

    header_attrs
        .into_iter()
        .fold(builder, |builder, (header_name, attr_key)| {
            builder.header(header_name, attr_key)
        })
        .build()
}

impl<S> tower::Layer<S> for MyOtelTracingLayer {
    type Service = MyOtelTracingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MyOtelTracingService {
            inner,
            header_capture_policy: self.header_capture_policy.clone(),
            capture_route: self.capture_route,
            capture_user_agent: self.capture_user_agent,
        }
    }
}

impl<S, ReqBody, ResBody> tower::Service<http::Request<ReqBody>> for MyOtelTracingService<S>
where
    S: tower::Service<http::Request<ReqBody>, Response = http::Response<ResBody>>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: http::Request<ReqBody>) -> Self::Future {
        #[cfg(not(feature = "otlp"))]
        let _ = (&self.header_capture_policy, self.capture_user_agent);

        let method = request.method().clone();
        let path = request.uri().path().to_owned();
        let route = matched_route(&request).unwrap_or_else(|| path.clone());
        let span = tracing::info_span!(
            "http.request",
            otel.kind = "server",
            http.request.method = %method,
            url.path = %path,
            http.route = tracing::field::Empty,
            http.response.status_code = tracing::field::Empty,
            duration.ms = tracing::field::Empty,
            error = tracing::field::Empty,
            error.type = tracing::field::Empty,
            otel.status_code = tracing::field::Empty,
            otel.status_message = tracing::field::Empty,
        );

        if self.capture_route {
            span.record("http.route", route.as_str());
            #[cfg(feature = "otlp")]
            span.set_attribute("http.route", route);
        }

        #[cfg(feature = "otlp")]
        span.set_parent(crate::propagation::extract_context(request.headers()));

        #[cfg(feature = "otlp")]
        {
            span.set_attribute("http.request.method", method.to_string());
            span.set_attribute("url.path", path.clone());

            for attribute in self
                .header_capture_policy
                .captured_attributes(request.headers())
            {
                match attribute.value {
                    HeaderValueCapture::String(value) => {
                        span.set_attribute(attribute.key, value);
                    }
                    HeaderValueCapture::Present => {
                        span.set_attribute(attribute.key, "present");
                    }
                }
            }

            if self.capture_user_agent
                && let Some(user_agent) = request
                    .headers()
                    .get(http::header::USER_AGENT)
                    .and_then(|value| value.to_str().ok())
            {
                span.set_attribute("user_agent.original", user_agent.to_owned());
            }
        }

        let started_at = Instant::now();
        let future = self.inner.call(request);

        Box::pin(async move {
            match future.instrument(span.clone()).await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let duration_ms = started_at.elapsed().as_secs_f64() * 1000.0;
                    span.record("http.response.status_code", i64::from(status));
                    span.record("duration.ms", duration_ms);
                    let error_type = response_status_error_type(response.status());
                    if let Some(error_type) = error_type {
                        span.record("error", true);
                        span.record("error.type", error_type);
                        span.record("otel.status_code", "ERROR");
                        span.record("otel.status_message", error_type);
                    }

                    #[cfg(feature = "otlp")]
                    {
                        span.set_attribute("http.response.status_code", i64::from(status));
                        span.set_attribute("duration.ms", duration_ms);
                        if let Some(error_type) = error_type {
                            span.set_attribute("error.type", error_type);
                        }
                    }

                    Ok(response)
                }
                Err(err) => {
                    span.record("error", true);
                    span.record("error.type", "inner_service_error");
                    span.record("otel.status_code", "ERROR");
                    span.record("otel.status_message", "inner_service_error");
                    #[cfg(feature = "otlp")]
                    span.set_attribute("error.type", "inner_service_error");
                    Err(err)
                }
            }
        })
    }
}

#[cfg(feature = "axum")]
fn matched_route<ReqBody>(request: &http::Request<ReqBody>) -> Option<String> {
    request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|matched_path| matched_path.as_str().to_owned())
}

#[cfg(not(feature = "axum"))]
fn matched_route<ReqBody>(_request: &http::Request<ReqBody>) -> Option<String> {
    None
}

fn response_status_error_type(status: http::StatusCode) -> Option<&'static str> {
    if status.is_server_error() {
        Some("http.server_error")
    } else if status.is_client_error() {
        Some("http.client_error")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "otlp")]
    use std::{
        convert::Infallible,
        future::{Ready, ready},
        task::{Context, Poll},
    };

    #[cfg(feature = "otlp")]
    use opentelemetry::{
        Value,
        trace::{Status, TracerProvider as _},
    };
    #[cfg(feature = "otlp")]
    use opentelemetry_sdk::testing::trace::InMemorySpanExporter;
    #[cfg(feature = "otlp")]
    use tower::{Layer as _, Service as _};
    #[cfg(feature = "otlp")]
    use tracing::instrument::WithSubscriber;
    #[cfg(feature = "otlp")]
    use tracing_subscriber::layer::SubscriberExt;

    #[test]
    fn builds_layer_with_header_attrs() {
        let layer = MyOtelTracingLayer::builder()
            .header_attr("x-user-id", "user.id")
            .build()
            .expect("valid layer config");

        assert_eq!(layer.header_attrs().len(), 1);
        assert_eq!(layer.header_capture_policy().rules().len(), 1);
    }

    #[test]
    fn builds_layer_with_header_capture_policy() {
        let policy = HeaderCapturePolicy::builder()
            .header("x-request-id", "request.id")
            .build()
            .expect("valid policy");
        let layer = MyOtelTracingLayer::builder()
            .headers(policy)
            .build()
            .expect("valid layer config");

        assert_eq!(layer.header_attrs().len(), 1);
        assert_eq!(layer.header_capture_policy().rules().len(), 1);
    }

    #[test]
    fn rejects_duplicate_header_capture_rules_after_merge() {
        let policy = HeaderCapturePolicy::builder()
            .header("x-request-id", "request.id")
            .build()
            .expect("valid policy");

        assert!(
            MyOtelTracingLayer::builder()
                .headers(policy)
                .header_attr("X-Request-Id", "request.id.alt")
                .build()
                .is_err()
        );
    }

    #[test]
    fn rejects_invalid_header_attr() {
        assert!(
            MyOtelTracingLayer::builder()
                .header_attr("bad header", "user.id")
                .build()
                .is_err()
        );
    }

    #[cfg(feature = "otlp")]
    #[tokio::test]
    async fn records_error_status_for_http_error_response() {
        let exporter = InMemorySpanExporter::default();
        let exporter_assert = exporter.clone();
        let provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_simple_exporter(exporter)
            .build();
        let tracer = provider.tracer("server-status-test");
        let subscriber =
            tracing_subscriber::registry().with(tracing_opentelemetry::layer().with_tracer(tracer));
        let mut service = MyOtelTracingLayer::new().layer(StatusService {
            status: http::StatusCode::INTERNAL_SERVER_ERROR,
        });

        let response = tracing::subscriber::with_default(subscriber, || {
            async { service.call(http::Request::new(())).await }.with_current_subscriber()
        })
        .await
        .expect("service response");

        assert_eq!(response.status(), http::StatusCode::INTERNAL_SERVER_ERROR);

        for result in provider.force_flush() {
            result.expect("flush test provider");
        }

        let spans = exporter_assert
            .get_finished_spans()
            .expect("finished spans available");
        let span = spans
            .iter()
            .find(|span| span.name == "http.request")
            .expect("server span exported");

        assert!(matches!(span.status, Status::Error { .. }));
        assert_eq!(
            string_attribute(span, "error.type"),
            Some("http.server_error")
        );
    }

    #[cfg(feature = "otlp")]
    struct StatusService {
        status: http::StatusCode,
    }

    #[cfg(feature = "otlp")]
    impl tower::Service<http::Request<()>> for StatusService {
        type Response = http::Response<()>;
        type Error = Infallible;
        type Future = Ready<std::result::Result<Self::Response, Self::Error>>;

        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: http::Request<()>) -> Self::Future {
            let mut response = http::Response::new(());
            *response.status_mut() = self.status;
            ready(Ok(response))
        }
    }

    #[cfg(feature = "otlp")]
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
}
