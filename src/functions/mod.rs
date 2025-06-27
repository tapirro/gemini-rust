//! Function calling support for Gemini API

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Tool {
    /// Function declarations
    FunctionDeclarations {
        /// List of function declarations available to the model
        #[serde(rename = "functionDeclarations")]
        function_declarations: Vec<FunctionDeclaration>,
    },
    /// Google Search tool
    #[cfg(feature = "grounding")]
    GoogleSearch(crate::grounding::SearchGrounding),
    /// URL Context tool
    #[cfg(feature = "grounding")]
    UrlContext(crate::grounding::UrlContext),
    /// Code execution tool
    CodeExecution {
        /// Configuration for code execution
        #[serde(rename = "codeExecution")]
        code_execution: CodeExecutionConfig,
    },
}

/// Function declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    /// Function name
    pub name: String,

    /// Function description
    pub description: String,

    /// Parameters schema (OpenAPI format)
    pub parameters: ParameterSchema,
}

/// Parameter schema for functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    /// Schema type (usually "object")
    #[serde(rename = "type")]
    pub schema_type: String, // Usually "object"

    /// Properties definition
    pub properties: HashMap<String, PropertySchema>,

    /// Required parameter names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Individual property schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    /// Type of the property
    #[serde(rename = "type")]
    pub property_type: String,

    /// Description of the property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Allowed enum values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,

    /// Schema for array items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<PropertySchema>>,
}

/// Function call from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Name of the function to call
    pub name: String,
    /// Arguments to pass to the function
    pub args: HashMap<String, serde_json::Value>,
}

/// Function response to send back to the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    /// Name of the function that was called
    pub name: String,
    /// Response data from the function
    pub response: serde_json::Value,
}

/// Code execution configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeExecutionConfig {}

/// Tool configuration for controlling function calling behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    /// Function calling configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_calling_config: Option<FunctionCallingConfig>,
}

/// Function calling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallingConfig {
    /// Mode for function calling
    pub mode: FunctionCallingMode,

    /// Allowed function names (for restricting which functions can be called)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
}

/// Function calling mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionCallingMode {
    /// Model decides whether to call functions
    Auto,
    /// Model must call a function
    Any,
    /// Model cannot call functions
    None,
}

/// Builder for function declarations
pub struct FunctionBuilder {
    name: String,
    description: String,
    parameters: HashMap<String, PropertySchema>,
    required: Vec<String>,
}

impl FunctionBuilder {
    /// Create a new function builder with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            parameters: HashMap::new(),
            required: Vec::new(),
        }
    }

    /// Set the description of the function
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a parameter to the function
    pub fn param(
        mut self,
        name: impl Into<String>,
        param_type: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let name = name.into();

        self.parameters.insert(
            name.clone(),
            PropertySchema {
                property_type: param_type.into(),
                description: Some(description.into()),
                enum_values: None,
                items: None,
            },
        );

        if required {
            self.required.push(name);
        }

        self
    }

    /// Add an enum parameter to the function
    pub fn enum_param(
        mut self,
        name: impl Into<String>,
        values: Vec<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let name = name.into();

        self.parameters.insert(
            name.clone(),
            PropertySchema {
                property_type: "string".to_string(),
                description: Some(description.into()),
                enum_values: Some(values),
                items: None,
            },
        );

        if required {
            self.required.push(name);
        }

        self
    }

    /// Build the function declaration
    pub fn build(self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: self.name,
            description: self.description,
            parameters: ParameterSchema {
                schema_type: "object".to_string(),
                properties: self.parameters,
                required: if self.required.is_empty() {
                    None
                } else {
                    Some(self.required)
                },
            },
        }
    }
}

/// Helper to create function tools
impl Tool {
    /// Create a tool with function declarations
    pub fn functions(declarations: Vec<FunctionDeclaration>) -> Self {
        Tool::FunctionDeclarations {
            function_declarations: declarations,
        }
    }

    /// Create a Google Search tool
    #[cfg(feature = "grounding")]
    pub fn google_search() -> Self {
        Tool::GoogleSearch(crate::grounding::SearchGrounding::default())
    }

    /// Create a URL context tool
    #[cfg(feature = "grounding")]
    pub fn url_context() -> Self {
        Tool::UrlContext(crate::grounding::UrlContext::default())
    }

    /// Create a code execution tool
    pub fn code_execution() -> Self {
        Tool::CodeExecution {
            code_execution: CodeExecutionConfig::default(),
        }
    }
}

/// Extension trait for easy tool configuration
pub trait ToolExt {
    /// Configure automatic function calling
    fn with_auto_function_calling(self) -> Self;
    /// Configure any function calling with optional allowed functions
    fn with_any_function_calling(self, allowed: Option<Vec<String>>) -> Self;
    /// Disable function calling
    fn without_function_calling(self) -> Self;
}

impl ToolExt for crate::models::GenerateContentRequest {
    /// Configure automatic function calling
    fn with_auto_function_calling(mut self) -> Self {
        self.tool_config = Some(ToolConfig {
            function_calling_config: Some(FunctionCallingConfig {
                mode: FunctionCallingMode::Auto,
                allowed_function_names: None,
            }),
        });
        self
    }

    /// Configure any function calling with optional allowed functions
    fn with_any_function_calling(mut self, allowed: Option<Vec<String>>) -> Self {
        self.tool_config = Some(ToolConfig {
            function_calling_config: Some(FunctionCallingConfig {
                mode: FunctionCallingMode::Any,
                allowed_function_names: allowed,
            }),
        });
        self
    }

    /// Disable function calling
    fn without_function_calling(mut self) -> Self {
        self.tool_config = Some(ToolConfig {
            function_calling_config: Some(FunctionCallingConfig {
                mode: FunctionCallingMode::None,
                allowed_function_names: None,
            }),
        });
        self
    }
}
