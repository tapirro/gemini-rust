//! Error handling for the Gemini API integration

use std::time::Duration;
use thiserror::Error;

/// Result type alias for library operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for the Gemini API client
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP request error
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("JSON serialization/deserialization failed: {0}")]
    Json(#[from] serde_json::Error),

    /// API error response
    #[error("API error (status: {status}): {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
        /// Additional error details
        details: Option<serde_json::Value>,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Retry after {retry_after:?}")]
    RateLimit {
        /// Suggested retry delay
        retry_after: Option<Duration>,
    },

    /// Configuration error
    #[error("Invalid configuration: {0}")]
    Config(String),

    /// Schema validation error
    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),

    /// Function call error
    #[error("Function call failed: {0}")]
    FunctionCall(String),

    /// Grounding operation error
    #[error("Grounding failed: {0}")]
    Grounding(String),

    /// Cache operation error
    #[error("Cache operation failed: {0}")]
    Cache(String),

    /// Streaming operation error
    #[error("Streaming error: {0}")]
    Streaming(String),

    /// Operation timeout
    #[error("Timeout after {0:?}")]
    Timeout(Duration),

    /// Invalid response format
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    /// Thinking budget exceeded
    #[error("Thinking budget exceeded")]
    ThinkingBudgetExceeded,
}

impl Error {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Http(_)
                | Error::RateLimit { .. }
                | Error::Timeout(_)
                | Error::Api {
                    status: 500..=599,
                    ..
                }
        )
    }

    /// Get retry delay if applicable
    pub fn retry_delay(&self) -> Option<Duration> {
        match self {
            Error::RateLimit { retry_after } => *retry_after,
            Error::Api { status: 429, .. } => Some(Duration::from_secs(60)),
            Error::Api {
                status: 500..=599, ..
            } => Some(Duration::from_secs(5)),
            _ => None,
        }
    }
}
