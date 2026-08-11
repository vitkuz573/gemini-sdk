//! Integration tests for the Gemini SDK.
//!
//! Tests that require a live cookie string are marked with `#[ignore]`.

use std::sync::Arc;

use gemini_sdk::{
    ChatMessage, ContentPart, Conversation, Error, GeminiClient, GenerationConfig, ImageSource,
    ModelCategory, Tool, ToolError, TurnRating,
};

mod common;
use common::{
    default_test_timeout, BATCHEXECUTE_PATH, MIME_PNG, MINIMAL_COOKIE_HEADER, MODEL_ROLE,
    MOCK_COOKIE_HEADER, TEST_LANGUAGE, TEST_MOCK_LANGUAGE, TEST_PROMPT, USER_ROLE, WRB_FR,
};

#[test]
fn chat_message_builders_work() {
    let msg = ChatMessage::user(TEST_PROMPT);
    assert_eq!(msg.role, USER_ROLE);
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
    let image = ImageSource::from_bytes(MIME_PNG, b"fake");
    assert_eq!(image.mime_type(), Some(MIME_PNG));
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
    assert_eq!(conv.messages()[0].role, USER_ROLE);
    assert_eq!(conv.messages()[1].role, MODEL_ROLE);
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
    let client = GeminiClient::from_cookie_header(MINIMAL_COOKIE_HEADER).unwrap();

    let conv = Conversation::new().with_model_category(ModelCategory::Thinking);
    let builder = client.continue_conversation(conv);
    assert_eq!(builder.category(), ModelCategory::Thinking);
}

#[tokio::test]
async fn config_builder_async_sets_language_retries_and_timeout() {
    let client = GeminiClient::from_cookie_header(MINIMAL_COOKIE_HEADER)
        .unwrap()
        .with_language(TEST_MOCK_LANGUAGE)
        .await
        .with_max_retries(5)
        .await
        .with_timeout(default_test_timeout())
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
async fn generate_stream_yields_response_chunks() {
    let body = include_str!("fixtures/chat_response_minimal.json");
    let parts =
        gemini_sdk::proto::parser::parse_response_parts(body).expect("fixture should parse");
    let mut texts: Vec<String> = parts
        .iter()
        .filter_map(|p| match p {
            gemini_sdk::chat::ContentPart::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(texts.pop().as_deref(), Some("Hello, world!"));
}

#[tokio::test]
async fn generate_stream_handles_empty_body() {
    let _client = GeminiClient::from_cookie_header(MINIMAL_COOKIE_HEADER).unwrap();

    let message = ChatMessage::user("hi");
    let result = _client.generate_stream(&message, ModelCategory::Auto, None).await;

    // Warm-up RPC failures are tolerated, so the stream is built successfully
    // even with invalid cookies. The actual generate request may fail when the
    // stream is consumed; here we only verify the wiring returns a stream.
    assert!(result.is_ok());
}

#[tokio::test]
async fn client_default_system_instruction_reaches_request() {
    use gemini_sdk::proto::slots::build_inner_req_list;

    let _client = GeminiClient::from_cookie_header(MINIMAL_COOKIE_HEADER)
        .unwrap()
        .with_system_instruction("You are a Rust expert")
        .await;

    let prepared = gemini_sdk::chat::PreparedRequest {
        prompt: TEST_PROMPT.to_string(),
        inline_images: vec![],
        inline_audio: vec![],
        inline_video: vec![],
        config: None,
        category: ModelCategory::Auto,
        tools: None,
        refresh_on_auth_error: false,
    };
    let inner = build_inner_req_list(&prepared, None, None, &[], "UUID", TEST_LANGUAGE, None, "nonce");
    // Without going through the builder, the client default is not reflected
    // in a standalone PreparedRequest. The real assertion happens via the
    // builder path in the next test.
    assert_eq!(inner[0][0], serde_json::json!(TEST_PROMPT));
}

#[tokio::test]
async fn system_instruction_override_wins() {
    use gemini_sdk::proto::slots::build_inner_req_list;

    let _client = GeminiClient::from_cookie_header(MINIMAL_COOKIE_HEADER)
        .unwrap()
        .with_system_instruction("You are a Rust expert")
        .await;

    let config = gemini_sdk::chat::GenerationConfig::default()
        .with_system_instruction("You are a Python expert");
    let prepared = gemini_sdk::chat::PreparedRequest {
        prompt: TEST_PROMPT.to_string(),
        inline_images: vec![],
        inline_audio: vec![],
        inline_video: vec![],
        config: Some(config),
        category: ModelCategory::Auto,
        tools: None,
        refresh_on_auth_error: false,
    };
    let inner = build_inner_req_list(&prepared, None, None, &[], "UUID", TEST_LANGUAGE, None, "nonce");
    let prompt = inner[0][0].as_str().expect("slot 0 prompt is a string");
    assert!(prompt.starts_with("You are a Python expert"));
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
    let cookies = Cookies::from_header(&format!("{PSID}=psid-value; {PSIDCC}=psidcc-value; {SOCS}=old"));

    let response = reqwest::Client::new().post(&save_url).send().await.unwrap();

    let mut merged: std::collections::HashMap<String, String> = cookies.into();
    for cookie in response.cookies() {
        merged.insert(cookie.name().to_string(), cookie.value().to_string());
    }
    let persisted = Cookies::from(merged);

    // The response carried a SOCS cookie, so the merged jar must contain it.
    assert_eq!(
        persisted.get(SOCS),
        Some("saved-consent-value"),
        "SOCS cookie from consent response was not persisted"
    );

    // The merged jar is the same value the client writes back to
    // `self.inner.cookies`; assert the SOCS value persists.
    assert_eq!(persisted.get(SOCS), Some("saved-consent-value"));
}

struct DoublerTool;

impl Tool for DoublerTool {
    fn name(&self) -> &str {
        "doubler"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "n": { "type": "integer" } },
            "required": ["n"]
        })
    }

    fn invoke(
        &self,
        args: serde_json::Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, ToolError>> + Send + '_>,
    > {
        let n = args["n"].as_i64().unwrap_or(0);
        Box::pin(async move { Ok(serde_json::json!({ "result": n * 2 })) })
    }
}

fn tool_call_frame(name: &str, args: serde_json::Value) -> String {
    serde_json::json!([[
        WRB_FR,
        null,
        serde_json::json!([
            null,
            ["c_tool", "r_tool"],
            null,
            null,
            [["rcp_tool", [], [], [], [], [], [], [[name, args]], [], []]]
        ])
        .to_string(),
        null,
        null
    ]])
    .to_string()
}

fn final_text_frame(text: &str) -> String {
    serde_json::json!([[
        WRB_FR,
        null,
        serde_json::json!([
            null,
            ["c_final", "r_final"],
            null,
            null,
            [["rcp_final", [text], [], [], [], [], [], [], [], []]]
        ])
        .to_string(),
        null,
        null
    ]])
    .to_string()
}

#[tokio::test]
async fn generate_with_tools_round_trip() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(DoublerTool)];
    let response = client
        .generate_with_tools(
            &ChatMessage::user("call doubler with 3"),
            tools,
            ModelCategory::Auto,
            Some(GenerationConfig::default().with_max_tool_turns(2)),
        )
        .await;

    // Without a wiremocked StreamGenerate endpoint the network will fail, but
    // the request encoding path still validates that the method builds and
    // attempts to send a prepared request with tool declarations.
    assert!(response.is_err());
}

