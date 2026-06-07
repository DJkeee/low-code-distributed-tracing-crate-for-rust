use my_infra_otel::{TracingConfig, init_global_tracing};

#[test]
fn guard_exposes_shutdown_timeout() {
    let config = TracingConfig::builder("shutdown-test")
        .build()
        .expect("valid config");
    let expected = config.shutdown_timeout;
    let guard = init_global_tracing(config).expect("first init succeeds");

    assert_eq!(guard.shutdown_timeout(), expected);
}
