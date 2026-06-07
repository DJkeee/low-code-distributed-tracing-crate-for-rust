use crate::AttributeKey;

#[derive(Debug, Clone, PartialEq)]
pub struct EventField {
    key: AttributeKey,
    value: EventValue,
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
        let key = AttributeKey::new(key.as_ref().to_owned())
            .unwrap_or_else(|_| AttributeKey::unchecked("event.invalid_key"));
        Self { key, value }
    }

    pub fn key(&self) -> &AttributeKey {
        &self.key
    }

    pub fn value(&self) -> &EventValue {
        &self.value
    }
}

pub fn record_event<I>(name: impl Into<String>, fields: I)
where
    I: IntoIterator<Item = EventField>,
{
    let name = name.into();
    let fields = fields.into_iter().collect::<Vec<_>>();
    tracing::info!(event.name = %name, field.count = fields.len(), "business event");
}