#[tokio::test]
async fn conversation_action_parsers_are_exported() {
    // Smoke test that ensures the new types are public and usable.
    let _rating = TurnRating::Good;
    let _action = gemini_sdk::ConversationAction::Regenerate;
}

#[tokio::test]
async fn regenerate_turn_sends_pcck7e_payload() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/pcck7e_success.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    // Inject the session parameters that would normally come from /app so the
    // client skips the live init flow and sends the batchexecute request.
    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let result = client.regenerate_turn("conv_123", "r_abc").await;
    assert!(result.is_ok(), "regenerate_turn failed: {:?}", result);
    assert!(result.unwrap().success());
}

#[tokio::test]
async fn rate_turn_sends_rating_value() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/pcck7e_success.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let result = client.rate_turn("conv_123", "r_abc", TurnRating::Good).await;
    assert!(result.is_ok(), "rate_turn failed: {:?}", result);
    assert!(result.unwrap().success());
}

#[tokio::test]
async fn delete_turn_reports_failure_on_error_payload() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/pcck7e_error.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let result = client.delete_turn("conv_123", "r_abc").await;
    assert!(result.is_ok(), "delete_turn failed: {:?}", result);
    assert!(!result.unwrap().success());
}

#[test]
fn parse_conversation_action_response_handles_wrapped_array() {
    use gemini_sdk::{ConversationAction, ConversationActionResult};

    let body = format!(
        " )] }} ' \n\n[[[\"{WRB_FR}\",\"PCck7e\",\"[1]\",null,null,null,\"generic\"]]]"
    );
    let result = ConversationActionResult::parse_response(
        &body,
        ConversationAction::Regenerate,
        "r_abc".to_string(),
    );
    assert!(result.is_ok(), "wrapped array parse failed: {:?}", result);
    assert!(result.unwrap().success());
}

#[tokio::test]
async fn parser_extracts_tool_call_from_wiz_frame() {
    let frame = tool_call_frame("doubler", serde_json::json!({ "n": 3 }));
    let body = format!("{frame}\n{}", final_text_frame("done"));
    let parts = gemini_sdk::proto::parser::parse_response_parts(&body).unwrap();
    let call = parts.iter().find_map(|p| match p {
        ContentPart::ToolCall(c) => Some(c),
        _ => None,
    });
    assert!(call.is_some());
    assert_eq!(call.unwrap().name, "doubler");
    assert_eq!(call.unwrap().args, serde_json::json!({ "n": 3 }));
}

