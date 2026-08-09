//! Integration tests for the Gemini SDK.
//!
//! Tests that require a live cookie string are marked with `#[ignore]`.

use gemini_sdk::{ChatMessage, Conversation, GeminiClient, ImageSource, ModelCategory};

#[test]
fn chat_message_builders_work() {
    let msg = ChatMessage::user("hello");
    assert_eq!(msg.role, "user");
    assert_eq!(msg.parts.len(), 1);
}

#[test]
fn conversation_keeps_history() {
    let mut conv = Conversation::new().with_model_category(ModelCategory::Pro);
    conv.add_user_text("hi").add_model_text("hello");
    assert_eq!(conv.messages().len(), 2);
}

#[test]
fn image_source_from_bytes() {
    let image = ImageSource::from_bytes("image/png", b"fake");
    assert_eq!(image.mime_type(), Some("image/png"));
}

#[tokio::test]
#[ignore = "requires live GEMINI_COOKIES"]
async fn live_list_models() {
    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES required");
    let client = GeminiClient::from_cookie_header(&cookies).unwrap();
    let models = client.list_models().await.expect("failed to list models");
    assert!(!models.is_empty(), "expected at least one model");
    for model in &models {
        assert!(!model.id().is_empty());
        assert!(!model.title().is_empty());
    }
}

#[tokio::test]
#[ignore = "requires live GEMINI_COOKIES"]
async fn live_text_chat() {
    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES required");
    let client = GeminiClient::from_cookie_header(&cookies).unwrap();
    let response = client.chat().send_message("Hi").await.unwrap();
    assert!(!response.text().is_empty());
}

#[test]
fn conversation_history_grows_with_turns() {
    let mut conv = Conversation::new();
    conv.add_user_text("hi").add_model_text("hello");
    assert_eq!(conv.messages().len(), 2);
    assert_eq!(conv.messages()[0].role, "user");
    assert_eq!(conv.messages()[1].role, "model");
}

#[test]
fn conversation_preserves_category_across_clone() {
    let conv = Conversation::new().with_model_category(ModelCategory::Pro);
    let cloned = conv.clone();
    assert_eq!(conv.model_category(), Some(ModelCategory::Pro));
    assert_eq!(cloned.model_category(), Some(ModelCategory::Pro));
}

#[test]
fn continue_conversation_uses_conversation_category() {
    // Build a client only to obtain a ChatBuilder; the test never makes a
    // network call because `send_message` is not invoked.
    let client = GeminiClient::from_cookie_header(
        "__Secure-1PSID=abc; __Secure-1PSIDCC=def",
    )
    .unwrap();

    let conv = Conversation::new().with_model_category(ModelCategory::Thinking);
    let builder = client.continue_conversation(conv);
    assert_eq!(builder.category(), ModelCategory::Thinking);
}
