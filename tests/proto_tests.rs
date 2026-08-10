//! Protocol-level unit/integration tests.

use gemini_sdk::chat::{ContentPart, ImageSource, PreparedRequest};
use gemini_sdk::errors::Error;
use gemini_sdk::models::ModelCategory;
use gemini_sdk::proto::parser::{
    extract_bard_error_code, extract_conversation_state, parse_chat_response, parse_model_list,
    parse_response_parts,
};
use gemini_sdk::proto::slots::{
    base64_decode, build_inner_req_list, derive_attachment_filename, ConversationState,
    WebAttachment,
};
use gemini_sdk::proto::{build_batchexecute_body, build_stream_generate_body, strip_xssi_prefix};

#[test]
fn strip_xssi_prefix_finds_first_json_line() {
    let body = include_str!("fixtures/xssi_prefix.txt");
    assert_eq!(strip_xssi_prefix(body), Some("[[\"wrb.fr\",\"x\"]]"));
}

#[test]
fn build_batchexecute_body_has_f_req() {
    let body = build_batchexecute_body(None);
    assert!(body.contains("f.req="));
}

#[test]
fn build_stream_generate_body_url_encodes() {
    let inner = vec![serde_json::Value::Null; 97];
    let body = build_stream_generate_body(&inner, None);
    assert!(body.starts_with("f.req="));
}

#[test]
fn parse_model_list_extracts_gemini_flash() {
    let body = include_str!("fixtures/model_list_minimal.txt");
    let models = parse_model_list(body).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].display_name(), "Gemini 3.6 Flash");
    assert_eq!(models[0].category_enum(), 1);
}

#[test]
fn parse_model_list_from_real_fixture() {
    let body = include_str!("fixtures/model_list_response.txt");
    let models = parse_model_list(body).unwrap();
    assert!(!models.is_empty());
    for model in &models {
        assert!(!model.id().is_empty());
        assert!(!model.title().is_empty());
    }
}

#[test]
fn parse_simple_text_response() {
    let body = include_str!("fixtures/chat_response_minimal.json");
    let response = parse_chat_response(body).unwrap();
    assert_eq!(response.text(), "Hello, world!");
}

#[test]
fn parse_concatenated_text_response() {
    let body = include_str!("fixtures/chat_response_concatenated.json");
    let response = parse_chat_response(body).unwrap();
    assert_eq!(response.text(), "Hello, world!");
}

#[test]
fn parse_thinking_response() {
    let body = include_str!("fixtures/chat_response_thinking.json");
    let response = parse_chat_response(body).unwrap();
    assert_eq!(response.text(), "hello ");
    assert_eq!(response.thinking(), "think step 1");
}

#[test]
fn parse_chat_response_detects_bard_error_1100() {
    let body = include_str!("fixtures/bard_error_1100.json");
    let result = parse_chat_response(body);
    assert!(result.is_err());
}

#[test]
fn parse_chat_response_detects_bard_error_wrapper() {
    let body = include_str!("fixtures/bard_error_wrapper.json");
    let result = parse_chat_response(body);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let message = err.to_string();
    assert!(message.contains("BardErrorInfo"), "error should mention BardErrorInfo: {message}");
    assert!(message.contains("1101"), "error should include the Bard error code: {message}");
}

#[test]
fn extract_bard_error_code_parses_code() {
    let body = include_str!("fixtures/bard_error_1096.json");
    assert_eq!(extract_bard_error_code(body), Some("1096".to_string()));
}

#[test]
fn extract_first_turn_meta_token() {
    let body = include_str!("fixtures/conversation_state_first_turn.json");
    let state = extract_conversation_state(body).unwrap();
    assert_eq!(state.conversation_id, "c_abc");
    assert_eq!(state.response_id, "r_def");
    assert_eq!(state.continuation_token, "first_turn_token");
}

#[test]
fn extract_continuation_token_key_21() {
    let body = include_str!("fixtures/conversation_state_key_21.json");
    let state = extract_conversation_state(body).unwrap();
    assert_eq!(state.conversation_id, "c_abc");
    assert_eq!(state.response_id, "r_def");
    assert_eq!(state.continuation_token, "token_value");
}

