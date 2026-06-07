use my_infra_otel::{ConfigError, MyOtelError, TracingConfig};

#[test]
fn public_builder_builds_default_config() {
    let config = TracingConfig::builder("service-a")
        .build()
        .expect("valid default config");

    assert_eq!(config.service_name, "service-a");
}

#[test]
fn public_builder_returns_typed_error() {
    assert!(matches!(
        TracingConfig::builder("").build(),
        Err(MyOtelError::Config(ConfigError::EmptyServiceName))
    ));
}
