//! Thinking mode configuration for Gemini 2.5 models

use serde::{Deserialize, Serialize};

/// Configuration for thinking mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    /// Number of thinking tokens the model can use (0-24576)
    pub thinking_budget: ThinkingBudget,
}

/// Thinking budget specification
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThinkingBudget {
    /// Exact number of tokens
    Tokens(u32),
    /// Let the model decide based on complexity
    Auto,
}

impl ThinkingConfig {
    /// Create a new thinking configuration with a specific token budget
    pub fn with_budget(tokens: u32) -> Self {
        assert!(
            tokens <= 24576,
            "Thinking budget cannot exceed 24576 tokens"
        );
        Self {
            thinking_budget: ThinkingBudget::Tokens(tokens),
        }
    }

    /// Create a configuration that lets the model decide thinking budget
    pub fn auto() -> Self {
        Self {
            thinking_budget: ThinkingBudget::Auto,
        }
    }

    /// Disable thinking mode
    pub fn disabled() -> Self {
        Self {
            thinking_budget: ThinkingBudget::Tokens(0),
        }
    }
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self::auto()
    }
}

/// Extension trait for GenerationConfig to easily set thinking mode
pub trait ThinkingExt {
    /// Apply thinking configuration to generation config
    fn with_thinking(self, config: ThinkingConfig) -> Self;
    /// Set a specific thinking budget in tokens
    fn with_thinking_budget(self, tokens: u32) -> Self;
    /// Enable auto thinking mode
    fn with_auto_thinking(self) -> Self;
    /// Disable thinking mode
    fn without_thinking(self) -> Self;
}

impl ThinkingExt for crate::models::GenerationConfig {
    /// Apply thinking configuration to generation config
    fn with_thinking(mut self, config: ThinkingConfig) -> Self {
        self.thinking_config = Some(config);
        self
    }

    /// Set a specific thinking budget in tokens
    fn with_thinking_budget(self, tokens: u32) -> Self {
        self.with_thinking(ThinkingConfig::with_budget(tokens))
    }

    /// Enable auto thinking mode
    fn with_auto_thinking(self) -> Self {
        self.with_thinking(ThinkingConfig::auto())
    }

    /// Disable thinking mode
    fn without_thinking(self) -> Self {
        self.with_thinking(ThinkingConfig::disabled())
    }
}

/// Helper to determine appropriate thinking budget based on task complexity
pub struct ThinkingBudgetCalculator;

impl ThinkingBudgetCalculator {
    /// Estimate thinking budget based on prompt characteristics
    pub fn estimate(prompt: &str, task_type: TaskComplexity) -> u32 {
        let base_budget = match task_type {
            TaskComplexity::Simple => 0,
            TaskComplexity::Moderate => 512,
            TaskComplexity::Complex => 2048,
            TaskComplexity::VeryComplex => 8192,
        };

        // Adjust based on prompt length and complexity indicators
        let prompt_words = prompt.split_whitespace().count();
        let complexity_multiplier = if prompt.contains("step by step")
            || prompt.contains("analyze")
            || prompt.contains("explain")
        {
            1.5
        } else {
            1.0
        };

        let adjusted = (base_budget as f64 * complexity_multiplier) as u32;

        // Add more budget for longer prompts
        let length_bonus = (prompt_words / 100) as u32 * 256;

        (adjusted + length_bonus).min(24576)
    }
}

/// Task complexity levels for thinking budget estimation
#[derive(Debug, Clone, Copy)]
pub enum TaskComplexity {
    /// Simple queries, fact retrieval
    Simple,
    /// Moderate reasoning, comparisons
    Moderate,
    /// Complex analysis, multi-step reasoning
    Complex,
    /// Very complex problems, deep analysis
    VeryComplex,
}