#[test]
fn malformed_response_no_panic() {
    let body = r#"[["wrb.fr",null,"not-valid-json"]]"#;
    let result = parse_chat_response(body);
    assert!(
        matches!(result, Err(Error::Parse(_))),
        "malformed frame should return structured parse error, got {result:?}"
    );
}

#[test]
fn extract_conversation_state_reads_ids_and_token() {
    let body = include_str!("fixtures/conversation_state.json");

    let state = extract_conversation_state(body).unwrap();
    assert_eq!(state.conversation_id, "c_abc");
    assert_eq!(state.response_id, "r_def");
    assert_eq!(state.continuation_token, "token_value");
}

#[test]
fn extract_conversation_state_from_real_fixture() {
    let body = include_str!("fixtures/turn1_response_raw.txt");
    let state = extract_conversation_state(body).unwrap();
    assert!(!state.conversation_id.is_empty());
    assert!(!state.response_id.is_empty());
    assert!(!state.response_part_id.is_empty());
    assert!(!state.continuation_token.is_empty());
}

#[test]
fn parse_real_response_fixture() {
    let body = include_str!("fixtures/turn1_response_raw.txt");
    let response = parse_chat_response(body).unwrap();
    assert!(!response.text().is_empty());
}

#[test]
fn parse_thinking_response_extracts_reasoning() {
    let body = include_str!("fixtures/thinking_response_raw.txt");
    let response = parse_chat_response(body).unwrap();

    assert!(response.text().contains("идентичный скриншот"));
    assert!(response.thinking().contains("**Comparing Images**"));
    assert!(response.thinking().contains("**Confirming Identity**"));
    assert!(response.thinking().contains("exact duplicate"));
    assert!(!response.text().contains("Comparing Images"));
}

