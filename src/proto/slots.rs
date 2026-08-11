//! Construction of the 97-slot `inner_req_list` used by `StreamGenerate`.

use base64::Engine;
use serde_json::{json, Value};

use std::sync::Arc;

use crate::chat::{PreparedRequest, ThinkingLevel};
use crate::proto::indices::builder::*;
use crate::tool::Tool;

/// Number of slots in the `StreamGenerate` inner request list.
pub const SLOT_COUNT: usize = 97;

/// An uploaded file reference ready to be placed into `StreamGenerate` slot 0.
#[derive(Debug, Clone)]
pub struct WebAttachment {
    /// Google `contrib_service` reference path.
    pub reference: String,
    /// MIME type of the uploaded file.
    pub mime_type: String,
    /// File name sent to the upload endpoint.
    pub filename: String,
}

/// Multi-turn conversation state carried across `StreamGenerate` calls.
#[derive(Debug, Clone)]
pub struct ConversationState {
    /// Conversation identifier.
    pub conversation_id: String,
    /// Response identifier from the previous turn.
    pub response_id: String,
    /// Response part identifier.
    pub response_part_id: String,
    /// Continuation token for the next turn.
    pub continuation_token: String,
}

impl ConversationState {
    pub(crate) fn to_slot2(&self) -> Value {
        json!([
            self.conversation_id,
            self.response_id,
            self.response_part_id,
            null,
            null,
            null,
            null,
            null,
            null,
            self.continuation_token,
        ])
    }
}

/// Builds a fresh 97-slot request list.
///
/// When `browser_payload` is provided it is used as the base and only prompt,
/// category, and UUID slots are overridden.
// REASON: this function intentionally mirrors the 97-slot frontend layout;
// grouping all parameters in one builder avoids partial-construction errors.
#[allow(clippy::too_many_arguments)]
pub fn build_inner_req_list(
    request: &PreparedRequest,
    conversation_state: Option<&ConversationState>,
    browser_payload: Option<&[Value]>,
    attachments: &[WebAttachment],
    request_uuid: &str,
    language: &str,
    waa_token: Option<&str>,
    nonce: &str,
) -> Vec<Value> {
    let mut inner = match browser_payload {
        Some(payload) => normalize_payload(payload),
        None => build_fallback_base(conversation_state),
    };

    let system_instruction = request.config.as_ref().and_then(|c| c.system_instruction.as_deref());
    inner[SLOT_PROMPT] = build_slot0(&request.prompt, attachments, system_instruction);
    inner[SLOT_LANGUAGE] = json!([language]);
    inner[SLOT_WAA_TOKEN] = waa_token.map_or_else(|| Value::Null, |t| json!(t));
    inner[SLOT_NONCE] = json!(nonce);
    inner[SLOT_CATEGORY] = json!(1);
    inner[SLOT_REQUEST_UUID] = json!(1);
    inner[SLOT_FRESH_FLAG] = json!(0);
    inner[18] = json!(0);
    inner[27] = json!(1);
    inner[SLOT_REQUEST_CATEGORY] = json!([request.category.as_enum_value()]);
    inner[SLOT_THINKING_FLAG] = json!([1]);
    inner[53] = json!(0);
    inner[59] = json!(request_uuid);
    inner[61] = json!([]);
    inner[66] = Value::Null;
    inner[68] = json!(2);
    inner[79] = json!(3);
    inner[SLOT_THINKING_LEVEL] = json!(ThinkingLevel::Standard.as_enum_value().unwrap_or(1));
    inner[91] = json!(0);
    // Slot 96 is 1 for a fresh conversation and 0 when continuing an existing one.
    inner[SLOT_CONVERSATION_TYPE] = json!(if conversation_state.is_some() { 0 } else { 1 });

    if let Some(tools) = &request.tools {
        inner[SLOT_TOOL_DECLARATIONS] = build_tool_declarations(tools);
    }

    if browser_payload.is_none() {
        inner[SLOT_CONTINUATION_FLAG] = json!([1]);
    }

    if let Some(level) = request
        .config
        .as_ref()
        .and_then(|c| c.thinking_level)
        .and_then(ThinkingLevel::as_enum_value)
    {
        inner[SLOT_THINKING_LEVEL] = json!(level);
    }

    inner
}

fn normalize_payload(payload: &[Value]) -> Vec<Value> {
    let mut slots = payload.to_vec();
    match slots.len().cmp(&SLOT_COUNT) {
        std::cmp::Ordering::Less => slots.resize(SLOT_COUNT, Value::Null),
        std::cmp::Ordering::Greater => slots.truncate(SLOT_COUNT),
        std::cmp::Ordering::Equal => {}
    }
    slots
}

