use crate::{
    AttributeKey,
    error::{ConfigError, MyOtelError, Result},
};

const DEFAULT_MAX_VALUE_LEN: usize = 128;
const DEFAULT_MAX_CAPTURED_HEADERS: usize = 16;
const DENYLISTED_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
    "x-csrf-token",
    "x-forwarded-client-cert",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderCapturePolicy {
    rules: Vec<HeaderCaptureRule>,
    max_value_len: usize,
    max_captured_headers: usize,
    non_utf8: NonUtf8Policy,
}

impl HeaderCapturePolicy {
    pub fn builder() -> HeaderCapturePolicyBuilder {
        HeaderCapturePolicyBuilder::default()
    }

    pub(crate) fn empty() -> Self {
        Self {
            rules: Vec::new(),
            max_value_len: DEFAULT_MAX_VALUE_LEN,
            max_captured_headers: DEFAULT_MAX_CAPTURED_HEADERS,
            non_utf8: NonUtf8Policy::Skip,
        }
    }

    pub fn rules(&self) -> &[HeaderCaptureRule] {
        &self.rules
    }

    pub fn max_value_len(&self) -> usize {
        self.max_value_len
    }

    pub fn max_captured_headers(&self) -> usize {
        self.max_captured_headers
    }

    pub fn non_utf8(&self) -> NonUtf8Policy {
        self.non_utf8
    }

    #[cfg(any(feature = "otlp", test))]
    pub(crate) fn captured_attributes(
        &self,
        headers: &http::HeaderMap,
    ) -> Vec<CapturedHeaderAttribute> {
        let mut attributes = Vec::new();

        for rule in &self.rules {
            if attributes.len() >= self.max_captured_headers {
                break;
            }

            if is_denylisted(rule.header_name.as_str()) {
                continue;
            }

            let Some(value) = headers.get(&rule.header_name) else {
                continue;
            };

            let value = match value.to_str() {
                Ok(value) => capture_header_value(value, rule.mode),
                Err(_) => match self.non_utf8 {
                    NonUtf8Policy::Skip => continue,
                    NonUtf8Policy::MarkPresent => HeaderValueCapture::Present,
                },
            };

            attributes.push(CapturedHeaderAttribute {
                key: rule.attr_key.as_str().to_owned(),
                value,
            });
        }

        attributes
    }
}

impl Default for HeaderCapturePolicy {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderCaptureRule {
    header_name: http::HeaderName,
    attr_key: AttributeKey,
    mode: HeaderValueMode,
}

impl HeaderCaptureRule {
    pub fn header_name(&self) -> &http::HeaderName {
        &self.header_name
    }

    pub fn attr_key(&self) -> &AttributeKey {
        &self.attr_key
    }

    pub fn mode(&self) -> &HeaderValueMode {
        &self.mode
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderValueMode {
    Raw,
    Truncated { max_len: usize },
    Redacted,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonUtf8Policy {
    Skip,
    MarkPresent,
}

#[cfg(any(feature = "otlp", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapturedHeaderAttribute {
    pub(crate) key: String,
    pub(crate) value: HeaderValueCapture,
}

#[cfg(any(feature = "otlp", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HeaderValueCapture {
    String(String),
    Present,
}

#[derive(Debug, Clone)]
pub struct HeaderCapturePolicyBuilder {
    rules: Vec<PendingHeaderRule>,
    max_value_len: usize,
    max_captured_headers: usize,
    non_utf8: NonUtf8Policy,
}

impl Default for HeaderCapturePolicyBuilder {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            max_value_len: DEFAULT_MAX_VALUE_LEN,
            max_captured_headers: DEFAULT_MAX_CAPTURED_HEADERS,
            non_utf8: NonUtf8Policy::Skip,
        }
    }
}

impl HeaderCapturePolicyBuilder {
    pub fn standard_request_ids(self) -> Self {
        self.preset_header("x-request-id", "request.id")
            .preset_header("x-correlation-id", "correlation.id")
    }

    pub fn gateway_headers(self) -> Self {
        self.preset_header("x-forwarded-proto", "http.forwarded.proto")
            .preset_header("x-forwarded-host", "http.forwarded.host")
            .preset_header("x-envoy-attempt-count", "http.gateway.attempt_count")
            .preset_header("cf-ray", "cloudflare.ray_id")
    }