#[test]
fn parse_response_parts_deduplicates_stream_chunks() {
    let body = include_str!("fixtures/thinking_response_raw.txt");
    let parts = parse_response_parts(body).unwrap();

    let text_parts: Vec<_> = parts
        .iter()
        .filter_map(|p| match p {
            ContentPart::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    let thinking_parts: Vec<_> = parts
        .iter()
        .filter_map(|p| match p {
            ContentPart::Thinking(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();

    assert_eq!(text_parts.len(), 1);
    assert_eq!(thinking_parts.len(), 1);
    assert!(thinking_parts[0].starts_with("**Comparing Images**"));
}

#[test]
fn build_inner_req_list_has_97_slots() {
    let prepared = PreparedRequest {
        prompt: "hello".to_string(),
        inline_images: vec![],
        inline_audio: vec![],
        inline_video: vec![],
        config: None,
        category: ModelCategory::Auto,
    };
    let inner = build_inner_req_list(&prepared, None, None, &[], "UUID", "en", None, "nonce");
    assert_eq!(inner.len(), 97);
}

#[test]
fn build_inner_req_list_with_attachments() {
    let prepared = PreparedRequest {
        prompt: "describe".to_string(),
        inline_images: vec![],
        inline_audio: vec![],
        inline_video: vec![],
        config: None,
        category: ModelCategory::Fast,
    };
    let attachments = vec![WebAttachment {
        reference: "/contrib_service/ttl_1d/abc".to_string(),
        mime_type: "image/png".to_string(),
        filename: "test.png".to_string(),
    }];
    let inner =
        build_inner_req_list(&prepared, None, None, &attachments, "UUID", "en", None, "nonce");
    assert!(inner[0][3].is_array());
}

#[test]
fn build_inner_req_list_with_conversation_state() {
    let prepared = PreparedRequest {
        prompt: "follow up".to_string(),
        inline_images: vec![],
        inline_audio: vec![],
        inline_video: vec![],
        config: None,
        category: ModelCategory::Pro,
    };
    let state = ConversationState {
        conversation_id: "c_abc".to_string(),
        response_id: "r_def".to_string(),
        response_part_id: "rcp_123".to_string(),
        continuation_token: "tok".to_string(),
    };
    let inner =
        build_inner_req_list(&prepared, Some(&state), None, &[], "UUID", "en", None, "nonce");
    assert_eq!(inner[17], serde_json::json!([[1]]));
    assert_eq!(inner[30], serde_json::json!([3]));
}

#[test]
fn build_inner_req_list_slot_30_reflects_model_category() {
    for (category, expected) in [
        (ModelCategory::Fast, 1),
        (ModelCategory::Thinking, 2),
        (ModelCategory::Pro, 3),
        (ModelCategory::Auto, 4),
        (ModelCategory::FastDynamicThinking, 5),
        (ModelCategory::FlashLite, 6),
    ] {
        let prepared = PreparedRequest {
            prompt: "hello".to_string(),
            inline_images: vec![],
            inline_audio: vec![],
            inline_video: vec![],
            config: None,
            category,
        };
        let inner = build_inner_req_list(&prepared, None, None, &[], "UUID", "en", None, "nonce");
        assert_eq!(inner[30], serde_json::json!([expected]), "slot 30 for {category:?}");
    }
}

#[test]
fn derive_attachment_filename_extensions() {
    assert_eq!(derive_attachment_filename("image/png", 0), "attachment.png");
    assert_eq!(derive_attachment_filename("image/jpeg", 0), "attachment.jpg");
    assert_eq!(derive_attachment_filename("application/pdf", 1), "attachment_1.pdf");
}

#[test]
fn base64_decode_tolerates_whitespace() {
    let decoded = base64_decode("aGVsbG8g\nd29ybGQ=").unwrap();
    assert_eq!(String::from_utf8(decoded).unwrap(), "hello world");
}

#[test]
fn base64_decode_rejects_invalid() {
    assert!(base64_decode("!!!").is_err());
}

#[test]
fn image_source_from_bytes_encodes_base64() {
    let image = ImageSource::from_bytes("image/png", b"fake-image-bytes");
    let (mime, data) = match image {
        ImageSource::InlineData { mime_type, data } => (mime_type, data),
        ImageSource::Url { .. } => panic!("expected inline data"),
    };
    assert_eq!(mime, "image/png");
    assert_eq!(data, "ZmFrZS1pbWFnZS1ieXRlcw==");
}

#[test]
fn build_inner_req_list_with_inline_images() {
    let prepared = PreparedRequest {
        prompt: "Look at this".to_string(),
        inline_images: vec![("image/png".to_string(), "ZmFrZQ==".to_string())],
        inline_audio: vec![],
        inline_video: vec![],
        config: None,
        category: ModelCategory::Auto,
    };
    let inner = build_inner_req_list(&prepared, None, None, &[], "UUID", "en", None, "nonce");
    // Inline images are uploaded first and become WebAttachments; without
    // uploading, the empty-attachment path still builds a valid slot 0.
    assert!(inner[0].is_array());
    assert_eq!(inner[0][0], serde_json::json!("Look at this"));
}

#[test]
fn system_instruction_in_slot0() {
    let config = gemini_sdk::chat::GenerationConfig::default()
        .with_system_instruction("You are a Rust expert");
    let prepared = PreparedRequest {
        prompt: "hello".to_string(),
        inline_images: vec![],
        inline_audio: vec![],
        inline_video: vec![],
        config: Some(config),
        category: ModelCategory::Auto,
    };
    let inner = build_inner_req_list(&prepared, None, None, &[], "UUID", "en", None, "nonce");
    let prompt = inner[0][0].as_str().expect("slot 0 prompt is a string");
    assert!(prompt.starts_with("You are a Rust expert\nhello"));
}

#[test]
fn no_system_instruction_preserved() {
    let prepared = PreparedRequest {
        prompt: "hello".to_string(),
        inline_images: vec![],
        inline_audio: vec![],
        inline_video: vec![],
        config: None,
        category: ModelCategory::Auto,
    };
    let inner = build_inner_req_list(&prepared, None, None, &[], "UUID", "en", None, "nonce");
    assert_eq!(inner[0][0], serde_json::json!("hello"));
}
