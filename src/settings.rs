//! Settings-page helpers for the Gemini web frontend.
//!
//! These helpers expose typed access to two undocumented batchexecute RPCs
//! used on the settings pages:
//!
//! - `jSf9Qc` — usage statistics
//! - `XPSWpd` — scheduled prompts
//!
//! All responses are returned as opaque `serde_json::Value` wrappers so future
//! protocol drift does not break consumers.

use serde_json::Value;

use crate::errors::{Error, Result};
use crate::proto::{
    indices::parser::{PAYLOAD, PAYLOAD_ALT, RPC_ID},
    strip_xssi_prefix,
};

/// RPC id used for retrieving usage statistics.
pub(crate) const JSF9QC_RPC_ID: &str = "jSf9Qc";

/// RPC id used for retrieving scheduled prompts.
pub(crate) const XPSWPD_RPC_ID: &str = "XPSWpd";

/// Usage statistics returned by the `jSf9Qc` batchexecute RPC.
///
/// The inner payload is intentionally exposed as a raw [`serde_json::Value`]
/// because the undocumented shape may drift over time.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageStats {
    value: Value,
}

impl UsageStats {
    /// Returns the raw parsed payload.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// Scheduled prompts returned by the `XPSWpd` batchexecute RPC.
///
/// The inner payload is intentionally exposed as a raw [`serde_json::Value`]
/// because the undocumented shape may drift over time.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledPrompts {
    value: Value,
}

impl ScheduledPrompts {
    /// Returns the raw parsed payload.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// Builds the inner payload for a `get_usage_stats()` request.
///
/// The captured shape is `[]`.
pub fn build_get_usage_stats_payload() -> Value {
    serde_json::json!([])
}

/// Builds the inner payload for a `get_scheduled_prompts()` request.
///
/// The captured shape is `[]`.
pub fn build_get_scheduled_prompts_payload() -> Value {
    serde_json::json!([])
}

/// Parses the batchexecute response body returned by the `jSf9Qc` RPC.
pub fn parse_usage_stats_response(body: &str) -> Result<UsageStats> {
    let value = extract_inner_value(body, JSF9QC_RPC_ID)?;
    Ok(UsageStats { value })
}

/// Parses the batchexecute response body returned by the `XPSWpd` RPC.
pub fn parse_scheduled_prompts_response(body: &str) -> Result<ScheduledPrompts> {
    let value = extract_inner_value(body, XPSWPD_RPC_ID)?;
    Ok(ScheduledPrompts { value })
}

fn extract_inner_value(body: &str, rpc_id: &str) -> Result<Value> {
    let rpc_entry = extract_rpc_entry(body, rpc_id)?;

    // The live frontend sometimes returns a bare null payload for these
    // settings RPCs when the account has no data to expose. Treat that as an
    // empty JSON object so callers can continue.
    let payload_value = rpc_entry.get(PAYLOAD).or_else(|| rpc_entry.get(PAYLOAD_ALT));
    if payload_value.map_or(true, |v| v.is_null()) {
        return Ok(Value::Object(serde_json::Map::new()));
    }

    let payload_str = extract_payload_str(&rpc_entry)?;
    serde_json::from_str(payload_str)
        .map_err(|e| Error::parse(format!("failed to parse {rpc_id} inner payload: {e}")))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_usage_stats_payload_matches_captured_shape() {
        let payload = build_get_usage_stats_payload();
        assert_eq!(payload, serde_json::json!([]));
    }

    #[test]
    fn get_scheduled_prompts_payload_matches_captured_shape() {
        let payload = build_get_scheduled_prompts_payload();
        assert_eq!(payload, serde_json::json!([]));
    }

    #[test]
    fn parse_usage_stats_null_payload_returns_empty_object() {
        let body = r#")] } '\n\n[["wrb.fr","jSf9Qc",null,null,null,[7],"generic"]]"#;
        let result = parse_usage_stats_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({}));
    }

    #[test]
    fn parse_usage_stats_extracts_payload() {
        let body = r#")] } '

[["wrb.fr","jSf9Qc","{\"requests_today\":12,\"requests_total\":345}",null,null,null,"generic"]]"#;
        let result = parse_usage_stats_response(body).unwrap();
        assert_eq!(
            result.value(),
            &serde_json::json!({"requests_today": 12, "requests_total": 345})
        );
    }

    #[test]
    fn parse_scheduled_prompts_null_payload_returns_empty_object() {
        let body = r#")] } '\n\n[["wrb.fr","XPSWpd",null,null,null,[7],"generic"]]"#;
        let result = parse_scheduled_prompts_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({}));
    }

    #[test]
    fn parse_scheduled_prompts_extracts_payload() {
        let body = r#")] } '

[["wrb.fr","XPSWpd","{\"prompts\":[{\"id\":\"sp_1\",\"text\":\"Morning summary\"}]}",null,null,null,"generic"]]"#;
        let result = parse_scheduled_prompts_response(body).unwrap();
        assert_eq!(
            result.value(),
            &serde_json::json!({"prompts": [{"id": "sp_1", "text": "Morning summary"}]})
        );
    }

    #[test]
    fn parse_usage_stats_handles_wrapped_array() {
        let body = r#")] } '

[[["wrb.fr","jSf9Qc","{\"x\":1}",null,null,null,"generic"]]]"#;
        let result = parse_usage_stats_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({"x": 1}));
    }

    #[test]
    fn parse_scheduled_prompts_handles_wrapped_array() {
        let body = r#")] } '

[[["wrb.fr","XPSWpd","{\"y\":2}",null,null,null,"generic"]]]"#;
        let result = parse_scheduled_prompts_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({"y": 2}));
    }
}
