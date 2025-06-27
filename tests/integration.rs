use gemini_rust::prelude::*;

#[tokio::test]
async fn test_client_creation() {
    let client = GeminiClientBuilder::default().api_key("test-key").build();

    assert!(client.is_ok());
}

#[tokio::test]
async fn test_content_creation() {
    let content = Content::user("Hello");
    assert_eq!(content.role, Role::User);
    assert_eq!(content.parts.len(), 1);

    if let Part::Text { text } = &content.parts[0] {
        assert_eq!(text, "Hello");
    } else {
        panic!("Expected text part");
    }
}

#[tokio::test]
async fn test_generate_content_request_default() {
    let request = GenerateContentRequest::default();
    assert!(request.contents.is_empty());
    assert!(request.system_instruction.is_none());
    assert!(request.tools.is_none());
    assert!(request.tool_config.is_none());
    assert!(request.safety_settings.is_none());
    assert!(request.generation_config.is_none());
    assert!(request.cached_content.is_none());
}

#[test]
fn test_generation_config_default() {
    let config = GenerationConfig::default();
    assert!(config.temperature.is_none());
    assert!(config.top_p.is_none());
    assert!(config.top_k.is_none());
}

#[test]
fn test_content_builder_methods() {
    let user_content = Content::user("User message");
    let model_content = Content::model("Model response");
    let system_content = Content::system("System instruction");

    assert_eq!(user_content.role, Role::User);
    assert_eq!(model_content.role, Role::Model);
    assert_eq!(system_content.role, Role::System);
}
