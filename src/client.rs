//! Main Gemini API client implementation

use crate::{
    config::{ApiVersion, GeminiConfig},
    error::{Error, Result},
    models::*,
};

#[cfg(feature = "caching")]
use crate::cache::CacheManager;
use reqwest::{Client as HttpClient, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, instrument, warn};

/// Main Gemini API client
#[derive(Clone)]
pub struct GeminiClient {
    config: Arc<GeminiConfig>,
    http_client: HttpClient,
    #[cfg(feature = "caching")]
    cache_manager: Arc<CacheManager>,
}

impl GeminiClient {
    /// Create a new client with the given configuration
    pub fn new(config: GeminiConfig) -> Result<Self> {
        let http_client = Self::build_http_client(&config)?;
        #[cfg(feature = "caching")]
        let cache_manager = Arc::new(CacheManager::new());

        Ok(Self {
            config: Arc::new(config),
            http_client,
            #[cfg(feature = "caching")]
            cache_manager,
        })
    }

    /// Create a new client from environment variables
    pub fn from_env() -> Result<Self> {
        let config = GeminiConfig::from_env()?;
        Self::new(config)
    }

    /// Get a builder for creating a customized client
    pub fn builder() -> GeminiClientBuilder {
        GeminiClientBuilder::default()
    }

    /// Generate content with the Gemini API
    #[instrument(skip(self, request))]
    pub async fn generate_content(
        &self,
        model: Option<&str>,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let model_name = self.config.get_model_name(model);
        let endpoint = format!(
            "{}/{}/models/{}:generateContent",
            self.config.base_url,
            self.config.api_version.as_str(),
            model_name
        );

        debug!("Generating content with model: {}", model_name);

        self.execute_with_retry(|client| {
            client
                .http_client
                .post(&endpoint)
                .query(&[("key", &client.config.api_key)])
                .json(&request)
        })
        .await
    }