fn build_fallback_base(conversation_state: Option<&ConversationState>) -> Vec<Value> {
    let mut slots = vec![Value::Null; SLOT_COUNT];
    slots[SLOT_CONVERSATION_STATE] = match conversation_state {
        Some(state) => state.to_slot2(),
        None => Value::Array(vec![
            json!(""),
            json!(""),
            json!(""),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            json!(""),
        ]),
    };
    slots[SLOT_WAA_TOKEN] = Value::Null;
    slots[SLOT_NONCE] = json!("");
    slots[17] = if conversation_state.is_some() {
        json!([[1]])
    } else {
        json!([[0]])
    };
    slots
}

fn build_slot0(
    prompt: &str,
    attachments: &[WebAttachment],
    system_instruction: Option<&str>,
) -> Value {
    let prompt = match system_instruction {
        Some(instruction) => format!("{instruction}\n{prompt}"),
        None => prompt.to_string(),
    };
    if attachments.is_empty() {
        json!([prompt, 0, null, null, null, null, 0])
    } else {
        let attachment_list: Vec<Value> = attachments
            .iter()
            .map(|att| {
                json!([
                    [att.reference.clone(), 1, null, att.mime_type.clone()],
                    att.filename.clone(),
                    null,
                    null,
                    null,
                    null,
                    null,
                    null,
                    [0]
                ])
            })
            .collect();
        json!([prompt, 0, null, attachment_list, null, null, 0])
    }
}

/// Derives a file name for an uploaded attachment from its MIME type.
pub fn derive_attachment_filename(mime_type: &str, index: usize) -> String {
    let ext = match mime_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "application/pdf" => "pdf",
        "audio/mp3" | "audio/mpeg" => "mp3",
        "audio/wav" => "wav",
        "audio/ogg" => "ogg",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/quicktime" => "mov",
        _ => {
            let clean = mime_type.split(';').next().unwrap_or(mime_type);
            clean.split('/').nth(1).unwrap_or("bin")
        }
    };
    if index == 0 {
        format!("attachment.{ext}")
    } else {
        format!("attachment_{index}.{ext}")
    }
}

/// Builds a JSON array of tool declarations from registered tools.
fn build_tool_declarations(tools: &[Arc<dyn Tool>]) -> Value {
    let declarations: Vec<Value> = tools
        .iter()
        .map(|tool| {
            serde_json::json!({
                "name": tool.name(),
                "parameters": tool.schema(),
            })
        })
        .collect();
    serde_json::json!([declarations])
}

