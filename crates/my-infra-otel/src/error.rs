pub type Result<T> = std::result::Result<T, MyOtelError>;

#[derive(Debug, thiserror::Error)]
pub enum MyOtelError {
    #[error("invalid configuration: {0}")]
    Config(#[from] ConfigError),

    #[error("global tracing subscriber is already initialized")]
    AlreadyInitialized,

    #[error("failed to initialize OpenTelemetry exporter: {0}")]
    ExporterInit(String),

    #[error("failed to initialize tracing subscriber: {0}")]
    SubscriberInit(String),

    #[cfg(feature = "reqwest-client")]
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("trace shutdown failed: {0}")]
    Shutdown(String),

    #[error("invalid header attribute: {0}")]
    HeaderAttr(String),

    #[error("invalid event field: {0}")]
    EventField(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("service name cannot be empty")]
    EmptyServiceName,

    #[error("invalid OTLP endpoint: {0}")]
    InvalidOtlpEndpoint(String),

    #[error("invalid resource attribute key: {0}")]
    InvalidResourceAttributeKey(String),

    #[error("reserved attribute key cannot be used: {0}")]
    ReservedAttributeKey(String),

    #[error("export timeout must be greater than zero")]
    InvalidExportTimeout,

    #[error("shutdown timeout must be greater than zero")]
    InvalidShutdownTimeout,
}
