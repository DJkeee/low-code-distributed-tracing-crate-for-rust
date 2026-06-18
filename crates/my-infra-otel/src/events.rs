use serde_json::{Map, Number, Value};

use crate::AttributeKey;

#[derive(Debug, Clone, PartialEq)]
pub struct EventField {
    key: AttributeKey,
    value: EventValue,
    invalid_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventValue {
    String(String),
    Bool(bool),
    I64(i64),
    F64(f64),
}

impl EventField {
    pub fn string(key: impl AsRef<str>, value: impl Into<String>) -> Self {
        Self::new(key, EventValue::String(value.into()))
    }

    pub fn bool(key: impl AsRef<str>, value: bool) -> Self {
        Self::new(key, EventValue::Bool(value))
    }

    pub fn i64(key: impl AsRef<str>, value: i64) -> Self {
        Self::new(key, EventValue::I64(value))
    }

    pub fn f64(key: impl AsRef<str>, value: f64) -> Self {
        Self::new(key, EventValue::F64(value))
    }

    fn new(key: impl AsRef<str>, value: EventValue) -> Self {
        let raw_key = key.as_ref();
        match AttributeKey::new(raw_key.to_owned()) {
            Ok(key) => Self {
                key,
                value,
                invalid_key: None,
            },
            Err(_) => Self {
                key: AttributeKey::unchecked("event.invalid_key"),
                value,
                invalid_key: Some(raw_key.to_owned()),
            },
        }
    }

    pub fn key(&self) -> &AttributeKey {
        &self.key
    }

    pub fn value(&self) -> &EventValue {
        &self.value
    }

    pub fn invalid_key(&self) -> Option<&str> {
        self.invalid_key.as_deref()
    }

    fn into_json_parts(self) -> (String, Value, Option<String>) {
        (
            self.key.as_str().to_owned(),
            self.value.into_json_value(),
            self.invalid_key,
        )
    }
}

impl EventValue {
    fn into_json_value(self) -> Value {
        match self {
            EventValue::String(value) => Value::String(value),
            EventValue::Bool(value) => Value::Bool(value),
            EventValue::I64(value) => Value::Number(Number::from(value)),
            EventValue::F64(value) => Number::from_f64(value)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        }
    }
}

pub fn record_event<I>(name: impl Into<String>, fields: I)
where
    I: IntoIterator<Item = EventField>,
{
    let name = name.into();
    let mut invalid_keys = Vec::new();
    let mut event_fields = Map::new();

    for field in fields {
        let (key, value, invalid_key) = field.into_json_parts();
        if let Some(invalid_key) = invalid_key {
            invalid_keys.push(Value::String(invalid_key));
        }
        event_fields.insert(key, value);
    }

    if !invalid_keys.is_empty() {
        event_fields.insert("event.invalid_keys".to_owned(), Value::Array(invalid_keys));
    }

    let event_fields = Value::Object(event_fields).to_string();

    tracing::info!(event.name = %name, event.fields = %event_fields, "business event");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_field_preserves_key_and_value() {
        let field = EventField::string("order.id", "123");

        assert_eq!(field.key().as_str(), "order.id");
        assert_eq!(field.value(), &EventValue::String("123".to_owned()));
        assert_eq!(field.invalid_key(), None);
    }

    #[test]
    fn event_field_exposes_invalid_key() {
        let field = EventField::string("trace_id", "abc");

        assert_eq!(field.key().as_str(), "event.invalid_key");
        assert_eq!(field.value(), &EventValue::String("abc".to_owned()));
        assert_eq!(field.invalid_key(), Some("trace_id"));
    }

    #[test]
    fn event_value_serializes_supported_types() {
        assert_eq!(
            EventValue::String("value".to_owned()).into_json_value(),
            Value::String("value".to_owned())
        );
        assert_eq!(EventValue::Bool(true).into_json_value(), Value::Bool(true));
        assert_eq!(
            EventValue::I64(42).into_json_value(),
            Value::Number(Number::from(42))
        );
        assert_eq!(
            EventValue::F64(1.5).into_json_value(),
            Value::Number(Number::from_f64(1.5).expect("finite f64"))
        );
    }
}
