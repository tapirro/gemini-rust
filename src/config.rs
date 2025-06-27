//! Configuration for the Gemini API client

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for the Gemini API client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    /// API key for authentication
    pub api_key: String,

    /// Base URL for the API (can be overridden for testing)
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// API version to use
    #[serde(default)]
    pub api_version: ApiVersion,

    /// HTTP client configuration
    #[serde(default)]
    pub http_config: HttpConfig,

    /// Retry configuration
    #[serde(default)]
    pub retry_config: RetryConfig,

    /// Default model configuration
    #[serde(default)]
    pub model_config: ModelConfig,
}

/// API version to use for requests
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum ApiVersion {
    /// Stable v1 API
    #[default]
    #[serde(rename = "v1")]
    V1,
    /// Beta v1 API with experimental features
    #[serde(rename = "v1beta")]
    V1Beta,
}

impl ApiVersion {
    /// Convert the API version to a string
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiVersion::V1 => "v1",
            ApiVersion::V1Beta => "v1beta",
        }
    }
}

/// HTTP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Request timeout
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Connection timeout
    #[serde(with = "humantime_serde")]
    pub connect_timeout: Duration,

    /// Whether to use connection pooling
    pub pool_connections: bool,

    /// Maximum idle connections per host
    pub pool_max_idle_per_host: usize,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(300), // 5 minutes for large context
            connect_timeout: Duration::from_secs(30),
            pool_connections: true,
            pool_max_idle_per_host: 10,
        }
    }
}

/// Retry configuration for failed requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial retry delay
    #[serde(with = "humantime_serde")]
    pub initial_delay: Duration,

    /// Maximum retry delay
    #[serde(with = "humantime_serde")]
    pub max_delay: Duration,

    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,

    /// Add jitter to retry delays
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Model configuration for default behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Default model to use
    #[serde(default = "default_model")]
    pub model: String,

    /// Whether to use the latest version suffix
    #[serde(default = "default_use_latest")]
    pub use_latest: bool,

    /// Model-specific parameters
    #[serde(flatten)]
    pub params: serde_json::Value,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            use_latest: default_use_latest(),
            params: serde_json::Value::Object(Default::default()),
        }
    }
}

fn default_base_url() -> String {
    "https://generativelanguage.googleapis.com".to_string()
}

fn default_model() -> String {
    "gemini-2.5-flash".to_string()
}

fn default_use_latest() -> bool {
    true
}

impl GeminiConfig {
    /// Create a new configuration with an API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            ..Default::default()
        }
    }

    /// Load configuration from environment variables
    pub fn from_env() -> crate::error::Result<Self> {
        let api_key = std::env::var("GEMINI_API_KEY").map_err(|_| {
            crate::error::Error::Config("GEMINI_API_KEY environment variable not set".to_string())
        })?;

        Ok(Self::new(api_key))
    }

    /// Get the full model name with version suffix if needed
    pub fn get_model_name(&self, model: Option<&str>) -> String {
        let base_model = model.unwrap_or(&self.model_config.model);

        if self.model_config.use_latest && !base_model.contains("-preview") {
            // Add appropriate suffix based on model type
            match base_model {
                "gemini-2.5-pro" => "gemini-2.5-pro-preview-05-06".to_string(),
                "gemini-2.5-flash" => "gemini-2.5-flash".to_string(),
                "gemini-2.0-flash" => "gemini-2.0-flash-001".to_string(),
                _ => base_model.to_string(),
            }
        } else {
            base_model.to_string()
        }
    }
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: default_base_url(),
            api_version: ApiVersion::default(),
            http_config: HttpConfig::default(),
            retry_config: RetryConfig::default(),
            model_config: ModelConfig::default(),
        }
    }
}
