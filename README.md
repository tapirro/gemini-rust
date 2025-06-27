# gemini-rust

A comprehensive Rust client for Google's Gemini API with full feature support.

## Features

- 🏗️ **Structured Output** - Type-safe JSON schema generation
- 🧠 **Thinking Mode** - Gemini 2.5's reasoning capabilities  
- 🔧 **Function Calling** - Parallel and compositional functions
- 🔍 **Grounding** - Google Search and URL context
- 💾 **Context Caching** - Efficient token management
- 🌊 **Streaming** - Real-time response streaming
- 🔄 **Automatic Retries** - With exponential backoff
- 🦀 **100% Rust** - Type-safe and memory-safe

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
gemini-rust = "0.1"
```

Or with specific features:

```toml
[dependencies]
gemini-rust = { version = "0.1", features = ["grounding", "caching"] }
```

## Quick Start

```rust
use gemini_rust::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create client from GEMINI_API_KEY environment variable
    let client = GeminiClient::from_env()?;
    
    // Generate content
    let response = client
        .generate_content(None, GenerateContentRequest {
            contents: vec![Content::user("Hello, Gemini!")],
            ..Default::default()
        })
        .await?;
    
    println!("{:?}", response);
    Ok(())
}
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))