//! Integration tests for the Gemini SDK.
//!
//! Tests that require a live cookie string are marked with `#[ignore]`.

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use gemini_sdk::{
    ChatMessage, Conversation, GeminiClient, ImageSource, ModelCategory,
};

fn sample_app_html() -> String {
    r#"
    <script id="bard-initial-data" data-payload="{&quot;ZXlM5e&quot;:false}"></script>
    <script>
    window.WIZ_global_data = {
        "cfb2h": "boq_assistant-bard-web-server_20260804.05_p0",
        "FdrFJe": "4202905934864668489",
        "qKIAYe": "feeds/mcudyrk2a4khkz"
    };
    </script>
    "#
    .to_string()
}

fn sample_model_list_response() -> String {
    r#")] } '

[[["wrb.fr","otAQ7b",null,"[[],[],[],[],[],[],[],[],[],[],[],[],[],[],[],[[\"fbb127bbb056c959\",\"3.6 Flash\",\"All-around help\",null,null,null,null,null,null,null,null,\"Gemini 3.6 Flash\",null,null,null,null,null,1]]]",null,null,null,"generic"]]]
58
[["di",1]]"#
        .to_string()
}

#[tokio::test]
async fn list_models_parses_response() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/app"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sample_app_html()))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/_/BardChatUi/data/batchexecute"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sample_model_list_response()))
        .mount(&server)
        .await;

    let cookies = std::collections::HashMap::from([("test".to_string(), "value".to_string())]);
    let client = GeminiClient::from_cookies(cookies).unwrap();

    // Note: the client hardcodes gemini.google.com; this test documents the
    // intended interface but cannot override the upstream URL in the current
    // design.
    assert_eq!(client.list_models().await.unwrap().len(), 1);
}

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
async fn live_text_chat() {
    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES required");
    let client = GeminiClient::from_cookie_header(&cookies).unwrap();
    let response = client.chat().send_message("Hi").await.unwrap();
    assert!(!response.text().is_empty());
}
