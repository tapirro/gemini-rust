//! Core data models for the Gemini API

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Role in a conversation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User/human input
    User,
    /// Model/AI response
    Model,
    /// System instruction
    System,
}

/// Content part types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    /// Text content part
    Text {
        /// Text content as a string
        text: String,
    },
    /// Inline data part (base64 encoded)
    InlineData {
        /// Inline data with base64 encoded content
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },
    /// File data part (file URI reference)
    FileData {
        /// File data with URI reference
        #[serde(rename = "fileData")]
        file_data: FileData,
    },
    /// Function call part
    #[cfg(feature = "functions")]
    FunctionCall {
        /// Function call data
        #[serde(rename = "functionCall")]
        function_call: crate::functions::FunctionCall,
    },
    /// Function response part
    #[cfg(feature = "functions")]
    FunctionResponse {
        /// Function response data
        #[serde(rename = "functionResponse")]
        function_response: crate::functions::FunctionResponse,
    },
}

/// Inline data with base64 encoded content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineData {
    /// MIME type of the data
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Base64 encoded data
    pub data: String, // Base64 encoded
}

/// File data with URI reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileData {
    /// MIME type of the file
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// URI reference to the file
    #[serde(rename = "fileUri")]
    pub file_uri: String,
}

/// Content in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Role of the content creator
    pub role: Role,
    /// Parts that make up the content
    pub parts: Vec<Part>,
}

impl Content {
    /// Create user content with text
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            parts: vec![Part::Text { text: text.into() }],
        }
    }

    /// Create model content with text
    pub fn model(text: impl Into<String>) -> Self {
        Self {
            role: Role::Model,
            parts: vec![Part::Text { text: text.into() }],
        }
    }

    /// Create system content with text
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            parts: vec![Part::Text { text: text.into() }],
        }
    }
}

/// Generation configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    /// Controls randomness in output (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Nucleus sampling parameter (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Top-k sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,

    /// Number of response candidates to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<i32>,

    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,

    /// Sequences that will stop generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// MIME type for the response format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,

    /// Schema for structured output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<ResponseSchema>,

    /// Penalty for repeated presence (-2.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Penalty for repeated frequency (-2.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Whether to return log probabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_logprobs: Option<bool>,

    /// Number of top logprobs to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<i32>,

    /// Configuration for thinking/reasoning behavior
    #[cfg(feature = "thinking")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<crate::thinking::ThinkingConfig>,
}

/// Response schema for structured output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSchema {
    /// The type of this schema
    #[serde(rename = "type")]
    pub schema_type: SchemaType,

    /// Format constraint for the schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// Description of this schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this field can be null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,

    /// Allowed enum values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,

    /// Properties for object types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, ResponseSchema>>,

    /// Required property names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    /// Ordering of properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_ordering: Option<Vec<String>>,

    /// Schema for array items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<ResponseSchema>>,

    /// Minimum number of array items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<i32>,

    /// Maximum number of array items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<i32>,
}

/// JSON schema data types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    /// String type
    String,
    /// Integer type
    Integer,
    /// Number type (float)
    Number,
    /// Boolean type
    Boolean,
    /// Array type
    Array,
    /// Object type
    Object,
}

/// Safety settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySetting {
    /// Category of harmful content
    pub category: HarmCategory,
    /// Threshold for blocking content
    pub threshold: HarmBlockThreshold,
}

/// Categories of harmful content
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HarmCategory {
    /// Hate speech content
    #[serde(rename = "HARM_CATEGORY_HATE_SPEECH")]
    HateSpeech,
    /// Dangerous content
    #[serde(rename = "HARM_CATEGORY_DANGEROUS_CONTENT")]
    DangerousContent,
    /// Sexually explicit content
    #[serde(rename = "HARM_CATEGORY_SEXUALLY_EXPLICIT")]
    SexuallyExplicit,
    /// Harassment content
    #[serde(rename = "HARM_CATEGORY_HARASSMENT")]
    Harassment,
}

/// Thresholds for blocking harmful content
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HarmBlockThreshold {
    /// Block no content
    #[serde(rename = "BLOCK_NONE")]
    BlockNone,
    /// Block only high-probability harmful content
    #[serde(rename = "BLOCK_ONLY_HIGH")]
    BlockOnlyHigh,
    /// Block medium and high-probability harmful content
    #[serde(rename = "BLOCK_MEDIUM_AND_ABOVE")]
    BlockMediumAndAbove,
    /// Block low, medium, and high-probability harmful content
    #[serde(rename = "BLOCK_LOW_AND_ABOVE")]
    BlockLowAndAbove,
}

