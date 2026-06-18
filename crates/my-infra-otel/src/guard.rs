use std::time::Duration;
#[cfg(feature = "otlp")]
use std::{sync::mpsc, thread};

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
            shutdown_provider(provider, self.shutdown_timeout)?;
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

#[cfg(feature = "otlp")]
fn shutdown_provider(
    provider: opentelemetry_sdk::trace::TracerProvider,
    timeout: Duration,
) -> Result<()> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let result = provider.shutdown().map_err(|err| err.to_string());
        let _ = sender.send(result);
    });

    match receiver.recv_timeout(timeout) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => Err(MyOtelError::Shutdown(err)),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(MyOtelError::Shutdown(format!(
            "shutdown timed out after {timeout:?}"
        ))),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(MyOtelError::Shutdown(
            "shutdown worker disconnected".to_owned(),
        )),
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
