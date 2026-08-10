//! Conversation action helpers for the Gemini web frontend.
//!
//! These actions allow programmatic control over conversation history:
//! regenerating a model response, rating a response, or deleting a turn.
//! All actions are implemented as thin wrappers around the `PCck7e`
//! batchexecute RPC.

use serde_json::Value;

use crate::errors::{Error, Result};
use crate::proto::{
    indices::parser::{PAYLOAD, PAYLOAD_ALT, RPC_ID},
    strip_xssi_prefix,
};

/// RPC id used for conversation actions in the Gemini web frontend.
pub(crate) const PCCK7E_RPC_ID: &str = "PCck7e";

/// An action that can be performed on a conversation turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConversationAction {
    /// Regenerate the model response for a turn.
    Regenerate,
    /// Rate a model response.
    Rate(TurnRating),
    /// Delete a turn from the conversation.
    Delete,
}

/// Rating applied to a model response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TurnRating {
    /// Positive rating (thumbs up).
    Good,
    /// Negative rating (thumbs down).
    Bad,
    /// Neutral / remove rating.
    Neutral,
}

impl TurnRating {
    /// Returns the wire value used in the `PCck7e` payload.
    fn to_wire_value(self) -> Value {
        match self {
            TurnRating::Good => Value::Number(1.into()),
            TurnRating::Bad => Value::Number(0.into()),
            TurnRating::Neutral => Value::Null,
        }
    }
}

/// Result of a conversation action.
#[derive(Debug, Clone, PartialEq)]
pub struct ConversationActionResult {
    success: bool,
    action: ConversationAction,
    response_id: String,
    raw: Value,
}

impl ConversationActionResult {
    /// Parses a raw batchexecute response body into a typed result.
    ///
    /// This is primarily useful for tests and custom response handling; most
    /// callers should use the high-level client methods instead.
    pub fn parse_response(
        body: &str,
        action: ConversationAction,
        response_id: String,
    ) -> Result<Self> {
        parse_conversation_action_response(body, action, response_id)
    }

    /// Whether the action was accepted by the server.
    pub fn success(&self) -> bool {
        self.success
    }

    /// The action that was performed.
    pub fn action(&self) -> ConversationAction {
        self.action
    }

    /// The response id the action targeted.
    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    /// The raw parsed payload returned by the server.
    ///
    /// Callers should avoid logging this value because it may contain
    /// conversation content or other sensitive data.
    pub fn raw(&self) -> &Value {
        &self.raw
    }
}

/// Builds the inner payload for a regenerate action.
///
/// The captured shape is `["r_{response_id}"]`. The batchexecute transport
/// already wraps this inner value inside its own array, so the builder must
/// return a single array — not a nested one.
pub(crate) fn build_regenerate_payload(response_id: &str) -> Value {
    serde_json::json!([normalize_response_id(response_id)])
}

/// Builds the inner payload for a rating action.
///
/// The captured shape is `["r_{response_id}", {rating_value}]` where the
/// rating value is `1` for [`TurnRating::Good`], `0` for [`TurnRating::Bad`],
/// and `null` for [`TurnRating::Neutral`].
pub(crate) fn build_rate_payload(response_id: &str, rating: TurnRating) -> Value {
    serde_json::json!([normalize_response_id(response_id), rating.to_wire_value()])
}

/// Builds the inner payload for a delete action.
///
/// The captured shape is `["r_{response_id}"]`.
pub(crate) fn build_delete_payload(response_id: &str) -> Value {
    serde_json::json!([normalize_response_id(response_id)])
}

/// Ensures a response id starts with the `r_` prefix used by the frontend.
fn normalize_response_id(response_id: &str) -> String {
    let trimmed = response_id.trim();
    if trimmed.starts_with("r_") {
        trimmed.to_string()
    } else {
        format!("r_{trimmed}")
    }
}

/// Parses the response to a `PCck7e` batchexecute call.
///
/// The parser is intentionally permissive: any payload that is not a clear
/// error object is treated as success, because the undocumented RPC can
/// return a variety of confirmation shapes.
pub fn parse_conversation_action_response(
    body: &str,
    action: ConversationAction,
    response_id: String,
) -> Result<ConversationActionResult> {
    let payload = strip_xssi_prefix(body)
        .ok_or_else(|| Error::parse("PCck7e response does not contain a JSON array"))?;

    let outer: Value = serde_json::from_str(payload)
        .map_err(|e| Error::parse(format!("failed to parse PCck7e JSON: {e}")))?;

    let rpc_entry = find_rpc_entry(&outer)
        .ok_or_else(|| Error::parse("PCck7e response does not contain PCck7e entry"))?
        .clone();

    let payload_value = rpc_entry
        .get(PAYLOAD)
        .or_else(|| rpc_entry.get(PAYLOAD_ALT))
        .cloned()
        .ok_or_else(|| Error::parse("PCck7e response payload missing"))?;

    // Treat null and the string forms "null", "[]", and "\"[]\"" as successful
    // no-content responses. The server returns these when the action is accepted
    // but carries no payload.
    let success = if payload_value.is_null() {
        true
    } else if let Some(s) = payload_value.as_str() {
        let trimmed = s.trim();
        if trimmed.is_empty()
            || trimmed.eq_ignore_ascii_case("null")
            || trimmed == "[]"
            || trimmed == "\"[]\""
        {
            true
        } else {
            let parsed: Value = serde_json::from_str(s)
                .map_err(|e| Error::parse(format!("failed to parse PCck7e inner payload: {e}")))?;
            !is_error_payload(&parsed)
        }
    } else {
        !is_error_payload(&payload_value)
    };

    Ok(ConversationActionResult {
        success,
        action,
        response_id,
        raw: payload_value,
    })
}

