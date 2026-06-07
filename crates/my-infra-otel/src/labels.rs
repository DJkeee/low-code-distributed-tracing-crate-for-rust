use crate::error::{ConfigError, MyOtelError, Result};

const RESERVED_FIELDS: &[&str] = &[
    "timestamp",
    "level",
    "target",
    "message",
    "trace_id",
    "span_id",
    "service.name",
    "service.version",
    "deployment.environment",
    "event.name",
];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeKey(String);

impl AttributeKey {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_attr_key(&value).map_err(MyOtelError::Config)?;
        Ok(Self(value))
    }

    pub(crate) fn unchecked(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderAttr {
    header_name: http::HeaderName,
    attr_key: AttributeKey,
}

impl HeaderAttr {
    pub fn new(header_name: impl AsRef<str>, attr_key: impl AsRef<str>) -> Result<Self> {
        let header_name = http::HeaderName::from_bytes(header_name.as_ref().as_bytes())
            .map_err(|err| MyOtelError::HeaderAttr(err.to_string()))?;
        let attr_key = AttributeKey::new(attr_key.as_ref().to_owned())?;

        Ok(Self {
            header_name,
            attr_key,
        })
    }

    pub fn header_name(&self) -> &http::HeaderName {
        &self.header_name
    }

    pub fn attr_key(&self) -> &AttributeKey {
        &self.attr_key
    }
}

pub(crate) fn validate_attr_key(value: &str) -> std::result::Result<(), ConfigError> {
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        return Err(ConfigError::InvalidResourceAttributeKey(value.to_owned()));
    }

    if RESERVED_FIELDS.contains(&value) {
        return Err(ConfigError::ReservedAttributeKey(value.to_owned()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_dot_separated_attr_key() {
        let key = AttributeKey::new("user.id").expect("valid attribute key");
        assert_eq!(key.as_str(), "user.id");
    }

    #[test]
    fn rejects_empty_attr_key() {
        assert!(matches!(
            AttributeKey::new(""),
            Err(MyOtelError::Config(
                ConfigError::InvalidResourceAttributeKey(_)
            ))
        ));
    }

    #[test]
    fn rejects_reserved_attr_key() {
        assert!(matches!(
            AttributeKey::new("trace_id"),
            Err(MyOtelError::Config(ConfigError::ReservedAttributeKey(_)))
        ));
    }

    #[test]
    fn accepts_header_attr() {
        let attr = HeaderAttr::new("x-user-id", "user.id").expect("valid header attr");
        assert_eq!(attr.header_name(), "x-user-id");
        assert_eq!(attr.attr_key().as_str(), "user.id");
    }

    #[test]
    fn rejects_invalid_header_name() {
        assert!(matches!(
            HeaderAttr::new("bad header", "user.id"),
            Err(MyOtelError::HeaderAttr(_))
        ));
    }
}
