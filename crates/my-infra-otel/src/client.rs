use crate::TracedRequestBuilder;

#[derive(Debug, Clone)]
pub struct TracedHttpClient {
    inner: reqwest::Client,
}

impl TracedHttpClient {
    pub fn new(inner: reqwest::Client) -> Self {
        Self { inner }
    }

    pub fn request(
        &self,
        method: reqwest::Method,
        url: impl reqwest::IntoUrl,
    ) -> TracedRequestBuilder {
        TracedRequestBuilder::new(self.inner.request(method.clone(), url), method)
    }

    pub fn get(&self, url: impl reqwest::IntoUrl) -> TracedRequestBuilder {
        self.request(reqwest::Method::GET, url)
    }

    pub fn post(&self, url: impl reqwest::IntoUrl) -> TracedRequestBuilder {
        self.request(reqwest::Method::POST, url)
    }
}