/// Searches the batchexecute outer array for the `PCck7e` entry, tolerating
/// an extra wrapping level.
fn find_rpc_entry(value: &Value) -> Option<&Value> {
    let arr = value.as_array()?;
    let direct = arr.iter().find(|entry| {
        entry
            .get(0)
            .and_then(|v| v.as_str())
            .map(|s| s == RPC_ID)
            .unwrap_or(false)
            && entry
                .get(1)
                .and_then(|v| v.as_str())
                .map(|s| s == PCCK7E_RPC_ID)
                .unwrap_or(false)
    });
    if direct.is_some() {
        return direct;
    }
    let first = arr.first().and_then(|v| v.as_array())?;
    first.iter().find(|entry| {
        entry
            .get(0)
            .and_then(|v| v.as_str())
            .map(|s| s == RPC_ID)
            .unwrap_or(false)
            && entry
                .get(1)
                .and_then(|v| v.as_str())
                .map(|s| s == PCCK7E_RPC_ID)
                .unwrap_or(false)
    })
}

/// Returns true when the inner payload clearly represents an error.
fn is_error_payload(value: &Value) -> bool {
    if let Some(s) = value.as_str() {
        return s.to_ascii_lowercase().starts_with("error");
    }
    if let Some(obj) = value.as_object() {
        if let Some(error) = obj.get("error") {
            return !error.is_null();
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_response_id_adds_prefix() {
        assert_eq!(normalize_response_id("abc"), "r_abc");
        assert_eq!(normalize_response_id("r_abc"), "r_abc");
        assert_eq!(normalize_response_id("  r_abc  "), "r_abc");
    }

    #[test]
    fn payload_builders_match_expected_shape() {
        assert_eq!(build_regenerate_payload("abc"), serde_json::json!(["r_abc"]));
        assert_eq!(
            build_rate_payload("abc", TurnRating::Good),
            serde_json::json!(["r_abc", 1])
        );
        assert_eq!(
            build_rate_payload("abc", TurnRating::Bad),
            serde_json::json!(["r_abc", 0])
        );
        assert_eq!(
            build_rate_payload("abc", TurnRating::Neutral),
            serde_json::json!(["r_abc", null])
        );
        assert_eq!(build_delete_payload("abc"), serde_json::json!(["r_abc"]));
    }

    #[test]
    fn parse_success_response() {
        let body = ")] } ' \n\n[[\"wrb.fr\",\"PCck7e\",\"[1]\",null,null,null,\"generic\"]]";
        let result = parse_conversation_action_response(body, ConversationAction::Regenerate, "r_abc".into())
            .unwrap();
        assert!(result.success());
        assert_eq!(result.action(), ConversationAction::Regenerate);
        assert_eq!(result.response_id(), "r_abc");
    }

    #[test]
    fn parse_error_response() {
        let body = ")] } ' \n\n[[\"wrb.fr\",\"PCck7e\",\"{\\\"error\\\":\\\"turn not found\\\"}\",null,null,null,\"generic\"]]";
        let result = parse_conversation_action_response(body, ConversationAction::Delete, "r_abc".into())
            .unwrap();
        assert!(!result.success());
    }

    #[test]
    fn parse_wrapped_array() {
        let body = ")] } ' \n\n[[[\"wrb.fr\",\"PCck7e\",\"[1]\",null,null,null,\"generic\"]]]";
        let result = parse_conversation_action_response(body, ConversationAction::Rate(TurnRating::Good), "r_abc".into())
            .unwrap();
        assert!(result.success());
    }

    #[test]
    fn parse_null_payload_as_success() {
        let body = ")] } ' \n\n[[\"wrb.fr\",\"PCck7e\",null,null,null,null,\"generic\"]]";
        let result = parse_conversation_action_response(body, ConversationAction::Regenerate, "r_abc".into())
            .unwrap();
        assert!(result.success());
    }

    #[test]
    fn parse_empty_array_payload_as_success() {
        let body = ")] } ' \n\n[[\"wrb.fr\",\"PCck7e\",\"[]\",null,null,null,\"generic\"]]";
        let result = parse_conversation_action_response(body, ConversationAction::Delete, "r_abc".into())
            .unwrap();
        assert!(result.success());
    }

    #[test]
    fn parse_quoted_empty_array_payload_as_success() {
        let body = ")] } ' \n\n[[\"wrb.fr\",\"PCck7e\",\"\\\"[]\\\"\",null,null,null,\"generic\"]]";
        let result = parse_conversation_action_response(body, ConversationAction::Rate(TurnRating::Neutral), "r_abc".into())
            .unwrap();
        assert!(result.success());
    }

    #[test]
    fn parse_string_null_payload_as_success() {
        let body = ")] } ' \n\n[[\"wrb.fr\",\"PCck7e\",\"null\",null,null,null,\"generic\"]]";
        let result = parse_conversation_action_response(body, ConversationAction::Regenerate, "r_abc".into())
            .unwrap();
        assert!(result.success());
    }
}
