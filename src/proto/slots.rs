//! Construction of the 97-slot `inner_req_list` used by `StreamGenerate`.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde_json::{json, Value};

use crate::chat::{PreparedRequest, ThinkingLevel};


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
pub fn build_inner_req_list(
    request: &PreparedRequest,
    conversation_state: Option<&ConversationState>,
    browser_payload: Option<&[Value]>,
    attachments: &[WebAttachment],
    request_uuid: &str,
) -> Vec<Value> {
    let mut inner = match browser_payload {
        Some(payload) => normalize_payload(payload),
        None => build_fallback_base(conversation_state),
    };

    inner[0] = build_slot0(&request.prompt, attachments);
    inner[1] = json!(["en"]);
    inner[7] = json!(1);
    inner[10] = json!(1);
    inner[11] = json!(0);
    inner[18] = json!(0);
    inner[27] = json!(1);
    inner[30] = json!([request.category.as_enum_value()]);
    inner[41] = json!([2]);
    inner[53] = json!(0);
    inner[59] = json!(request_uuid);
    inner[61] = json!([]);
    inner[68] = json!(1);
    inner[79] = json!(6);
    inner[91] = json!(0);
    inner[96] = json!(0);

    if browser_payload.is_none() {
        inner[6] = json!([1]);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        inner[66] = json!([ts, 0]);
    }

    if let Some(level) = request
        .config
        .as_ref()
        .and_then(|c| c.thinking_level)
        .and_then(ThinkingLevel::as_enum_value)
    {
        inner[80] = json!(level);
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
    slots[2] = match conversation_state {
        Some(state) => state.to_slot2(),
        None => json![["", "", "", null, null, null, null, null, null, ""]],
    };
    slots[3] = json!("");
    slots[4] = json!("");
    slots[17] = if conversation_state.is_some() {
        json!([[1]])
    } else {
        json!([[0]])
    };
    slots
}

fn build_slot0(prompt: &str, attachments: &[WebAttachment]) -> Value {
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
            config: None,
            category: ModelCategory::Auto,
        }
    }

    #[test]
    fn slot_count_is_97() {
        let req = minimal_prepared();
        let inner = build_inner_req_list(&req, None, None, &[], "UUID");
        assert_eq!(inner.len(), SLOT_COUNT);
    }

    #[test]
    fn slot_30_reflects_category() {
        let mut req = minimal_prepared();
        req.category = ModelCategory::Pro;
        let inner = build_inner_req_list(&req, None, None, &[], "UUID");
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
        let inner = build_inner_req_list(&req, None, None, &attachments, "UUID");
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
}
