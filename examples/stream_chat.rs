//! Example: consume a streaming `StreamGenerate` response line by line.
//!
//! Run with:
//!
//! ```text
//! GEMINI_COOKIES="__Secure-1PSID=...; __Secure-1PSIDCC=..." \
//!   cargo run --example stream_chat -- "Tell me a story"
//! ```

use futures::StreamExt;
use gemini_sdk::{ChatMessage, GeminiClient, ModelCategory};

#[tokio::main]
async fn main() -> gemini_sdk::Result<()> {
    tracing_subscriber::fmt::init();

    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES env var required");
    let prompt = std::env::args().nth(1).unwrap_or_else(|| "Tell me a short story.".to_string());

    let client = GeminiClient::from_cookie_header(&cookies)?;
    let message = ChatMessage::user(prompt);

    let response = client.stream_generate(&message, ModelCategory::Auto, None).await?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                print!("{}", String::from_utf8_lossy(&bytes));
            }
            Err(e) => eprintln!("stream error: {e}"),
        }
    }

    Ok(())
}