/// Main request structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    /// Input content for generation
    pub contents: Vec<Content>,

    /// System instruction for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,

    /// Available tools for function calling
    #[cfg(feature = "functions")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::functions::Tool>>,

    /// Configuration for tool usage
    #[cfg(feature = "functions")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<crate::functions::ToolConfig>,

    /// Safety settings for content filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,

    /// Configuration for generation behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,

    /// Reference to cached content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,
}

/// Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    /// Generated response candidates
    pub candidates: Vec<Candidate>,

    /// Feedback about the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_feedback: Option<PromptFeedback>,

    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// A response candidate
pub struct Candidate {
    /// Generated content
    pub content: Content,

    /// Reason for finishing generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,

    /// Safety ratings for the content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,

    /// Citation information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_metadata: Option<CitationMetadata>,

    /// Grounding metadata for search results
    #[cfg(feature = "grounding")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_metadata: Option<crate::grounding::GroundingMetadata>,

    /// URL context metadata
    #[cfg(feature = "grounding")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_context_metadata: Option<crate::grounding::UrlContextMetadata>,
}

/// Reasons for finishing content generation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FinishReason {
    /// Natural stopping point
    #[serde(rename = "STOP")]
    Stop,
    /// Reached maximum token limit
    #[serde(rename = "MAX_TOKENS")]
    MaxTokens,
    /// Stopped due to safety concerns
    #[serde(rename = "SAFETY")]
    Safety,
    /// Stopped due to recitation concerns
    #[serde(rename = "RECITATION")]
    Recitation,
    /// Other reason
    #[serde(rename = "OTHER")]
    Other,
}

/// Feedback about the prompt before generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptFeedback {
    /// Reason for blocking the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<BlockReason>,

    /// Safety ratings for the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

/// Reasons why content was blocked
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BlockReason {
    /// Unspecified reason
    #[serde(rename = "BLOCKED_REASON_UNSPECIFIED")]
    Unspecified,
    /// Safety violation
    #[serde(rename = "SAFETY")]
    Safety,
    /// Other reason
    #[serde(rename = "OTHER")]
    Other,
}

/// Safety rating for content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRating {
    /// Category of potential harm
    pub category: HarmCategory,
    /// Probability of harm
    pub probability: HarmProbability,
}

/// Probability levels for harmful content
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HarmProbability {
    /// Negligible probability
    #[serde(rename = "NEGLIGIBLE")]
    Negligible,
    /// Low probability
    #[serde(rename = "LOW")]
    Low,
    /// Medium probability
    #[serde(rename = "MEDIUM")]
    Medium,
    /// High probability
    #[serde(rename = "HIGH")]
    High,
}

/// Token usage metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    /// Number of tokens in the prompt
    pub prompt_token_count: i32,
    /// Number of tokens in the candidates
    pub candidates_token_count: i32,
    /// Total number of tokens used
    pub total_token_count: i32,

    /// Number of tokens from cached content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content_token_count: Option<i32>,
}

/// Citation metadata for generated content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationMetadata {
    /// List of citation sources
    pub citation_sources: Vec<CitationSource>,
}

/// Source of a citation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationSource {
    /// Starting index of the citation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<i32>,

    /// Ending index of the citation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_index: Option<i32>,

    /// URI of the source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,

    /// License information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// Request for counting tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct CountTokensRequest {
    /// Content to count tokens for
    pub contents: Vec<Content>,
}

/// Response from token counting API
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountTokensResponse {
    /// Total number of tokens in the provided content
    pub total_tokens: i32,
}

/// Builder for structured output
pub struct StructuredOutput;

impl StructuredOutput {
    /// Create a JSON schema for structured output
    pub fn json_schema() -> ResponseSchema {
        ResponseSchema {
            schema_type: SchemaType::Object,
            format: None,
            description: None,
            nullable: None,
            enum_values: None,
            properties: Some(HashMap::new()),
            required: None,
            property_ordering: None,
            items: None,
            min_items: None,
            max_items: None,
        }
    }

    /// Create an enum schema with allowed values
    pub fn enum_schema(values: Vec<String>) -> ResponseSchema {
        ResponseSchema {
            schema_type: SchemaType::String,
            format: None,
            description: None,
            nullable: None,
            enum_values: Some(values),
            properties: None,
            required: None,
            property_ordering: None,
            items: None,
            min_items: None,
            max_items: None,
        }
    }
}
