use my_infra_otel::{EventField, record_event};

#[test]
fn record_event_is_safe_without_initialized_subscriber() {
    record_event(
        "checkout.started",
        [
            EventField::string("operation.name", "checkout"),
            EventField::bool("operation.ok", true),
        ],
    );
}
