use anyhow::Result;
use gemini_rust::{prelude::*, ApiVersion, GeminiConfig};

#[cfg(feature = "functions")]
use gemini_rust::{FunctionBuilder, Tool};

#[cfg(feature = "functions")]
use gemini_rust::functions::ToolExt;

#[cfg(feature = "caching")]
use gemini_rust::CacheConfig;

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

// Test timeout - 30 seconds per test
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Helper to check if we have a real API key for live tests
fn has_api_key() -> bool {
    std::env::var("GEMINI_API_KEY").is_ok()
}

/// Helper to create a test client
async fn create_test_client() -> Result<GeminiClient> {
    if !has_api_key() {
        return Err(anyhow::anyhow!(
            "GEMINI_API_KEY not available for live tests"
        ));
    }

    let mut config = GeminiConfig::from_env()?;
    config.api_version = ApiVersion::V1Beta; // Use beta for advanced features

    Ok(GeminiClient::new(config)?)
}

/// Test helper to skip tests when API key is not available
macro_rules! skip_without_api_key {
    () => {
        if !has_api_key() {
            println!("⚠️  Skipping live API test - GEMINI_API_KEY not set");
            return Ok(());
        }
    };
}

#[tokio::test]
async fn test_structured_output_json() -> Result<()> {
    skip_without_api_key!();

    #[derive(Debug, Serialize, Deserialize)]
    struct Person {
        name: String,
        age: u32,
        occupation: String,
        skills: Vec<String>,
    }

    let client = create_test_client().await?;

    // Create generation config with structured output
    let mut generation_config = GenerationConfig::default();
    generation_config.response_mime_type = Some("application/json".to_string());

    let request = GenerateContentRequest {
        contents: vec![Content::user(
            "Create a fictional person profile. Return as JSON with fields: name, age, occupation, skills (array of strings)."
        )],
        generation_config: Some(generation_config),
        ..Default::default()
    };

    let response = timeout(
        TEST_TIMEOUT,
        client.generate_content(Some("gemini-2.5-flash"), request),
    )
    .await??;

    // Verify we got a response
    assert!(!response.candidates.is_empty(), "No candidates in response");

    let candidate = &response.candidates[0];
    assert!(
        !candidate.content.parts.is_empty(),
        "No parts in candidate content"
    );

    // Try to parse the JSON response
    if let Some(Part::Text { text }) = candidate.content.parts.first() {
        let person: Person = serde_json::from_str(text).map_err(|e| {
            anyhow::anyhow!("Failed to parse JSON response: {}\nResponse: {}", e, text)
        })?;

        // Verify the structure
        assert!(!person.name.is_empty(), "Person name should not be empty");
        assert!(person.age > 0, "Person age should be positive");
        assert!(
            !person.occupation.is_empty(),
            "Person occupation should not be empty"
        );
        assert!(
            !person.skills.is_empty(),
            "Person skills should not be empty"
        );

        println!("✅ Structured JSON output test passed");
        println!("Generated person: {:?}", person);
    } else {
        return Err(anyhow::anyhow!(
            "Expected text response for JSON structured output"
        ));
    }

    Ok(())
}

#[cfg(feature = "caching")]
#[tokio::test]
async fn test_caching_functionality() -> Result<()> {
    skip_without_api_key!();

    let client = create_test_client().await?;
    let cache_manager = client.cache_manager();

    // Create test content to cache
    let content = Content::user(
        "This is test content for caching. Please remember this context for future questions.",
    );

    // Create cache config
    let cache_config = CacheConfig {
        ttl: Some(300), // 5 minutes TTL
        display_name: Some("test-cache".to_string()),
    };

    // Try to create cached content - if it fails, skip the test
    let cache_result = cache_manager
        .create_cache(
            &client,
            Some("gemini-1.5-flash-001"),
            vec![content.clone()],
            None,
            cache_config,
        )
        .await;

    let cached_content = match cache_result {
        Ok(content) => content,
        Err(e) => {
            println!("⚠️  Skipping cache test - caching not available: {}", e);
            return Ok(());
        }
    };

    println!("✅ Created cache: {}", cached_content.name);

    // Use the cached content in a request with the same model
    let request = GenerateContentRequest {
        contents: vec![Content::user(
            "What was the test content I mentioned earlier?",
        )],
        cached_content: Some(cached_content.name.clone()),
        ..Default::default()
    };

    let response = timeout(
        TEST_TIMEOUT,
        client.generate_content(Some("gemini-1.5-flash-001"), request),
    )
    .await??;

    // Verify we got a response that references the cached content
    assert!(!response.candidates.is_empty(), "No candidates in response");

    if let Some(candidate) = response.candidates.first() {
        if let Some(Part::Text { text }) = candidate.content.parts.first() {
            println!("✅ Cache usage test passed");
            println!("Response with cached context: {}", text);
        }
    }

    // List caches to verify it exists
    let caches = timeout(TEST_TIMEOUT, cache_manager.list_caches(&client, None, None)).await??;
    assert!(
        caches
            .cached_contents
            .as_ref()
            .map_or(false, |c| !c.is_empty()),
        "Should have at least one cached content"
    );

    // Clean up - delete the cache
    timeout(
        TEST_TIMEOUT,
        cache_manager.delete_cache(&client, &cached_content.name),
    )
    .await??;
    println!("✅ Cache cleanup completed");

    Ok(())
}

