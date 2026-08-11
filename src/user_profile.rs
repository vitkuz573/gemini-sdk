//! User profile and preference helpers for the Gemini web frontend.
//!
//! These helpers expose typed access to the signed-in user's identity
//! (`o30O0e`) and the last-selected mode preference (`L5adhe`). They are
//! intentionally thin wrappers around the existing batchexecute transport.

use serde_json::Value;

use crate::errors::{Error, Result};
use crate::proto::{
    indices::parser::{PAYLOAD, PAYLOAD_ALT, RPC_ID},
    strip_xssi_prefix,
};

/// RPC id used for retrieving the signed-in user's profile.
pub(crate) const O30O0E_RPC_ID: &str = "o30O0e";

/// RPC id used for reading and writing the last-selected mode preference.
pub(crate) const L5ADHE_RPC_ID: &str = "L5adhe";

/// Signed-in user identity returned by the `o30O0e` batchexecute RPC.
///
/// All fields are optional because the frontend omits or nulls entries for
/// accounts that have not shared them, or for partially-enrolled sessions.
/// Callers should avoid logging these values because they contain PII.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserInfo {
    name: Option<String>,
    photo_url: Option<String>,
    email: Option<String>,
}

impl UserInfo {
    /// Returns the user's display name, if present.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the user's profile photo URL, if present.
    #[must_use]
    pub fn photo_url(&self) -> Option<&str> {
        self.photo_url.as_deref()
    }

    /// Returns the user's email address, if present.
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
}

/// The last-selected Gemini mode returned by the `L5adhe` batchexecute RPC.
///
/// The value is exposed as an opaque string because the frontend treats mode
/// ids as opaque identifiers.
#[derive(Debug, Clone, PartialEq)]
pub struct LastSelectedMode {
    mode_id: Option<String>,
}

impl LastSelectedMode {
    /// Returns the last-selected mode id, if one is set.
    #[must_use]
    pub fn mode_id(&self) -> Option<&str> {
        self.mode_id.as_deref()
    }
}

/// Builds the inner payload for a `get_user_info()` request.
///
/// The captured shape is:
/// ```json
/// [["me"], [[["person.photo", "person.name", "person.email"], null, [1, 7]]]]
/// ```
pub fn build_get_user_info_payload() -> Value {
    serde_json::json!([["me"], [[["person.photo", "person.name", "person.email"], null, [1, 7]]]])
}

/// Builds the inner payload for a `get_last_selected_mode()` request.
///
/// The captured read shape is:
/// ```json
/// [[null, null, null, null, null, null, null, null], [["last_selected_mode_id_on_web"]]]
/// ```
pub fn build_get_last_selected_mode_payload(current_mode_id: Option<&str>) -> Value {
    build_last_selected_mode_payload_inner(current_mode_id)
}

/// Builds the inner payload for a `set_last_selected_mode()` request.
///
/// The captured write shape is:
/// ```json
/// [[null, null, null, null, null, null, null, "{mode_id}"], [["last_selected_mode_id_on_web"]]]
/// ```
pub fn build_set_last_selected_mode_payload(mode_id: &str) -> Value {
    build_last_selected_mode_payload_inner(Some(mode_id))
}

fn build_last_selected_mode_payload_inner(mode_id: Option<&str>) -> Value {
    let mut leading = vec![Value::Null; 7];
    leading.push(mode_id.map_or(Value::Null, |s| Value::String(s.to_string())));
    serde_json::json!([leading, [["last_selected_mode_id_on_web"]]])
}

