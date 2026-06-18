use my_infra_otel::{
    ConfigError, HeaderCapturePolicy, HeaderValueMode, MyOtelError, NonUtf8Policy,
};

#[test]
fn header_capture_policy_uses_truncated_mode_by_default() {
    let policy = HeaderCapturePolicy::builder()
        .header("x-request-id", "request.id")
        .build()
        .expect("valid header capture policy");

    assert_eq!(policy.rules().len(), 1);
    assert_eq!(policy.rules()[0].header_name(), "x-request-id");
    assert_eq!(policy.rules()[0].attr_key().as_str(), "request.id");
    assert_eq!(
        policy.rules()[0].mode(),
        &HeaderValueMode::Truncated {
            max_len: policy.max_value_len()
        }
    );
}

#[test]
fn header_capture_policy_rejects_denylisted_header() {
    let err = HeaderCapturePolicy::builder()
        .header("authorization", "http.authorization")
        .build()
        .expect_err("denylisted header must fail");

    assert!(matches!(
        err,
        MyOtelError::Config(ConfigError::ReservedHeaderName(_))
    ));
}

#[test]
fn header_capture_policy_rejects_duplicate_header_rule() {
    let err = HeaderCapturePolicy::builder()
        .header("x-request-id", "request.id")
        .header("X-Request-Id", "request.id.alt")
        .build()
        .expect_err("duplicate header rule must fail");

    assert!(matches!(
        err,
        MyOtelError::Config(ConfigError::DuplicateHeaderRule(_))
    ));
}

#[test]
fn header_capture_policy_rejects_invalid_header_name() {
    let err = HeaderCapturePolicy::builder()
        .header("bad header", "request.id")
        .build()
        .expect_err("invalid header name must fail");

    assert!(matches!(
        err,
        MyOtelError::Config(ConfigError::InvalidHeaderName(_))
    ));
}

#[test]
fn header_capture_policy_rejects_invalid_attr_key() {
    let err = HeaderCapturePolicy::builder()
        .header("x-request-id", "bad key")
        .build()
        .expect_err("invalid attr key must fail");

    assert!(matches!(
        err,
        MyOtelError::Config(ConfigError::InvalidHeaderAttributeKey(_))
    ));
}

#[test]
fn header_capture_policy_rejects_zero_value_limits() {
    let err = HeaderCapturePolicy::builder()
        .header("x-request-id", "request.id")
        .max_value_len(0)
        .build()
        .expect_err("zero max value length must fail");

    assert!(matches!(
        err,
        MyOtelError::Config(ConfigError::InvalidHeaderValueLimit)
    ));

    let err = HeaderCapturePolicy::builder()
        .header_with(
            "x-request-id",
            "request.id",
            HeaderValueMode::Truncated { max_len: 0 },
        )
        .build()
        .expect_err("zero per-rule max value length must fail");

    assert!(matches!(
        err,
        MyOtelError::Config(ConfigError::InvalidHeaderValueLimit)
    ));
}

#[test]
fn header_capture_policy_accepts_explicit_modes_and_non_utf8_policy() {
    let policy = HeaderCapturePolicy::builder()
        .header_with("x-user-id", "user.id", HeaderValueMode::Redacted)
        .header_with("x-debug-present", "debug.present", HeaderValueMode::Present)
        .non_utf8(NonUtf8Policy::MarkPresent)
        .max_captured_headers(2)
        .build()
        .expect("valid header capture policy");

    assert_eq!(policy.rules()[0].mode(), &HeaderValueMode::Redacted);
    assert_eq!(policy.rules()[1].mode(), &HeaderValueMode::Present);
    assert_eq!(policy.non_utf8(), NonUtf8Policy::MarkPresent);
    assert_eq!(policy.max_captured_headers(), 2);
}

#[test]
fn standard_request_ids_adds_expected_headers() {
    let policy = HeaderCapturePolicy::builder()
        .standard_request_ids()
        .build()
        .expect("valid header capture policy");

    assert_eq!(policy.rules().len(), 2);
    assert_rule(&policy, "x-request-id", "request.id");
    assert_rule(&policy, "x-correlation-id", "correlation.id");
}

#[test]
fn gateway_headers_adds_expected_headers() {
    let policy = HeaderCapturePolicy::builder()
        .gateway_headers()
        .build()
        .expect("valid header capture policy");

    assert_eq!(policy.rules().len(), 4);
    assert_rule(&policy, "x-forwarded-proto", "http.forwarded.proto");
    assert_rule(&policy, "x-forwarded-host", "http.forwarded.host");
    assert_rule(
        &policy,
        "x-envoy-attempt-count",
        "http.gateway.attempt_count",
    );
    assert_rule(&policy, "cf-ray", "cloudflare.ray_id");
}

#[test]
fn repeated_profiles_do_not_create_duplicates() {
    let policy = HeaderCapturePolicy::builder()
        .standard_request_ids()
        .standard_request_ids()
        .gateway_headers()
        .gateway_headers()
        .build()
        .expect("valid header capture policy");

    assert_eq!(policy.rules().len(), 6);
}

fn assert_rule(policy: &HeaderCapturePolicy, header_name: &str, attr_key: &str) {
    let rule = policy
        .rules()
        .iter()
        .find(|rule| rule.header_name() == header_name)
        .expect("rule must exist");

    assert_eq!(rule.attr_key().as_str(), attr_key);
    assert_eq!(
        rule.mode(),
        &HeaderValueMode::Truncated {
            max_len: policy.max_value_len()
        }
    );
}
