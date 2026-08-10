//! Locale and model configuration helpers for the Gemini web frontend.
//!
//! These helpers expose typed access to four undocumented batchexecute RPCs
//! used during `/app` bootstrap:
//!
//! - `cYRIkd` — locale tools
//! - `whPPme` — model configuration
//! - `Te6DCf` — locale configuration
//! - `ku4Jyf` — tools configuration
//!
//! All responses are returned as opaque `serde_json::Value` wrappers so future
//! protocol drift does not break consumers.

use serde_json::Value;

use crate::errors::{Error, Result};
use crate::proto::{
    indices::parser::{PAYLOAD, PAYLOAD_ALT, RPC_ID},
    strip_xssi_prefix,
};

/// RPC id used for retrieving locale tools.
pub(crate) const CYRIKD_RPC_ID: &str = "cYRIkd";

/// RPC id used for retrieving model configuration.
pub(crate) const WHPPME_RPC_ID: &str = "whPPme";

/// RPC id used for retrieving locale configuration.
pub(crate) const TE6DCF_RPC_ID: &str = "Te6DCf";

/// RPC id used for retrieving tools configuration.
pub(crate) const KU4JYF_RPC_ID: &str = "ku4Jyf";

/// Locale tools returned by the `cYRIkd` batchexecute RPC.
///
/// The inner payload is intentionally exposed as a raw [`serde_json::Value`]
/// because the undocumented shape may drift over time.
#[derive(Debug, Clone, PartialEq)]
pub struct LocaleTools {
    value: Value,
}

impl LocaleTools {
    /// Returns the raw parsed payload.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// Model configuration returned by the `whPPme` batchexecute RPC.
///
/// The inner payload is intentionally exposed as a raw [`serde_json::Value`]
/// because the undocumented shape may drift over time.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelConfig {
    value: Value,
}

impl ModelConfig {
    /// Returns the raw parsed payload.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// Locale configuration returned by the `Te6DCf` batchexecute RPC.
///
/// The inner payload is intentionally exposed as a raw [`serde_json::Value`]
/// because the undocumented shape may drift over time.
#[derive(Debug, Clone, PartialEq)]
pub struct LocaleConfig {
    value: Value,
}

impl LocaleConfig {
    /// Returns the raw parsed payload.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// Tools configuration returned by the `ku4Jyf` batchexecute RPC.
///
/// The inner payload is intentionally exposed as a raw [`serde_json::Value`]
/// because the undocumented shape may drift over time.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolsConfig {
    value: Value,
}

impl ToolsConfig {
    /// Returns the raw parsed payload.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// Builds the inner payload for a `get_locale_tools()` request.
///
/// The captured shape is `["{language}"]`.
pub fn build_get_locale_tools_payload(language: &str) -> Value {
    serde_json::json!([language])
}

/// Builds the inner payload for a `get_model_config()` request.
///
/// The captured shape is `["{language}", null, [4]]`.
pub fn build_get_model_config_payload(language: &str) -> Value {
    serde_json::json!([language, null, [4]])
}

/// Builds the inner payload for a `get_locale_config()` request.
///
/// The captured shape is `[["{language}"], [1, 2]]`.
pub fn build_get_locale_config_payload(language: &str) -> Value {
    serde_json::json!([[language], [1, 2]])
}

/// Builds the inner payload for a `get_tools_config()` request.
///
/// The captured shape is
/// `["{language}", null, null, null, 4, null, null, [1, 3, 7, 17], null, []]`.
pub fn build_get_tools_config_payload(language: &str) -> Value {
    serde_json::json!([
        language,
        null,
        null,
        null,
        4,
        null,
        null,
        [1, 3, 7, 17],
        null,
        []
    ])
}

/// Parses the batchexecute response body returned by the `cYRIkd` RPC.
pub fn parse_locale_tools_response(body: &str) -> Result<LocaleTools> {
    let value = extract_inner_value(body, CYRIKD_RPC_ID)?;
    Ok(LocaleTools { value })
}

/// Parses the batchexecute response body returned by the `whPPme` RPC.
pub fn parse_model_config_response(body: &str) -> Result<ModelConfig> {
    let value = extract_inner_value(body, WHPPME_RPC_ID)?;
    Ok(ModelConfig { value })
}

/// Parses the batchexecute response body returned by the `Te6DCf` RPC.
pub fn parse_locale_config_response(body: &str) -> Result<LocaleConfig> {
    let value = extract_inner_value(body, TE6DCF_RPC_ID)?;
    Ok(LocaleConfig { value })
}

/// Parses the batchexecute response body returned by the `ku4Jyf` RPC.
pub fn parse_tools_config_response(body: &str) -> Result<ToolsConfig> {
    let value = extract_inner_value(body, KU4JYF_RPC_ID)?;
    Ok(ToolsConfig { value })
}

fn extract_inner_value(body: &str, rpc_id: &str) -> Result<Value> {
    let rpc_entry = extract_rpc_entry(body, rpc_id)?;
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
        entry
            .get(0)
            .and_then(|v| v.as_str())
            .map(|s| s == RPC_ID)
            .unwrap_or(false)
            && entry
                .get(1)
                .and_then(|v| v.as_str())
                .map(|s| s == rpc_id)
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
                .map(|s| s == rpc_id)
                .unwrap_or(false)
    })
}

