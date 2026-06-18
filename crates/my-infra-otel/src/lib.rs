#![doc = include_str!("../../../README.md")]

pub mod config;
pub mod error;
pub mod events;
pub mod guard;
pub mod header_capture;
pub mod init;
pub mod labels;
pub mod layer;
pub mod logging;
pub mod propagation;

#[cfg(feature = "reqwest-client")]
pub mod client;
#[cfg(feature = "reqwest-client")]
pub mod request_builder;

pub(crate) mod internal;

#[cfg(feature = "reqwest-client")]
pub use crate::client::TracedHttpClient;
pub use crate::config::{LogFormat, SamplingMode, TracingConfig, TracingConfigBuilder};
pub use crate::error::{ConfigError, MyOtelError, Result};
pub use crate::events::{EventField, EventValue, record_event};
pub use crate::guard::TracingGuard;
pub use crate::header_capture::{
    HeaderCapturePolicy, HeaderCapturePolicyBuilder, HeaderCaptureRule, HeaderValueMode,
    NonUtf8Policy,
};
pub use crate::init::init_global_tracing;
pub use crate::labels::{AttributeKey, HeaderAttr};
pub use crate::layer::{MyOtelTracingLayer, MyOtelTracingLayerBuilder};
#[cfg(feature = "reqwest-client")]
pub use crate::request_builder::TracedRequestBuilder;