    pub fn header(self, header_name: impl AsRef<str>, attr_key: impl AsRef<str>) -> Self {
        self.header_rule(header_name, attr_key, None)
    }

    pub fn header_with(
        self,
        header_name: impl AsRef<str>,
        attr_key: impl AsRef<str>,
        mode: HeaderValueMode,
    ) -> Self {
        self.header_rule(header_name, attr_key, Some(mode))
    }

    pub fn max_value_len(mut self, max_value_len: usize) -> Self {
        self.max_value_len = max_value_len;
        self
    }

    pub fn max_captured_headers(mut self, max_captured_headers: usize) -> Self {
        self.max_captured_headers = max_captured_headers;
        self
    }

    pub fn non_utf8(mut self, non_utf8: NonUtf8Policy) -> Self {
        self.non_utf8 = non_utf8;
        self
    }

    pub fn build(self) -> Result<HeaderCapturePolicy> {
        validate_value_limit(self.max_value_len)?;
        validate_value_limit(self.max_captured_headers)?;

        let mut rules = Vec::with_capacity(self.rules.len());

        for pending in self.rules {
            let header_name = parse_header_name(&pending.header_name)?;

            if is_denylisted(header_name.as_str()) {
                return Err(ConfigError::ReservedHeaderName(pending.header_name).into());
            }

            if rules
                .iter()
                .any(|rule: &HeaderCaptureRule| rule.header_name == header_name)
            {
                return Err(
                    ConfigError::DuplicateHeaderRule(header_name.as_str().to_owned()).into(),
                );
            }

            let attr_key = parse_attr_key(&pending.attr_key)?;
            let mode = pending.mode.unwrap_or(HeaderValueMode::Truncated {
                max_len: self.max_value_len,
            });
            validate_mode(mode)?;

            rules.push(HeaderCaptureRule {
                header_name,
                attr_key,
                mode,
            });
        }

        Ok(HeaderCapturePolicy {
            rules,
            max_value_len: self.max_value_len,
            max_captured_headers: self.max_captured_headers,
            non_utf8: self.non_utf8,
        })
    }

    fn header_rule(
        mut self,
        header_name: impl AsRef<str>,
        attr_key: impl AsRef<str>,
        mode: Option<HeaderValueMode>,
    ) -> Self {
        self.rules.push(PendingHeaderRule {
            header_name: header_name.as_ref().to_owned(),
            attr_key: attr_key.as_ref().to_owned(),
            mode,
        });
        self
    }

    fn preset_header(self, header_name: &'static str, attr_key: &'static str) -> Self {
        if self.rules.iter().any(|rule| {
            rule.header_name.eq_ignore_ascii_case(header_name)
                && rule.attr_key == attr_key
                && rule.mode.is_none()
        }) {
            return self;
        }

        self.header_rule(header_name, attr_key, None)
    }
}

#[derive(Debug, Clone)]
struct PendingHeaderRule {
    header_name: String,
    attr_key: String,
    mode: Option<HeaderValueMode>,
}

fn parse_header_name(value: &str) -> Result<http::HeaderName> {
    http::HeaderName::from_bytes(value.as_bytes())
        .map_err(|_| MyOtelError::Config(ConfigError::InvalidHeaderName(value.to_owned())))
}

fn parse_attr_key(value: &str) -> Result<AttributeKey> {
    AttributeKey::new(value.to_owned()).map_err(|err| match err {
        MyOtelError::Config(ConfigError::ReservedAttributeKey(key)) => {
            MyOtelError::Config(ConfigError::InvalidHeaderAttributeKey(key))
        }
        MyOtelError::Config(ConfigError::InvalidResourceAttributeKey(key)) => {
            MyOtelError::Config(ConfigError::InvalidHeaderAttributeKey(key))
        }
        err => err,
    })
}

fn validate_mode(mode: HeaderValueMode) -> Result<()> {
    if let HeaderValueMode::Truncated { max_len: 0 } = mode {
        return Err(ConfigError::InvalidHeaderValueLimit.into());
    }

    Ok(())
}

fn validate_value_limit(value: usize) -> Result<()> {
    if value == 0 {
        return Err(ConfigError::InvalidHeaderValueLimit.into());
    }

    Ok(())
}

fn is_denylisted(header_name: &str) -> bool {
    DENYLISTED_HEADERS
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(header_name))
}