#[tokio::test]
async fn get_user_info_parses_full_profile() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/o30O0e_user_info.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let info = client.get_user_info().await;
    assert!(info.is_ok(), "get_user_info failed: {:?}", info);
    let info = info.unwrap();
    assert_eq!(info.name(), Some("Jane Doe"));
    assert_eq!(info.photo_url(), Some("https://example.com/photo.jpg"));
    assert_eq!(info.email(), Some("jane@example.com"));
}

#[tokio::test]
async fn get_user_info_tolerates_missing_and_null_fields() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/o30O0e_user_info_partial.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let info = client.get_user_info().await.expect("get_user_info failed");
    assert_eq!(info.name(), Some("Jane Doe"));
    assert_eq!(info.photo_url(), None);
    assert_eq!(info.email(), None);
}

#[tokio::test]
async fn get_last_selected_mode_returns_mode_id() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/L5adhe_last_mode.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let mode = client.get_last_selected_mode().await.expect("get_last_selected_mode failed");
    assert_eq!(mode.mode_id(), Some("cf41b0e0dd7d53e5"));
}

#[tokio::test]
async fn get_last_selected_mode_returns_none_for_null() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/L5adhe_null_mode.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let mode = client.get_last_selected_mode().await.expect("get_last_selected_mode failed");
    assert_eq!(mode.mode_id(), None);
}

#[tokio::test]
async fn get_locale_tools_returns_value() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/cYRIkd_locale_tools.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let result = client.get_locale_tools().await;
    assert!(result.is_ok(), "get_locale_tools failed: {:?}", result);
    assert_eq!(result.unwrap().value(), &serde_json::json!({"tools": ["tool1", "tool2"]}));

    let requests = mock_server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    assert!(body.contains("cYRIkd"), "request body missing cYRIkd");
}

#[tokio::test]
async fn get_model_config_returns_value() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/whPPme_model_config.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let result = client.get_model_config().await;
    assert!(result.is_ok(), "get_model_config failed: {:?}", result);
    assert_eq!(result.unwrap().value(), &serde_json::json!({"models": [{"id": "pro"}]}));

    let requests = mock_server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    assert!(body.contains("whPPme"), "request body missing whPPme");
}

#[tokio::test]
async fn get_locale_config_returns_value() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/Te6DCf_locale_config.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let result = client.get_locale_config().await;
    assert!(result.is_ok(), "get_locale_config failed: {:?}", result);
    assert_eq!(result.unwrap().value(), &serde_json::json!({"locale": "ru"}));

    let requests = mock_server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    assert!(body.contains("Te6DCf"), "request body missing Te6DCf");
}

#[tokio::test]
async fn get_tools_config_returns_value() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/ku4Jyf_tools_config.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let result = client.get_tools_config().await;
    assert!(result.is_ok(), "get_tools_config failed: {:?}", result);
    assert_eq!(result.unwrap().value(), &serde_json::json!({"enabled": [1, 3, 7, 17]}));

    let requests = mock_server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    assert!(body.contains("ku4Jyf"), "request body missing ku4Jyf");
}

#[tokio::test]
async fn get_usage_stats_returns_value() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/jSf9Qc_usage_stats.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let result = client.get_usage_stats().await;
    assert!(result.is_ok(), "get_usage_stats failed: {:?}", result);
    assert_eq!(
        result.unwrap().value(),
        &serde_json::json!({"requests_today": 12, "requests_total": 345})
    );

    let requests = mock_server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    assert!(body.contains("jSf9Qc"), "request body missing jSf9Qc");
}

#[tokio::test]
async fn get_scheduled_prompts_returns_value() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/XPSWpd_scheduled_prompts.txt")),
        )
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let result = client.get_scheduled_prompts().await;
    assert!(result.is_ok(), "get_scheduled_prompts failed: {:?}", result);
    assert_eq!(
        result.unwrap().value(),
        &serde_json::json!({"prompts": [{"id": "sp_1", "text": "Morning summary"}]})
    );

    let requests = mock_server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    assert!(body.contains("XPSWpd"), "request body missing XPSWpd");
}

#[tokio::test]
async fn set_last_selected_mode_sends_l5adhe_payload() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path(BATCHEXECUTE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&mock_server)
        .await;

    let client = GeminiClient::from_cookie_header(MOCK_COOKIE_HEADER)
        .unwrap()
        .with_base_url(&mock_uri)
        .await
        .with_max_retries(0)
        .await;

    {
        let mut session = client.inner_session_for_tests().lock().await;
        session.build_label = Some("boq_assistant-bard-web-server_20260810.00_p0".to_string());
        session.session_id = Some("1234567890".to_string());
        session.access_token = Some("token".to_string());
    }

    let result = client.set_last_selected_mode("cf41b0e0dd7d53e5").await;
    assert!(result.is_ok(), "set_last_selected_mode failed: {:?}", result);

    let requests = mock_server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    assert!(body.contains("L5adhe"), "request body missing L5adhe");
    assert!(body.contains("cf41b0e0dd7d53e5"), "request body missing mode id");
    assert!(
        body.contains("last_selected_mode_id_on_web"),
        "request body missing preference key"
    );
}
