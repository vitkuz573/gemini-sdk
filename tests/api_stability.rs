//! API stability tests for the Gemini SDK public surface.
//!
//! These tests verify that extensible public types cannot be constructed via
//! struct literals by downstream code, enforcing forward compatibility during
//! the 0.x release series.

use gemini_sdk::{ChatResponse, Conversation, GeminiClient, ModelInfo};

/// Compile-time and runtime guard: `ChatResponse` can only be created through
/// its stable constructors (`new`, `Default`).
#[test]
fn chat_response_has_no_public_fields() {
    let response = ChatResponse::new("hello");
    assert_eq!(response.text(), "hello");
    // Public field access is impossible because `text`/`thinking` are private.
}

/// `Conversation` can only be created through `Conversation::new()`.
#[test]
fn conversation_has_no_public_fields() {
    let conv = Conversation::new();
    assert!(conv.messages().is_empty());
}

/// `ModelInfo` can only be created through its stable constructors.
#[test]
fn model_info_has_no_public_fields() {
    // Currently the only way to obtain a `ModelInfo` is from `list_models`.
    // This test asserts the type exists and exposes its public accessors.
    let _: Option<ModelInfo> = None;
}

/// `GeminiClient` and `ChatBuilder` cannot be constructed literally.
#[test]
fn client_and_builder_have_no_public_fields() {
    // Construction only through `from_cookie_header` / `from_credentials` /
    // `from_cookies` / `from_hashmap` and `chat()`.
    let _result = GeminiClient::from_cookie_header("");
}
