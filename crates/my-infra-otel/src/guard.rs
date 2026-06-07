use std::time::Duration;

#[cfg(feature = "otlp")]
use crate::error::MyOtelError;
use crate::{TracingConfig, error::Result};

#[derive(Debug)]
pub struct TracingGuard {
    shutdown_timeout: Duration,
    #[cfg(feature = "otlp")]
    tracer_provider: Option<opentelemetry_sdk::trace::TracerProvider>,
}

impl TracingGuard {
    pub(crate) fn new(
        config: &TracingConfig,
        #[cfg(feature = "otlp")] tracer_provider: opentelemetry_sdk::trace::TracerProvider,
    ) -> Self {
        Self {
            shutdown_timeout: config.shutdown_timeout,
            #[cfg(feature = "otlp")]
            tracer_provider: Some(tracer_provider),
        }
    }

    #[cfg(feature = "otlp")]
    pub fn shutdown(mut self) -> Result<()> {
        if let Some(provider) = self.tracer_provider.take() {
            provider
                .shutdown()
                .map_err(|err| MyOtelError::Shutdown(err.to_string()))?;
        }

        Ok(())
    }

    #[cfg(not(feature = "otlp"))]
    pub fn shutdown(self) -> Result<()> {
        Ok(())
    }

    pub fn shutdown_timeout(&self) -> Duration {
        self.shutdown_timeout
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        #[cfg(feature = "otlp")]
        if let Some(provider) = self.tracer_provider.take() {
            let _ = provider.shutdown();
        }
    }
}
