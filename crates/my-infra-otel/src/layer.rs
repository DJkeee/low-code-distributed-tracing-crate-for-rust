use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use crate::{HeaderAttr, error::Result};
use tracing::Instrument;
#[cfg(feature = "otlp")]
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug, Clone)]
pub struct MyOtelTracingLayer {
    header_attrs: Vec<HeaderAttr>,
    capture_route: bool,
    capture_user_agent: bool,
}

#[derive(Debug, Clone)]
pub struct MyOtelTracingLayerBuilder {
    header_attrs: Vec<(String, String)>,
    capture_route: bool,
    capture_user_agent: bool,
}

#[derive(Debug, Clone)]
pub struct MyOtelTracingService<S> {
    inner: S,
    header_attrs: Vec<HeaderAttr>,
    capture_route: bool,
    capture_user_agent: bool,
}

impl MyOtelTracingLayer {
    pub fn new() -> Self {
        Self {
            header_attrs: Vec::new(),
            capture_route: true,
            capture_user_agent: false,
        }
    }

    pub fn builder() -> MyOtelTracingLayerBuilder {
        MyOtelTracingLayerBuilder {
            header_attrs: Vec::new(),
            capture_route: true,
            capture_user_agent: false,
        }
    }

    pub fn header_attrs(&self) -> &[HeaderAttr] {
        &self.header_attrs
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

    pub fn capture_route(mut self, enabled: bool) -> Self {
        self.capture_route = enabled;
        self
    }

    pub fn capture_user_agent(mut self, enabled: bool) -> Self {
        self.capture_user_agent = enabled;
        self
    }

    pub fn build(self) -> Result<MyOtelTracingLayer> {
        let header_attrs = self
            .header_attrs
            .into_iter()
            .map(|(header, attr)| HeaderAttr::new(header, attr))
            .collect::<Result<Vec<_>>>()?;

        Ok(MyOtelTracingLayer {
            header_attrs,
            capture_route: self.capture_route,
            capture_user_agent: self.capture_user_agent,
        })
    }
}

impl<S> tower::Layer<S> for MyOtelTracingLayer {
    type Service = MyOtelTracingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MyOtelTracingService {
            inner,
            header_attrs: self.header_attrs.clone(),
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
        let _ = (&self.header_attrs, self.capture_user_agent);

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

            for header_attr in &self.header_attrs {
                if let Some(value) = request
                    .headers()
                    .get(header_attr.header_name())
                    .and_then(|value| value.to_str().ok())
                {
                    span.set_attribute(
                        header_attr.attr_key().as_str().to_owned(),
                        value.to_owned(),
                    );
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

                    #[cfg(feature = "otlp")]
                    {
                        span.set_attribute("http.response.status_code", i64::from(status));
                        span.set_attribute("duration.ms", duration_ms);
                    }

                    Ok(response)
                }
                Err(err) => {
                    span.record("error", true);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_layer_with_header_attrs() {
        let layer = MyOtelTracingLayer::builder()
            .header_attr("x-user-id", "user.id")
            .build()
            .expect("valid layer config");

        assert_eq!(layer.header_attrs().len(), 1);
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
}
