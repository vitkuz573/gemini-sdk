//! Example: send a simple text-only message to the Gemini web frontend.
//!
//! Run with:
//!
//! ```text
//! GEMINI_COOKIES="__Secure-1PSID=...; __Secure-1PSIDCC=..." \
//!   cargo run --example text_chat -- "What is Rust?"
//! ```

use gemini_sdk::{GeminiClient, ModelCategory};

#[tokio::main]
async fn main() -> gemini_sdk::Result<()> {
    tracing_subscriber::fmt::init();

    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES env var required");
    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Hello, Gemini!".to_string());

    let client = GeminiClient::from_cookie_header(&cookies)?;

    let response = client
        .chat()
        .with_category(ModelCategory::Auto)
        .send_message(&prompt)
        .await?;

    println!("Gemini: {}", response.text());

    Ok(())
}
