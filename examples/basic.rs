use anyhow::Result;
use gemini_rust::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Load API key from environment
    dotenv::dotenv().ok();

    // Create client
    let client = GeminiClient::from_env()?;

    // Simple text generation
    let request = GenerateContentRequest {
        contents: vec![Content::user("Write a haiku about Rust programming")],
        ..Default::default()
    };

    let response = client.generate_content(None, request).await?;

    if let Some(candidate) = response.candidates.first() {
        if let Some(Part::Text { text }) = candidate.content.parts.first() {
            println!("Response:\n{}", text);
        }
    }

    Ok(())
}
