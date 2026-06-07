use std::{env, time::Duration};

use crate::{
    error::{ConfigError, MyOtelError, Result},
    labels::validate_attr_key,
};

#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub service_name: String,
    pub service_version: Option<String>,
    pub environment: String,
    pub otlp_endpoint: String,
    pub log_filter: String,
    pub resource_attrs: Vec<(String, String)>,
    pub sampling: SamplingMode,
    pub export_timeout: Duration,
    pub shutdown_timeout: Duration,
    pub log_format: LogFormat,
}

#[derive(Debug, Clone)]
pub struct TracingConfigBuilder {
    service_name: String,
    service_version: Option<String>,
    environment: String,
    otlp_endpoint: String,
    log_filter: String,
    resource_attrs: Vec<(String, String)>,
    sampling: SamplingMode,
    export_timeout: Duration,
    shutdown_timeout: Duration,
    log_format: LogFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingMode {
    AlwaysOn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}

impl TracingConfig {
    pub fn builder(service_name: impl Into<String>) -> TracingConfigBuilder {
        TracingConfigBuilder {
            service_name: service_name.into(),
            service_version: None,
            environment: "local".to_owned(),
            otlp_endpoint: "http://localhost:4318/v1/traces".to_owned(),
            log_filter: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_owned()),
            resource_attrs: Vec::new(),
            sampling: SamplingMode::AlwaysOn,
            export_timeout: Duration::from_secs(5),
            shutdown_timeout: Duration::from_secs(5),
            log_format: LogFormat::Json,
        }
    }
}

impl TracingConfigBuilder {
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.service_version = Some(version.into());
        self
    }

    pub fn environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = environment.into();
        self
    }

    pub fn otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = endpoint.into();
        self
    }

    pub fn log_filter(mut self, filter: impl Into<String>) -> Self {
        self.log_filter = filter.into();
        self
    }

    pub fn resource_attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.resource_attrs.push((key.into(), value.into()));
        self
    }

    pub fn sampling(mut self, sampling: SamplingMode) -> Self {
        self.sampling = sampling;
        self
    }

    pub fn export_timeout(mut self, timeout: Duration) -> Self {
        self.export_timeout = timeout;
        self
    }

    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    pub fn log_format(mut self, format: LogFormat) -> Self {
        self.log_format = format;
        self
    }

    pub fn build(self) -> Result<TracingConfig> {
        let config = TracingConfig {
            service_name: self.service_name,
            service_version: self.service_version,
            environment: self.environment,
            otlp_endpoint: self.otlp_endpoint,
            log_filter: self.log_filter,
            resource_attrs: self.resource_attrs,
            sampling: self.sampling,
            export_timeout: self.export_timeout,
            shutdown_timeout: self.shutdown_timeout,
            log_format: self.log_format,
        };
        config.validate()?;
        Ok(config)
    }
}

impl TracingConfig {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.service_name.trim().is_empty() {
            return Err(ConfigError::EmptyServiceName.into());
        }

        validate_otlp_endpoint(&self.otlp_endpoint)?;

        for (key, _) in &self.resource_attrs {
            validate_attr_key(key).map_err(MyOtelError::Config)?;
        }

        if self.export_timeout.is_zero() {
            return Err(ConfigError::InvalidExportTimeout.into());
        }

        if self.shutdown_timeout.is_zero() {
            return Err(ConfigError::InvalidShutdownTimeout.into());
        }

        Ok(())
    }
}

fn validate_otlp_endpoint(endpoint: &str) -> std::result::Result<(), ConfigError> {
    let uri = endpoint
        .parse::<http::Uri>()
        .map_err(|_| ConfigError::InvalidOtlpEndpoint(endpoint.to_owned()))?;

    match (uri.scheme_str(), uri.authority()) {
        (Some("http" | "https"), Some(_)) => Ok(()),
        _ => Err(ConfigError::InvalidOtlpEndpoint(endpoint.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_with_defaults() {
        let config = TracingConfig::builder("service-a")
            .build()
            .expect("valid default config");

        assert_eq!(config.service_name, "service-a");
        assert_eq!(config.environment, "local");
        assert_eq!(config.otlp_endpoint, "http://localhost:4318/v1/traces");
        assert_eq!(config.sampling, SamplingMode::AlwaysOn);
        assert_eq!(config.log_format, LogFormat::Json);
    }

    #[test]
    fn rejects_empty_service_name() {
        assert!(matches!(
            TracingConfig::builder(" ").build(),
            Err(MyOtelError::Config(ConfigError::EmptyServiceName))
        ));
    }

    #[test]
    fn rejects_invalid_otlp_endpoint() {
        assert!(matches!(
            TracingConfig::builder("service-a")
                .otlp_endpoint("localhost:4318/v1/traces")
                .build(),
            Err(MyOtelError::Config(ConfigError::InvalidOtlpEndpoint(_)))
        ));
    }

    #[test]
    fn rejects_reserved_resource_attr() {
        assert!(matches!(
            TracingConfig::builder("service-a")
                .resource_attr("trace_id", "value")
                .build(),
            Err(MyOtelError::Config(ConfigError::ReservedAttributeKey(_)))
        ));
    }

    #[test]
    fn rejects_zero_timeouts() {
        assert!(matches!(
            TracingConfig::builder("service-a")
                .export_timeout(Duration::ZERO)
                .build(),
            Err(MyOtelError::Config(ConfigError::InvalidExportTimeout))
        ));

        assert!(matches!(
            TracingConfig::builder("service-a")
                .shutdown_timeout(Duration::ZERO)
                .build(),
            Err(MyOtelError::Config(ConfigError::InvalidShutdownTimeout))
        ));
    }
}