/// Parses the batchexecute response body returned by the `o30O0e` RPC.
///
/// Missing or null fields are returned as `None` instead of producing an error.
/// When the payload is null/empty/missing entirely, an empty [`UserInfo`] is
/// returned so callers can continue without treating a bare identity RPC reply
/// as fatal.
pub fn parse_user_info_response(body: &str) -> Result<UserInfo> {
    let rpc_entry = extract_rpc_entry(body, O30O0E_RPC_ID)?;

    // The live frontend sometimes returns a bare `["wrb.fr","o30O0e",null,...]`
    // entry for sessions that do not expose profile data. Treat that as an
    // empty identity rather than a parse failure.
    let payload_value = rpc_entry.get(PAYLOAD).or_else(|| rpc_entry.get(PAYLOAD_ALT));
    if payload_value.map_or(true, |v| v.is_null()) {
        return Ok(UserInfo::default());
    }

    let payload_str = extract_payload_str(&rpc_entry)?;
    let inner: Value = serde_json::from_str(payload_str)
        .map_err(|e| Error::parse(format!("failed to parse o30O0e inner payload: {e}")))?;

    let name = read_optional_string(&inner, &["name"]);
    let photo_url = read_optional_string(&inner, &["photoUrl", "photo_url"]);
    let email = read_optional_string(&inner, &["email"]);

    Ok(UserInfo { name, photo_url, email })
}

/// Parses the batchexecute response body returned by the `L5adhe` RPC.
///
/// A non-string (including `null`) yields `LastSelectedMode { mode_id: None }`.
pub fn parse_last_selected_mode_response(body: &str) -> Result<LastSelectedMode> {
    let rpc_entry = extract_rpc_entry(body, L5ADHE_RPC_ID)?;

    // The live frontend sometimes returns a bare null payload when no mode
    // preference is stored. Treat that the same as an unset mode.
    let payload_value = rpc_entry.get(PAYLOAD).or_else(|| rpc_entry.get(PAYLOAD_ALT));
    if payload_value.map_or(true, |v| v.is_null()) {
        return Ok(LastSelectedMode { mode_id: None });
    }

    let payload_str = extract_payload_str(&rpc_entry)?;
    let inner: Value = serde_json::from_str(payload_str)
        .map_err(|e| Error::parse(format!("failed to parse L5adhe inner payload: {e}")))?;

    let mode_id = match inner {
        Value::String(s) if !s.is_empty() => Some(s),
        _ => None,
    };

    Ok(LastSelectedMode { mode_id })
}

fn extract_rpc_entry(body: &str, rpc_id: &str) -> Result<Value> {
    let payload = strip_xssi_prefix(body)
        .ok_or_else(|| Error::parse(format!("{rpc_id} response does not contain a JSON array")))?;

    let outer: Value = serde_json::from_str(payload)
        .map_err(|e| Error::parse(format!("failed to parse {rpc_id} JSON: {e}")))?;

    find_rpc_entry(&outer, rpc_id)
        .cloned()
        .ok_or_else(|| Error::parse(format!("{rpc_id} response does not contain {rpc_id} entry")))
}

fn find_rpc_entry<'a>(value: &'a Value, rpc_id: &str) -> Option<&'a Value> {
    let arr = value.as_array()?;
    let direct = arr.iter().find(|entry| {
        entry.get(0).and_then(|v| v.as_str()).map(|s| s == RPC_ID).unwrap_or(false)
            && entry.get(1).and_then(|v| v.as_str()).map(|s| s == rpc_id).unwrap_or(false)
    });
    if direct.is_some() {
        return direct;
    }
    let first = arr.first().and_then(|v| v.as_array())?;
    first.iter().find(|entry| {
        entry.get(0).and_then(|v| v.as_str()).map(|s| s == RPC_ID).unwrap_or(false)
            && entry.get(1).and_then(|v| v.as_str()).map(|s| s == rpc_id).unwrap_or(false)
    })
}

fn extract_payload_str(entry: &Value) -> Result<&str> {
    entry
        .get(PAYLOAD)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| entry.get(PAYLOAD_ALT).and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .ok_or_else(|| Error::parse("response payload missing"))
}