/// Decodes base64 data tolerating common whitespace.
pub fn base64_decode(data: &str) -> crate::Result<Vec<u8>> {
    let stripped: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD
        .decode(&stripped)
        .map_err(|e| crate::errors::Error::bad_request(format!("invalid base64 data: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModelCategory;

    fn minimal_prepared() -> PreparedRequest {
        PreparedRequest {
            prompt: "hello".to_string(),
            inline_images: vec![],
            inline_audio: vec![],
            inline_video: vec![],
            config: None,
            category: ModelCategory::Auto,
            tools: None,
            refresh_on_auth_error: false,
        }
    }

    #[test]
    fn slot_count_is_97() {
        let req = minimal_prepared();
        let inner = build_inner_req_list(&req, None, None, &[], "UUID", "en", None, "nonce");
        assert_eq!(inner.len(), SLOT_COUNT);
    }

    #[test]
    fn slot_1_uses_language() {
        let req = minimal_prepared();
        let inner = build_inner_req_list(&req, None, None, &[], "UUID", "ru", None, "nonce");
        assert_eq!(inner[1], json!(["ru"]));
    }

    #[test]
    fn slot_2_empty_conversation_is_single_array() {
        let req = minimal_prepared();
        let inner = build_inner_req_list(&req, None, None, &[], "UUID", "en", None, "nonce");
        assert!(inner[2].is_array());
        let slot2 = inner[2].as_array().expect("slot 2 is an array");
        assert!(!slot2[0].is_array());
    }

    #[test]
    fn slot_3_waa_token_or_null() {
        let req = minimal_prepared();
        let inner_no_waa = build_inner_req_list(&req, None, None, &[], "UUID", "en", None, "nonce");
        assert_eq!(inner_no_waa[3], Value::Null);
        let inner_waa =
            build_inner_req_list(&req, None, None, &[], "UUID", "en", Some("tok"), "nonce");
        assert_eq!(inner_waa[3], json!("tok"));
    }

    #[test]
    fn slot_41_68_79_defaults() {
        let req = minimal_prepared();
        let inner = build_inner_req_list(&req, None, None, &[], "UUID", "en", None, "nonce");
        assert_eq!(inner[41], json!([1]));
        assert_eq!(inner[68], json!(2));
        assert_eq!(inner[79], json!(3));
        assert_eq!(inner[80], json!(1));
        assert_eq!(inner[66], Value::Null);
    }

    #[test]
    fn slot_96_is_fresh_for_new_conversation() {
        let req = minimal_prepared();
        let inner = build_inner_req_list(&req, None, None, &[], "UUID", "en", None, "nonce");
        assert_eq!(inner[96], json!(1));
    }

    #[test]
    fn slot_96_is_zero_for_continuation() {
        let req = minimal_prepared();
        let state = ConversationState {
            conversation_id: "c_abc".to_string(),
            response_id: "r_def".to_string(),
            response_part_id: "rcp_123".to_string(),
            continuation_token: "tok".to_string(),
        };
        let inner =
            build_inner_req_list(&req, Some(&state), None, &[], "UUID", "en", None, "nonce");
        assert_eq!(inner[96], json!(0));
    }

    #[test]
    fn slot_30_reflects_category() {
        let mut req = minimal_prepared();
        req.category = ModelCategory::Pro;
        let inner = build_inner_req_list(&req, None, None, &[], "UUID", "en", None, "nonce");
        assert_eq!(inner[30], json!([3]));
    }

    #[test]
    fn slot_0_with_attachments() {
        let req = minimal_prepared();
        let attachments = vec![WebAttachment {
            reference: "/contrib_service/ttl_1d/abc".to_string(),
            mime_type: "image/png".to_string(),
            filename: "test.png".to_string(),
        }];
        let inner =
            build_inner_req_list(&req, None, None, &attachments, "UUID", "en", None, "nonce");
        let slot0 = &inner[0];
        assert!(slot0[3].is_array());
    }

    #[test]
    fn derive_attachment_filename_defaults() {
        assert_eq!(derive_attachment_filename("image/png", 0), "attachment.png");
        assert_eq!(derive_attachment_filename("image/jpeg", 0), "attachment.jpg");
        assert_eq!(derive_attachment_filename("application/pdf", 1), "attachment_1.pdf");
        assert_eq!(derive_attachment_filename("text/plain", 0), "attachment.plain");
    }

    #[test]
    fn derive_attachment_filename_audio_video() {
        assert_eq!(derive_attachment_filename("audio/mp3", 0), "attachment.mp3");
        assert_eq!(derive_attachment_filename("audio/mpeg", 0), "attachment.mp3");
        assert_eq!(derive_attachment_filename("audio/wav", 0), "attachment.wav");
        assert_eq!(derive_attachment_filename("audio/ogg", 0), "attachment.ogg");
        assert_eq!(derive_attachment_filename("video/mp4", 1), "attachment_1.mp4");
        assert_eq!(derive_attachment_filename("video/webm", 2), "attachment_2.webm");
        assert_eq!(derive_attachment_filename("video/quicktime", 0), "attachment.mov");
    }

    #[tokio::test]
    async fn slot_89_contains_tool_declarations() {
        use crate::tool::Tool;
        use serde_json::Value;
        use std::future::Future;
        use std::pin::Pin;

        struct FakeTool;
        impl Tool for FakeTool {
            fn name(&self) -> &str {
                "fake_tool"
            }
            fn schema(&self) -> Value {
                serde_json::json!({"type": "object"})
            }
            fn invoke(
                &self,
                _args: Value,
            ) -> Pin<Box<dyn Future<Output = Result<Value, crate::tool::ToolError>> + Send + '_>>
            {
                Box::pin(async move { Ok(Value::Null) })
            }
        }

        let mut req = minimal_prepared();
        req.tools = Some(vec![std::sync::Arc::new(FakeTool)]);
        let inner = build_inner_req_list(&req, None, None, &[], "UUID", "en", None, "nonce");
        let slot89 = &inner[SLOT_TOOL_DECLARATIONS];
        assert!(slot89.is_array());
        let arr = slot89.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        let declarations = arr[0].as_array().expect("wrapped declarations array");
        assert_eq!(declarations.len(), 1);
        let first = &declarations[0];
        assert_eq!(first["name"], "fake_tool");
    }

    #[test]
    fn no_tools_leaves_slot_89_null() {
        let req = minimal_prepared();
        let inner = build_inner_req_list(&req, None, None, &[], "UUID", "en", None, "nonce");
        assert_eq!(inner[SLOT_TOOL_DECLARATIONS], Value::Null);
    }
}
