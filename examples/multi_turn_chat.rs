//! Example: continue an existing conversation over multiple turns.
//!
//! Run with:
//!
//! ```text
//! GEMINI_COOKIES="__Secure-1PSID=...; __Secure-1PSIDCC=..." \
//!   cargo run --example multi_turn_chat -- "What is Rust?" "What about memory safety?"
//! ```

use gemini_sdk::{GeminiClient, ModelCategory};

#[tokio::main]
async fn main() -> gemini_sdk::Result<()> {
    tracing_subscriber::fmt::init();

    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES env var required");
    let first_prompt = std::env::args().nth(1).unwrap_or_else(|| "Hello, Gemini!".to_string());
    let follow_up_prompt = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "Can you tell me more?".to_string());

    let client = GeminiClient::from_cookie_header(&cookies)?;

    let first_response = client
        .chat()
        .with_category(ModelCategory::Auto)
        .send_message(&first_prompt)
        .await?;
    println!("Gemini: {}", first_response.text());

    let mut conversation = gemini_sdk::Conversation::new()
        .with_model_category(ModelCategory::Auto);
    conversation.add_user_text(first_prompt);
    conversation.add_model_text(first_response.text().to_string());

    let follow_up_response = client
        .continue_conversation(conversation)
        .send_message(&follow_up_prompt)
        .await?;
    println!("Gemini: {}", follow_up_response.text());

    Ok(())
}