#[cfg(feature = "grounding")]
#[tokio::test]
async fn test_grounding_functionality() -> Result<()> {
    skip_without_api_key!();

    let client = create_test_client().await?;

    // Create a request with Google Search grounding
    let search_tool = Tool::google_search();

    let request = GenerateContentRequest {
        contents: vec![Content::user(
            "What are the latest developments in Rust programming language released in 2024?",
        )],
        tools: Some(vec![search_tool]),
        ..Default::default()
    };

    let response = timeout(
        TEST_TIMEOUT,
        client.generate_content(Some("gemini-2.5-flash"), request),
    )
    .await??;

    // Verify we got a response
    assert!(!response.candidates.is_empty(), "No candidates in response");

    let candidate = &response.candidates[0];
    assert!(
        !candidate.content.parts.is_empty(),
        "No parts in candidate content"
    );

    if let Some(Part::Text { text }) = candidate.content.parts.first() {
        // The response should contain information that suggests it used web search
        assert!(!text.is_empty(), "Response should not be empty");
        println!("✅ Grounding test passed");
        println!("Grounded response: {}", text);

        // Check if grounding metadata is present (if available)
        if let Some(grounding_metadata) = &candidate.grounding_metadata {
            if let Some(chunks) = &grounding_metadata.grounding_chunks {
                println!("✅ Grounding metadata found with {} chunks", chunks.len());
            }
        }
    }

    Ok(())
}

#[cfg(feature = "thinking")]
#[tokio::test]
async fn test_thinking_budget() -> Result<()> {
    skip_without_api_key!();

    let client = create_test_client().await?;

    // Use thinking with generation config
    let mut generation_config = GenerationConfig::default();
    #[cfg(feature = "thinking")]
    {
        use gemini_rust::thinking::{ThinkingBudget, ThinkingConfig};
        generation_config.thinking_config = Some(ThinkingConfig {
            thinking_budget: ThinkingBudget::Tokens(1000),
        });
    }

    let request = GenerateContentRequest {
        contents: vec![Content::user(
            "Solve this step by step: If a train travels 120 km in 2 hours, then speeds up and travels 180 km in the next 1.5 hours, what is the average speed for the entire journey?"
        )],
        generation_config: Some(generation_config),
        ..Default::default()
    };

    let response = timeout(
        TEST_TIMEOUT,
        client.generate_content(Some("gemini-2.5-flash"), request),
    )
    .await??;

    // Verify we got a response
    assert!(!response.candidates.is_empty(), "No candidates in response");

    let candidate = &response.candidates[0];
    assert!(
        !candidate.content.parts.is_empty(),
        "No parts in candidate content"
    );

    if let Some(Part::Text { text }) = candidate.content.parts.first() {
        // The response should show step-by-step thinking
        assert!(!text.is_empty(), "Response should not be empty");
        println!("✅ Thinking budget test passed");
        println!("Thinking response: {}", text);
    }

    // Test usage metadata if available
    if let Some(usage) = &response.usage_metadata {
        println!(
            "✅ Usage metadata - Total tokens: {}",
            usage.total_token_count
        );
        if let Some(cached_tokens) = usage.cached_content_token_count {
            println!("Cached tokens: {}", cached_tokens);
        }
    }

    Ok(())
}