fn extract_payload_str(entry: &Value) -> Result<&str> {
    entry
        .get(PAYLOAD)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            entry
                .get(PAYLOAD_ALT)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
        })
        .ok_or_else(|| Error::parse("response payload missing"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_locale_tools_payload_matches_captured_shape() {
        let payload = build_get_locale_tools_payload("ru");
        assert_eq!(payload, serde_json::json!(["ru"]));
    }

    #[test]
    fn get_model_config_payload_matches_captured_shape() {
        let payload = build_get_model_config_payload("ru");
        assert_eq!(payload, serde_json::json!(["ru", null, [4]]));
    }

    #[test]
    fn get_locale_config_payload_matches_captured_shape() {
        let payload = build_get_locale_config_payload("ru");
        assert_eq!(payload, serde_json::json!([["ru"], [1, 2]]));
    }

    #[test]
    fn get_tools_config_payload_matches_captured_shape() {
        let payload = build_get_tools_config_payload("ru");
        assert_eq!(
            payload,
            serde_json::json!(["ru", null, null, null, 4, null, null, [1, 3, 7, 17], null, []])
        );
    }

    #[test]
    fn parse_locale_tools_extracts_payload() {
        let body = r#")] } '

[["wrb.fr","cYRIkd","{\"tools\":[\"tool1\",\"tool2\"]}",null,null,null,"generic"]]"#;
        let result = parse_locale_tools_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({"tools": ["tool1", "tool2"]}));
    }

    #[test]
    fn parse_model_config_extracts_payload() {
        let body = r#")] } '

[["wrb.fr","whPPme","{\"models\":[{\"id\":\"pro\"}]}",null,null,null,"generic"]]"#;
        let result = parse_model_config_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({"models": [{"id": "pro"}]}));
    }

    #[test]
    fn parse_locale_config_extracts_payload() {
        let body = r#")] } '

[["wrb.fr","Te6DCf","{\"locale\":\"ru\"}",null,null,null,"generic"]]"#;
        let result = parse_locale_config_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({"locale": "ru"}));
    }

    #[test]
    fn parse_tools_config_extracts_payload() {
        let body = r#")] } '

[["wrb.fr","ku4Jyf","{\"enabled\":[1,3,7,17]}",null,null,null,"generic"]]"#;
        let result = parse_tools_config_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({"enabled": [1, 3, 7, 17]}));
    }

    #[test]
    fn parse_locale_tools_handles_wrapped_array() {
        let body = r#")] } '

[[["wrb.fr","cYRIkd","{\"tools\":[\"x\"]}",null,null,null,"generic"]]]"#;
        let result = parse_locale_tools_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({"tools": ["x"]}));
    }

    #[test]
    fn parse_model_config_handles_wrapped_array() {
        let body = r#")] } '

[[["wrb.fr","whPPme","{\"x\":1}",null,null,null,"generic"]]]"#;
        let result = parse_model_config_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({"x": 1}));
    }

    #[test]
    fn parse_locale_config_handles_wrapped_array() {
        let body = r#")] } '

[[["wrb.fr","Te6DCf","{\"y\":2}",null,null,null,"generic"]]]"#;
        let result = parse_locale_config_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({"y": 2}));
    }

    #[test]
    fn parse_tools_config_handles_wrapped_array() {
        let body = r#")] } '

[[["wrb.fr","ku4Jyf","{\"z\":3}",null,null,null,"generic"]]]"#;
        let result = parse_tools_config_response(body).unwrap();
        assert_eq!(result.value(), &serde_json::json!({"z": 3}));
    }
}
