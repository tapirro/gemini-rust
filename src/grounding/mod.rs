//! Grounding support for search and URL context

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for grounding tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GroundingConfig {
    /// Google Search grounding
    Search(SearchGrounding),
    /// URL context grounding
    UrlContext(UrlContext),
    /// Both search and URL context
    Combined {
        /// Google Search grounding configuration
        search: SearchGrounding,
        /// URL context configuration
        url_context: UrlContext,
    },
}

/// Google Search grounding configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchGrounding {
    /// Dynamic retrieval configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_retrieval_config: Option<DynamicRetrievalConfig>,
}

/// URL context configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UrlContext {
    /// Maximum number of URLs to process (default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_urls: Option<u32>,
}

/// Dynamic retrieval configuration for search grounding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicRetrievalConfig {
    /// Mode for dynamic retrieval
    pub mode: DynamicRetrievalMode,

    /// Threshold for dynamic retrieval (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_threshold: Option<f32>,
}

/// Mode for dynamic retrieval behavior
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DynamicRetrievalMode {
    /// Always use grounding
    ModeUnspecified,
    /// Dynamically decide based on threshold
    ModeDynamic,
}

/// Metadata returned with grounded responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingMetadata {
    /// Search queries used for grounding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_queries: Option<Vec<String>>,

    /// Search entry point for rendering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_entry_point: Option<SearchEntryPoint>,

    /// Grounding chunks found
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_chunks: Option<Vec<GroundingChunk>>,

    /// Grounding support segments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_supports: Option<Vec<GroundingSupport>>,

    /// Retrieval metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Search entry point for rendering search suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEntryPoint {
    /// Rendered content for search suggestions
    pub rendered_content: String,
}

/// A chunk of grounding information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingChunk {
    /// Web source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web: Option<WebSource>,
}

/// Web source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSource {
    /// URI of the web source
    pub uri: String,
    /// Title of the web source
    pub title: String,
    /// Domain of the web source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

/// Grounding support information for text segments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingSupport {
    /// Text segment that was grounded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment: Option<TextSegment>,

    /// Indices of grounding chunks that support this segment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_chunk_indices: Option<Vec<i32>>,

    /// Confidence scores for the grounding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_scores: Option<Vec<f32>>,
}

/// A segment of text that was grounded
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSegment {
    /// Starting index of the text segment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<i32>,

    /// Ending index of the text segment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_index: Option<i32>,

    /// The text content of the segment
    pub text: String,
}

/// URL context metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlContextMetadata {
    /// Metadata about URLs that were processed
    pub url_metadata: Vec<UrlMetadata>,
}

/// Metadata about a processed URL
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlMetadata {
    /// The URL that was retrieved
    pub retrieved_url: String,

    /// Status of URL retrieval
    pub url_retrieval_status: UrlRetrievalStatus,
}

/// Status of URL retrieval for grounding
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UrlRetrievalStatus {
    /// URL was successfully retrieved
    #[serde(rename = "URL_RETRIEVAL_STATUS_SUCCESS")]
    Success,
    /// Error occurred during URL retrieval
    #[serde(rename = "URL_RETRIEVAL_STATUS_ERROR")]
    Error,
    /// URL was unreachable
    #[serde(rename = "URL_RETRIEVAL_STATUS_UNREACHABLE")]
    Unreachable,
}

/// Helper to convert grounding config into tools
impl GroundingConfig {
    /// Convert grounding configuration to tools vector
    #[cfg(feature = "functions")]
    pub fn to_tools(&self) -> Vec<crate::functions::Tool> {
        match self {
            GroundingConfig::Search(search) => {
                vec![crate::functions::Tool::GoogleSearch(search.clone())]
            }
            GroundingConfig::UrlContext(url_context) => {
                vec![crate::functions::Tool::UrlContext(url_context.clone())]
            }
            GroundingConfig::Combined {
                search,
                url_context,
            } => vec![
                crate::functions::Tool::GoogleSearch(search.clone()),
                crate::functions::Tool::UrlContext(url_context.clone()),
            ],
        }
    }
}

/// Builder for grounding configuration
pub struct GroundingBuilder {
    search: Option<SearchGrounding>,
    url_context: Option<UrlContext>,
}

impl Default for GroundingBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundingBuilder {
    /// Create a new grounding builder
    pub fn new() -> Self {
        Self {
            search: None,
            url_context: None,
        }
    }

    /// Enable Google Search grounding
    pub fn with_search(mut self) -> Self {
        self.search = Some(SearchGrounding::default());
        self
    }

    /// Enable Google Search with dynamic retrieval
    pub fn with_dynamic_search(mut self, threshold: f32) -> Self {
        self.search = Some(SearchGrounding {
            dynamic_retrieval_config: Some(DynamicRetrievalConfig {
                mode: DynamicRetrievalMode::ModeDynamic,
                dynamic_threshold: Some(threshold),
            }),
        });
        self
    }

    /// Enable URL context
    pub fn with_url_context(mut self) -> Self {
        self.url_context = Some(UrlContext::default());
        self
    }

    /// Set maximum URLs for context
    pub fn max_urls(mut self, max: u32) -> Self {
        if let Some(ref mut ctx) = self.url_context {
            ctx.max_urls = Some(max);
        }
        self
    }

    /// Build the grounding configuration
    pub fn build(self) -> Option<GroundingConfig> {
        match (self.search, self.url_context) {
            (Some(search), Some(url_context)) => Some(GroundingConfig::Combined {
                search,
                url_context,
            }),
            (Some(search), None) => Some(GroundingConfig::Search(search)),
            (None, Some(url_context)) => Some(GroundingConfig::UrlContext(url_context)),
            (None, None) => None,
        }
    }
}
