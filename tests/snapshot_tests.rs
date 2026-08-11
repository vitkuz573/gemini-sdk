use gemini_sdk::{Conversation, GeminiClient, ModelCategory};
use serde_json::Value;

mod common;
use common::{MODEL_ROLE, MOCK_COOKIE_HEADER, USER_ROLE};

#[test]
fn conversation_save_restore_roundtrip() {
    let mut conversation = Conversation::new().with_model_category(ModelCategory::Pro);
    conversation.add_user_text("Hello");
    conversation.add_model_text("Hi there");

    let snapshot = conversation.save().unwrap();
    let restored = Conversation::restore(&snapshot).unwrap();

    assert_eq!(restored.messages().len(), 2);
    assert_eq!(restored.messages()[0].role, USER_ROLE);
    assert_eq!(restored.messages()[1].role, MODEL_ROLE);
    assert_eq!(restored.model_category(), Some(ModelCategory::Pro));
}

#[tokio::test]
async fn client_save_restore_roundtrip() {
    let original = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER).unwrap();
    let snapshot = original.save_session().await.unwrap();
    let (_restored, _) = GeminiClient::restore_session(&snapshot).await.unwrap();

    // credentials are restored through the snapshot; verifying the snapshot
    // round-trips and that the restored client can be constructed is sufficient.
    let parsed: Value = serde_json::from_str(&snapshot).unwrap();
    assert_eq!(parsed["format_version"], 1);
    assert!(
        parsed["credentials"]["psid"].as_str().unwrap().contains("abc"),
        "psid should contain the original cookie value; snapshot: {snapshot}"
    );
}

#[tokio::test]
async fn snapshot_contains_format_version() {
    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER).unwrap();
    let mut conversation = Conversation::new();
    conversation.add_user_text("Hello");

    let snapshot = client.save_session_with_conversation(&conversation).await.unwrap();
    let parsed: Value = serde_json::from_str(&snapshot).unwrap();

    assert_eq!(parsed["format_version"], 1);
    assert!(parsed["credentials"].is_object());
    assert!(parsed["session"].is_object());
    assert!(parsed["conversation"].is_object());
}
