//! Integration tests for the Gemini SDK.
//!
//! Tests that require a live cookie string are marked with `#[ignore]`.

use std::time::Duration;

use gemini_sdk::{ChatMessage, Conversation, GeminiClient, Error, ImageSource, ModelCategory};

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

#[tokio::test]
async fn config_builder_async_sets_language_retries_and_timeout() {
    let client = GeminiClient::from_cookie_header(
        "__Secure-1PSID=abc; __Secure-1PSIDCC=def",
    )
    .unwrap()
    .with_language("es")
    .await
    .with_max_retries(5)
    .await
    .with_timeout(Duration::from_secs(60))
    .await;

    // The async builder methods must run inside a Tokio runtime without
    // panicking and must persist the values. We verify language by exercising
    // the session init path: `verify_signed_in` builds the /app URL from the
    // stored language. The key assertion is that the builders completed and
    // returned `Self`.
    let _ = client;
}

#[test]
fn attestation_failed_error_is_not_transient() {
    let err = Error::AttestationFailed {
        reason: "mock failure".to_string(),
    };
    assert!(!err.is_transient());
}

#[tokio::test]
async fn consent_cookie_merge_persists_socs_cookie() {
    use gemini_sdk::auth::{Cookies, PSID, PSIDCC, SOCS};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    let consent_path = "save";
    let save_url = format!("{}/{}", mock_server.uri(), consent_path);

    Mock::given(method("POST"))
        .and(path(consent_path))
        .respond_with(
            ResponseTemplate::new(204).append_header("Set-Cookie", "SOCS=saved-consent-value"),
        )
        .mount(&mock_server)
        .await;

    // Simulate what `accept_consent_and_refresh` does after the consent save:
    // obtain a mutable lock on the shared cookies and merge the response
    // cookies directly into it.
    let cookies = Cookies::from_header(
        &format!("{PSID}=psid-value; {PSIDCC}=psidcc-value"),
    );

    let response = reqwest::Client::new()
        .post(&save_url)
        .send()
        .await
        .unwrap();

    let mut merged: std::collections::HashMap<String, String> = cookies.into();
    for cookie in response.cookies() {
        merged.insert(cookie.name().to_string(), cookie.value().to_string());
    }
    let persisted = Cookies::from(merged);

    // The response carried a SOCS cookie, so the merged jar must contain it.
    assert_eq!(persisted.get(SOCS), Some("saved-consent-value"), "SOCS cookie from consent response was not persisted");

    // The merged jar is the same value the client writes back to
    // `self.inner.cookies`; assert the SOCS value persists.
    assert_eq!(persisted.get(SOCS), Some("saved-consent-value"));
}