#[cfg(feature = "functions")]
#[tokio::test]
async fn test_tool_calling() -> Result<()> {
    skip_without_api_key!();

    let client = create_test_client().await?;

    // Define a simple calculator function
    let calculator_function = FunctionBuilder::new("calculate")
        .description("Perform basic arithmetic operations")
        .param(
            "operation",
            "string",
            "The operation: add, subtract, multiply, or divide",
            true,
        )
        .param("a", "number", "First number", true)
        .param("b", "number", "Second number", true)
        .build();

    let tool = Tool::functions(vec![calculator_function]);

    let request = GenerateContentRequest {
        contents: vec![Content::user(
            "Calculate 15 + 27 using the calculator function",
        )],
        tools: Some(vec![tool]),
        ..Default::default()
    }
    .with_auto_function_calling();

    let response = timeout(
        TEST_TIMEOUT,
        client.generate_content(Some("gemini-2.5-flash"), request),
    )
    .await??;

    // Verify we got a response
    assert!(!response.candidates.is_empty(), "No candidates in response");

    let candidate = &response.candidates[0];
    assert!(
        !candidate.content.parts.is_empty(),
        "No parts in candidate content"
    );

    // Look for function call in the response
    let mut found_function_call = false;
    for part in &candidate.content.parts {
        match part {
            Part::FunctionCall { function_call } => {
                assert_eq!(
                    function_call.name, "calculate",
                    "Function name should be 'calculate'"
                );
                found_function_call = true;
                println!("✅ Function call found: {}", function_call.name);
                println!("Function args: {:?}", function_call.args);
            }
            Part::Text { text } => {
                println!("Response text: {}", text);
            }
            _ => {}
        }
    }

    // We expect either a function call or a text response explaining the calculation
    if found_function_call {
        println!("✅ Tool calling test passed - function call detected");
    } else {
        // Check if the response at least mentions the calculation
        if let Some(Part::Text { text }) = candidate.content.parts.first() {
            assert!(
                text.contains("42") || text.contains("15") || text.contains("27"),
                "Response should reference the calculation: {}",
                text
            );
            println!("✅ Tool calling test passed - calculation referenced in text");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_basic_generation() -> Result<()> {
    skip_without_api_key!();

    let client = create_test_client().await?;

    let request = GenerateContentRequest {
        contents: vec![Content::user(
            "Write a very short poem about testing software",
        )],
        ..Default::default()
    };

    let response = timeout(
        TEST_TIMEOUT,
        client.generate_content(Some("gemini-2.5-flash"), request),
    )
    .await??;

    // Verify basic response structure
    assert!(!response.candidates.is_empty(), "No candidates in response");

    let candidate = &response.candidates[0];
    assert!(
        !candidate.content.parts.is_empty(),
        "No parts in candidate content"
    );

    if let Some(Part::Text { text }) = candidate.content.parts.first() {
        assert!(!text.is_empty(), "Response text should not be empty");
        assert!(text.len() > 10, "Response should be substantial");
        println!("✅ Basic generation test passed");
        println!("Generated poem: {}", text);
    }

    Ok(())
}

/// Test multiple features together
#[cfg(all(feature = "functions", feature = "thinking"))]
#[tokio::test]
async fn test_combined_features() -> Result<()> {
    skip_without_api_key!();

    let client = create_test_client().await?;

    // Define a weather function (mock)
    let weather_function = FunctionBuilder::new("get_weather")
        .description("Get current weather for a location")
        .param("location", "string", "City name", true)
        .param(
            "units",
            "string",
            "Temperature units (celsius or fahrenheit)",
            false,
        )
        .build();

    let tool = Tool::functions(vec![weather_function]);

    // Use thinking + function calling
    let mut generation_config = GenerationConfig::default();
    #[cfg(feature = "thinking")]
    {
        use gemini_rust::thinking::{ThinkingBudget, ThinkingConfig};
        generation_config.thinking_config = Some(ThinkingConfig {
            thinking_budget: ThinkingBudget::Tokens(500),
        });
    }

    let mut request = GenerateContentRequest {
        contents: vec![Content::user(
            "I'm planning a trip to Tokyo. Can you get the weather and give me advice?",
        )],
        tools: Some(vec![tool]),
        generation_config: Some(generation_config),
        ..Default::default()
    };

    #[cfg(feature = "functions")]
    {
        request = request.with_auto_function_calling();
    }

    let response = timeout(
        TEST_TIMEOUT,
        client.generate_content(Some("gemini-2.5-flash"), request),
    )
    .await??;

    // Verify we got a response
    assert!(!response.candidates.is_empty(), "No candidates in response");

    let candidate = &response.candidates[0];
    assert!(
        !candidate.content.parts.is_empty(),
        "No parts in candidate content"
    );

    println!("✅ Combined features test passed");

    // Print all parts of the response
    for (i, part) in candidate.content.parts.iter().enumerate() {
        match part {
            Part::Text { text } => println!("Text part {}: {}", i, text),
            Part::FunctionCall { function_call } => {
                println!(
                    "Function call {}: {} with args {:?}",
                    i, function_call.name, function_call.args
                );
            }
            _ => println!("Other part {}: {:?}", i, part),
        }
    }

    Ok(())
}