#[cfg(any(feature = "otlp", test))]
fn capture_header_value(value: &str, mode: HeaderValueMode) -> HeaderValueCapture {
    match mode {
        HeaderValueMode::Raw => HeaderValueCapture::String(value.to_owned()),
        HeaderValueMode::Truncated { max_len } => {
            HeaderValueCapture::String(truncate_value(value, max_len))
        }
        HeaderValueMode::Redacted => HeaderValueCapture::String("redacted".to_owned()),
        HeaderValueMode::Present => HeaderValueCapture::Present,
    }
}

#[cfg(any(feature = "otlp", test))]
fn truncate_value(value: &str, max_len: usize) -> String {
    value.chars().take(max_len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_allowlisted_header() {
        let policy = HeaderCapturePolicy::builder()
            .header_with("x-request-id", "request.id", HeaderValueMode::Raw)
            .build()
            .expect("valid policy");
        let mut headers = http::HeaderMap::new();
        headers.insert("x-request-id", "req-123".parse().expect("valid value"));

        let attributes = policy.captured_attributes(&headers);

        assert_eq!(
            attributes,
            vec![CapturedHeaderAttribute {
                key: "request.id".to_owned(),
                value: HeaderValueCapture::String("req-123".to_owned())
            }]
        );
    }

    #[test]
    fn skips_missing_header() {
        let policy = HeaderCapturePolicy::builder()
            .header("x-request-id", "request.id")
            .build()
            .expect("valid policy");

        assert!(
            policy
                .captured_attributes(&http::HeaderMap::new())
                .is_empty()
        );
    }

    #[test]
    fn truncates_long_value() {
        let policy = HeaderCapturePolicy::builder()
            .header_with(
                "x-request-id",
                "request.id",
                HeaderValueMode::Truncated { max_len: 3 },
            )
            .build()
            .expect("valid policy");
        let mut headers = http::HeaderMap::new();
        headers.insert("x-request-id", "abcdef".parse().expect("valid value"));

        let attributes = policy.captured_attributes(&headers);

        assert_eq!(
            attributes[0].value,
            HeaderValueCapture::String("abc".to_owned())
        );
    }

    #[test]
    fn redacts_value() {
        let policy = HeaderCapturePolicy::builder()
            .header_with("x-user-id", "user.id", HeaderValueMode::Redacted)
            .build()
            .expect("valid policy");
        let mut headers = http::HeaderMap::new();
        headers.insert("x-user-id", "user-123".parse().expect("valid value"));

        let attributes = policy.captured_attributes(&headers);

        assert_eq!(
            attributes[0].value,
            HeaderValueCapture::String("redacted".to_owned())
        );
    }

    #[test]
    fn records_presence_only() {
        let policy = HeaderCapturePolicy::builder()
            .header_with("x-debug", "debug.present", HeaderValueMode::Present)
            .build()
            .expect("valid policy");
        let mut headers = http::HeaderMap::new();
        headers.insert("x-debug", "secret".parse().expect("valid value"));

        let attributes = policy.captured_attributes(&headers);

        assert_eq!(attributes[0].value, HeaderValueCapture::Present);
    }

    #[test]
    fn skips_non_utf8_by_default() {
        let policy = HeaderCapturePolicy::builder()
            .header("x-bin", "bin.present")
            .build()
            .expect("valid policy");
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "x-bin",
            http::HeaderValue::from_bytes(&[0xff]).expect("valid opaque value"),
        );

        assert!(policy.captured_attributes(&headers).is_empty());
    }

    #[test]
    fn marks_non_utf8_present_when_configured() {
        let policy = HeaderCapturePolicy::builder()
            .header("x-bin", "bin.present")
            .non_utf8(NonUtf8Policy::MarkPresent)
            .build()
            .expect("valid policy");
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "x-bin",
            http::HeaderValue::from_bytes(&[0xff]).expect("valid opaque value"),
        );

        let attributes = policy.captured_attributes(&headers);

        assert_eq!(attributes[0].value, HeaderValueCapture::Present);
    }

    #[test]
    fn respects_max_captured_headers() {
        let policy = HeaderCapturePolicy::builder()
            .header("x-one", "one")
            .header("x-two", "two")
            .max_captured_headers(1)
            .build()
            .expect("valid policy");
        let mut headers = http::HeaderMap::new();
        headers.insert("x-one", "1".parse().expect("valid value"));
        headers.insert("x-two", "2".parse().expect("valid value"));

        let attributes = policy.captured_attributes(&headers);

        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].key, "one");
    }
}