fn read_optional_string(value: &Value, keys: &[&str]) -> Option<String> {
    let obj = value.as_object()?;
    for key in keys {
        if let Some(v) = obj.get(*key) {
            if v.is_null() {
                return None;
            }
            return v.as_str().map(std::string::ToString::to_string);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_user_info_payload_matches_captured_shape() {
        let payload = build_get_user_info_payload();
        assert_eq!(
            payload,
            serde_json::json!([
                ["me"],
                [[["person.photo", "person.name", "person.email"], null, [1, 7]]]
            ])
        );
    }

    #[test]
    fn get_last_selected_mode_payload_is_all_nulls() {
        let payload = build_get_last_selected_mode_payload(None);
        assert_eq!(
            payload,
            serde_json::json!([
                [null, null, null, null, null, null, null, null],
                [["last_selected_mode_id_on_web"]]
            ])
        );
    }

    #[test]
    fn set_last_selected_mode_payload_puts_id_at_index_7() {
        let payload = build_set_last_selected_mode_payload("cf41b0e0dd7d53e5");
        assert_eq!(
            payload,
            serde_json::json!([
                [null, null, null, null, null, null, null, "cf41b0e0dd7d53e5"],
                [["last_selected_mode_id_on_web"]]
            ])
        );
    }

    #[test]
    fn parse_user_info_full_response() {
        let body = r#")] } '

[["wrb.fr","o30O0e","{\"name\":\"Jane Doe\",\"photoUrl\":\"https://example.com/photo.jpg\",\"email\":\"jane@example.com\"}",null,null,null,"generic"]]"#;
        let info = parse_user_info_response(body).unwrap();
        assert_eq!(info.name(), Some("Jane Doe"));
        assert_eq!(info.photo_url(), Some("https://example.com/photo.jpg"));
        assert_eq!(info.email(), Some("jane@example.com"));
    }

    #[test]
    fn parse_user_info_tolerates_missing_and_null_fields() {
        let body = r#")] } '

[["wrb.fr","o30O0e","{\"name\":\"Jane Doe\",\"email\":null}",null,null,null,"generic"]]"#;
        let info = parse_user_info_response(body).unwrap();
        assert_eq!(info.name(), Some("Jane Doe"));
        assert_eq!(info.photo_url(), None);
        assert_eq!(info.email(), None);
    }

    #[test]
    fn parse_user_info_accepts_snake_case_photo_url() {
        let body = r#")] } '

[["wrb.fr","o30O0e","{\"name\":\"Jane Doe\",\"photo_url\":\"https://example.com/photo.jpg\"}",null,null,null,"generic"]]"#;
        let info = parse_user_info_response(body).unwrap();
        assert_eq!(info.photo_url(), Some("https://example.com/photo.jpg"));
    }

    #[test]
    fn parse_user_info_handles_wrapped_array() {
        let body = r#")] } '\n\n[[["wrb.fr","o30O0e","{\"name\":\"Wrapped\"}",null,null,null,"generic"]]]"#;
        let info = parse_user_info_response(body).unwrap();
        assert_eq!(info.name(), Some("Wrapped"));
    }

    #[test]
    fn parse_user_info_null_payload_returns_default() {
        // Live sessions sometimes return a bare null payload with no identity
        // fields. This should yield an empty UserInfo instead of a parse error.
        let body = r#")] } '\n\n[["wrb.fr","o30O0e",null,null,null,[3],"generic"]]"#;
        let info = parse_user_info_response(body).unwrap();
        assert_eq!(info, UserInfo::default());
    }

    #[test]
    fn parse_last_selected_mode_null_payload_returns_none() {
        let body = r#")] } '\n\n[["wrb.fr","L5adhe",null,null,null,[7],"generic"]]"#;
        let mode = parse_last_selected_mode_response(body).unwrap();
        assert_eq!(mode.mode_id(), None);
    }

    #[test]
    fn parse_last_selected_mode_returns_string() {
        let body = r#")] } '

[["wrb.fr","L5adhe","\"cf41b0e0dd7d53e5\"",null,null,null,"generic"]]"#;
        let mode = parse_last_selected_mode_response(body).unwrap();
        assert_eq!(mode.mode_id(), Some("cf41b0e0dd7d53e5"));
    }

    #[test]
    fn parse_last_selected_mode_returns_none_for_null() {
        let body = r#")] } '

[["wrb.fr","L5adhe","null",null,null,null,"generic"]]"#;
        let mode = parse_last_selected_mode_response(body).unwrap();
        assert_eq!(mode.mode_id(), None);
    }

    #[test]
    fn parse_last_selected_mode_returns_none_for_empty_string() {
        let body = r#")] } '

[["wrb.fr","L5adhe","\"\"",null,null,null,"generic"]]"#;
        let mode = parse_last_selected_mode_response(body).unwrap();
        assert_eq!(mode.mode_id(), None);
    }
}