    /// Stream content generation
    #[cfg(feature = "streaming")]
    #[instrument(skip(self, request))]
    pub async fn stream_generate_content(
        &self,
        model: Option<&str>,
        request: GenerateContentRequest,
    ) -> Result<impl futures::Stream<Item = Result<GenerateContentResponse>>> {
        let model_name = self.config.get_model_name(model);
        let endpoint = format!(
            "{}/{}/models/{}:streamGenerateContent",
            self.config.base_url,
            self.config.api_version.as_str(),
            model_name
        );

        debug!("Streaming content with model: {}", model_name);

        let response = self
            .http_client
            .post(&endpoint)
            .query(&[("key", &self.config.api_key)])
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.handle_api_error(status, error_body));
        }

        #[cfg(feature = "streaming")]
        {
            Ok(crate::streaming::parse_stream(response))
        }
        #[cfg(not(feature = "streaming"))]
        {
            Err(Error::Config("Streaming feature not enabled".to_string()))
        }
    }

    /// Count tokens for the given content
    #[instrument(skip(self, contents))]
    pub async fn count_tokens(
        &self,
        model: Option<&str>,
        contents: Vec<Content>,
    ) -> Result<CountTokensResponse> {
        let model_name = self.config.get_model_name(model);
        let endpoint = format!(
            "{}/{}/models/{}:countTokens",
            self.config.base_url,
            self.config.api_version.as_str(),
            model_name
        );

        let request = CountTokensRequest { contents };

        self.execute_with_retry(|client| {
            client
                .http_client
                .post(&endpoint)
                .query(&[("key", &client.config.api_key)])
                .json(&request)
        })
        .await
    }

    /// Get the cache manager
    #[cfg(feature = "caching")]
    pub fn cache_manager(&self) -> &Arc<CacheManager> {
        &self.cache_manager
    }

    /// Get the configuration
    pub fn config(&self) -> &GeminiConfig {
        &self.config
    }

    /// Get the HTTP client
    pub fn http_client(&self) -> &HttpClient {
        &self.http_client
    }

    /// Build the HTTP client with configuration
    fn build_http_client(config: &GeminiConfig) -> Result<HttpClient> {
        let mut builder = HttpClient::builder()
            .timeout(config.http_config.timeout)
            .connect_timeout(config.http_config.connect_timeout);

        if config.http_config.pool_connections {
            builder = builder
                .pool_idle_timeout(Duration::from_secs(90))
                .pool_max_idle_per_host(config.http_config.pool_max_idle_per_host);
        }

        builder.build().map_err(Error::from)
    }

    /// Execute a request with retry logic
    async fn execute_with_retry<T, F>(&self, build_request: F) -> Result<T>
    where
        T: DeserializeOwned,
        F: Fn(&Self) -> RequestBuilder,
    {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.config.retry_config.max_attempts {
            attempts += 1;

            let request = build_request(self);
            let response = match request.send().await {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(Error::from(e));
                    if attempts < self.config.retry_config.max_attempts {
                        let delay = self.calculate_retry_delay(attempts);
                        warn!(
                            "Request failed (attempt {}), retrying in {:?}",
                            attempts, delay
                        );
                        sleep(delay).await;
                        continue;
                    }
                    break;
                }
            };

            let status = response.status();

            if status.is_success() {
                return response.json::<T>().await.map_err(Error::from);
            }

            let error_body = response.text().await.unwrap_or_default();
            let error = self.handle_api_error(status, error_body);

            if !error.is_retryable() || attempts >= self.config.retry_config.max_attempts {
                return Err(error);
            }

            last_error = Some(error);

            let delay = last_error
                .as_ref()
                .and_then(|e| e.retry_delay())
                .unwrap_or_else(|| self.calculate_retry_delay(attempts));

            warn!("API error (attempt {}), retrying in {:?}", attempts, delay);
            sleep(delay).await;
        }

        Err(last_error.unwrap_or_else(|| Error::Config("Max retry attempts exceeded".to_string())))
    }

    /// Calculate retry delay with exponential backoff
    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.retry_config.initial_delay.as_secs_f64();
        let multiplier = self.config.retry_config.backoff_multiplier;
        let max_delay = self.config.retry_config.max_delay;

        let delay = base_delay * multiplier.powi(attempt as i32 - 1);
        let delay = Duration::from_secs_f64(delay);

        let delay = std::cmp::min(delay, max_delay);

        if self.config.retry_config.jitter {
            // Add up to 25% jitter
            let jitter = rand::random::<f64>() * 0.25;
            let jittered = delay.as_secs_f64() * (1.0 + jitter);
            Duration::from_secs_f64(jittered)
        } else {
            delay
        }
    }

    /// Handle API errors
    fn handle_api_error(&self, status: StatusCode, body: String) -> Error {
        let details = serde_json::from_str::<serde_json::Value>(&body).ok();

        match status {
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = details
                    .as_ref()
                    .and_then(|d| d.get("retryAfter"))
                    .and_then(|v| v.as_u64())
                    .map(Duration::from_secs);

                Error::RateLimit { retry_after }
            }
            _ => Error::Api {
                status: status.as_u16(),
                message: details
                    .as_ref()
                    .and_then(|d| d.get("error"))
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or(&body)
                    .to_string(),
                details,
            },
        }
    }
}

/// Builder for creating a customized GeminiClient
#[derive(Default)]
pub struct GeminiClientBuilder {
    config: Option<GeminiConfig>,
}

impl GeminiClientBuilder {
    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        let mut config = self.config.unwrap_or_default();
        config.api_key = key.into();
        self.config = Some(config);
        self
    }

    /// Set the base URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        let mut config = self.config.unwrap_or_default();
        config.base_url = url.into();
        self.config = Some(config);
        self
    }

    /// Set the API version
    pub fn api_version(mut self, version: ApiVersion) -> Self {
        let mut config = self.config.unwrap_or_default();
        config.api_version = version;
        self.config = Some(config);
        self
    }

    /// Set the default model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        let mut config = self.config.unwrap_or_default();
        config.model_config.model = model.into();
        self.config = Some(config);
        self
    }

    /// Set request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        let mut config = self.config.unwrap_or_default();
        config.http_config.timeout = timeout;
        self.config = Some(config);
        self
    }

    /// Set retry configuration
    pub fn max_retries(mut self, retries: u32) -> Self {
        let mut config = self.config.unwrap_or_default();
        config.retry_config.max_attempts = retries;
        self.config = Some(config);
        self
    }

    /// Build the client
    pub fn build(self) -> Result<GeminiClient> {
        let config = self
            .config
            .ok_or_else(|| Error::Config("Configuration not properly initialized".to_string()))?;

        if config.api_key.is_empty() {
            return Err(Error::Config("API key is required".to_string()));
        }

        GeminiClient::new(config)
    }
}
