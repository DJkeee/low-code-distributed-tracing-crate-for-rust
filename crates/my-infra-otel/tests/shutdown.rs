use std::time::Duration;

use my_infra_otel::{MyOtelError, TracingConfig, init_global_tracing};

#[test]
fn guard_shutdown_flushes_without_panic() {
    let config = TracingConfig::builder("shutdown-test")
        .otlp_endpoint("http://127.0.0.1:9/v1/traces")
        .export_timeout(Duration::from_millis(50))
        .shutdown_timeout(Duration::from_millis(50))
        .build()
        .expect("valid config");
    let expected = config.shutdown_timeout;
    let guard = init_global_tracing(config).expect("first init succeeds");

    assert_eq!(guard.shutdown_timeout(), expected);
    assert!(matches!(
        init_global_tracing(
            TracingConfig::builder("shutdown-test-second")
                .otlp_endpoint("http://127.0.0.1:9/v1/traces")
                .export_timeout(Duration::from_millis(50))
                .shutdown_timeout(Duration::from_millis(50))
                .build()
                .expect("valid config")
        ),
        Err(MyOtelError::AlreadyInitialized)
    ));
    guard.shutdown().expect("shutdown succeeds");
}
