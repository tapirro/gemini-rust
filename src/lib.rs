//! # gemini-rust
//!
//! A comprehensive Rust client for Google's Gemini API.
//!
//! ## Features
//!
//! - **Structured Output**: Type-safe JSON schema generation
//! - **Thinking Mode**: Support for Gemini 2.5's reasoning capabilities
//! - **Function Calling**: Parallel and compositional function support
//! - **Grounding**: Google Search and URL context integration
//! - **Context Caching**: Efficient token management
//! - **Streaming**: Real-time response streaming
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use gemini_rust::{GeminiClient, Content, GenerateContentRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client from environment variable GEMINI_API_KEY
//!     let client = GeminiClient::from_env()?;
//!     
//!     // Generate content
//!     let request = GenerateContentRequest {
//!         contents: vec![Content::user("Hello, Gemini!")],
//!         ..Default::default()
//!     };
//!     
//!     let response = client.generate_content(None, request).await?;
//!     println!("{:?}", response);
//!     
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod client;
pub mod config;
pub mod error;
pub mod models;

#[cfg(feature = "grounding")]
#[cfg_attr(docsrs, doc(cfg(feature = "grounding")))]
pub mod grounding;

#[cfg(feature = "caching")]
#[cfg_attr(docsrs, doc(cfg(feature = "caching")))]
pub mod cache;

#[cfg(feature = "functions")]
#[cfg_attr(docsrs, doc(cfg(feature = "functions")))]
pub mod functions;

#[cfg(feature = "thinking")]
#[cfg_attr(docsrs, doc(cfg(feature = "thinking")))]
pub mod thinking;

#[cfg(feature = "streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "streaming")))]
pub mod streaming;

// Re-export main types
pub use client::{GeminiClient, GeminiClientBuilder};
pub use config::{ApiVersion, GeminiConfig, ModelConfig};
pub use error::{Error, Result};
pub use models::*;

#[cfg(feature = "grounding")]
pub use grounding::{GroundingBuilder, GroundingConfig, SearchGrounding, UrlContext};

#[cfg(feature = "caching")]
pub use cache::{CacheConfig, CacheManager, CachedContent};

#[cfg(feature = "functions")]
pub use functions::{FunctionBuilder, FunctionCall, FunctionDeclaration, FunctionResponse, Tool};

#[cfg(feature = "thinking")]
pub use thinking::{ThinkingBudget, ThinkingConfig, ThinkingExt};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Content, GeminiClient, GeminiClientBuilder, GenerateContentRequest,
        GenerateContentResponse, GenerationConfig, Part, ResponseSchema, Result, Role, SchemaType,
    };

    #[cfg(feature = "grounding")]
    pub use crate::grounding::GroundingBuilder;

    #[cfg(feature = "functions")]
    pub use crate::functions::{FunctionBuilder, Tool};

    #[cfg(feature = "thinking")]
    pub use crate::thinking::ThinkingExt;
}
