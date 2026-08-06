//! Protocol-level unit/integration tests.

use gemini_sdk::proto::parser::{
    extract_bard_error_code, extract_conversation_state, parse_chat_response, parse_model_list,
};
use gemini_sdk::proto::slots::{
    base64_decode, build_inner_req_list, derive_attachment_filename, ConversationState,
    WebAttachment,
};
use gemini_sdk::proto::{build_batchexecute_body, build_stream_generate_body, strip_xssi_prefix};
use gemini_sdk::chat::PreparedRequest;
use gemini_sdk::models::ModelCategory;

#[test]
fn strip_xssi_prefix_finds_first_json_line() {
    let body = ")] } ' \n\n[[\"wrb.fr\",\"x\"]]\n58";
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
    let body = r#")] } '

[[["wrb.fr","otAQ7b",null,"[[],[],[],[],[],[],[],[],[],[],[],[],[],[],[],[[\"fbb127bbb056c959\",\"3.6 Flash\",\"All-around help\",null,null,null,null,null,null,null,null,\"Gemini 3.6 Flash\",null,null,null,null,null,1]]]",null,null,null,"generic"]]]
58
[["di",1]]"#;

    let models = parse_model_list(body).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].display_name(), "Gemini 3.6 Flash");
    assert_eq!(models[0].category_enum, 1);
}

#[test]
fn parse_model_list_from_real_fixture() {
    let body = include_str!("fixtures/model_list_response.txt");
    let models = parse_model_list(body).unwrap();
    assert!(!models.is_empty());
    for model in &models {
        assert!(!model.id.is_empty());
        assert!(!model.title.is_empty());
    }
}

#[test]
fn parse_chat_response_extracts_text() {
    let body = r#"[["wrb.fr", null, "[[null, null, null, null, [[\"rc_123\", [\"Hello, world!\"]]]]]"]]"#;
    let response = parse_chat_response(body).unwrap();
    assert_eq!(response.text(), "Hello, world!");
}

#[test]
fn parse_chat_response_detects_bard_error_1100() {
    let body = r#"[["wrb.fr",null,null,null,null,[13,null,[["type.googleapis.com/assistant.boq.bard.application.BardErrorInfo",[1100]]]]]]"#;
    let result = parse_chat_response(body);
    assert!(result.is_err());
}

#[test]
fn extract_bard_error_code_parses_code() {
    let body = r#"[["wrb.fr",null,null,null,null,[13,null,[["type.googleapis.com/assistant.boq.bard.application.BardErrorInfo",[1096]]]]]]"#;
    assert_eq!(extract_bard_error_code(body), Some(1096));
}

#[test]
fn extract_conversation_state_reads_ids_and_token() {
    let body = r#"[["wrb.fr", null, "[null, [\"c_abc\", \"r_def\"], null, null, [[\"rcp_123\", [\"text\"]]]]"]]
[["wrb.fr", null, "[null,[null,\"r_def\"],{\"26\":\"token_value\"}]"]]"#;

    let state = extract_conversation_state(body).unwrap();
    assert_eq!(state.conversation_id, "c_abc");
    assert_eq!(state.response_id, "r_def");
    assert_eq!(state.continuation_token, "token_value");
}

#[test]
fn extract_conversation_state_reads_token_from_key_21() {
    let body = r#"[["wrb.fr", null, "[null, [\"c_abc\", \"r_def\"], null, null, [[\"rcp_123\", [\"text\"]]]]"]]
[["wrb.fr", null, "[null,[null,\"r_def\"],{\"21\":[\"token_value\"],\"44\":true}]"]]"#;

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
fn build_inner_req_list_has_97_slots() {
    let prepared = PreparedRequest {
        prompt: "hello".to_string(),
        inline_images: vec![],
        config: None,
        category: ModelCategory::Auto,
    };
    let inner = build_inner_req_list(&prepared, None, None, &[], "UUID");
    assert_eq!(inner.len(), 97);
}

#[test]
fn build_inner_req_list_with_attachments() {
    let prepared = PreparedRequest {
        prompt: "describe".to_string(),
        inline_images: vec![],
        config: None,
        category: ModelCategory::Fast,
    };
    let attachments = vec![WebAttachment {
        reference: "/contrib_service/ttl_1d/abc".to_string(),
        mime_type: "image/png".to_string(),
        filename: "test.png".to_string(),
    }];
    let inner = build_inner_req_list(&prepared, None, None, &attachments, "UUID");
    assert!(inner[0][3].is_array());
}

#[test]
fn build_inner_req_list_with_conversation_state() {
    let prepared = PreparedRequest {
        prompt: "follow up".to_string(),
        inline_images: vec![],
        config: None,
        category: ModelCategory::Pro,
    };
    let state = ConversationState {
        conversation_id: "c_abc".to_string(),
        response_id: "r_def".to_string(),
        response_part_id: "rcp_123".to_string(),
        continuation_token: "tok".to_string(),
    };
    let inner = build_inner_req_list(&prepared, Some(&state), None, &[], "UUID");
    assert_eq!(inner[17], serde_json::json!([[1]]));
    assert_eq!(inner[30], serde_json::json!([3]));
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
