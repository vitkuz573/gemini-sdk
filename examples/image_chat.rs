//! Example: send a message with an inline image.
//!
//! Run with:
//!
//! ```text
//! GEMINI_COOKIES="__Secure-1PSID=...; __Secure-1PSIDCC=..." \
//!   cargo run --example image_chat -- /path/to/image.png "Describe this image."
//! ```

use gemini_sdk::{GeminiClient, ImageSource};

#[tokio::main]
async fn main() -> gemini_sdk::Result<()> {
    tracing_subscriber::fmt::init();

    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES env var required");
    let image_path = std::env::args().nth(1).expect("image path required");
    let prompt = std::env::args().nth(2).unwrap_or_else(|| "Describe this image.".to_string());

    let bytes = tokio::fs::read(&image_path).await.expect("failed to read image");
    let mime = match image_path.split('.').next_back() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    };

    let image = ImageSource::from_bytes(mime, &bytes);
    let client = GeminiClient::from_cookie_header(&cookies)?;

    let response = client.chat().send_message_with_images(&prompt, vec![image]).await?;

    println!("Gemini: {}", response.text());

    Ok(())
}
