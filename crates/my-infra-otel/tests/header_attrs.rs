use my_infra_otel::{HeaderAttr, MyOtelError};

#[test]
fn public_header_attr_validates_header_name() {
    let attr = HeaderAttr::new("x-user-id", "user.id").expect("valid header attr");

    assert_eq!(attr.header_name(), "x-user-id");
    assert_eq!(attr.attr_key().as_str(), "user.id");
}

#[test]
fn public_header_attr_rejects_invalid_header_name() {
    assert!(matches!(
        HeaderAttr::new("bad header", "user.id"),
        Err(MyOtelError::HeaderAttr(_))
    ));
}
