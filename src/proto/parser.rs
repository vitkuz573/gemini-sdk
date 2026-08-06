//! Response parsing for batchexecute and `StreamGenerate` WIZ frames.

use serde_json::Value;

use crate::chat::{ChatResponse, ContentPart};
use crate::errors::{Error, Result};
use crate::models::ModelInfo;
use crate::proto::slots::ConversationState;

/// Re-export of [`parse_chat_response`] for the crate root.
pub use parse_chat_response as parse_chat_response_fn;

/// Parses a `GetUserStatus` batchexecute response into a list of model infos.
pub fn parse_model_list(body: &str) -> Result<Vec<ModelInfo>> {
    let payload = crate::proto::strip_xssi_prefix(body).ok_or_else(|| {
        Error::parse("GetUserStatus response does not contain a JSON array")
    })?;

    let outer: Value = serde_json::from_str(payload).map_err(|e| {
        Error::parse(format!("failed to parse GetUserStatus JSON: {e}"))
    })?;

    // Batchexecute can return the RPC entry either directly as the only
    // element of the outer array, or nested one level deeper. Accept both
    // shapes by looking for the first array that contains an `otAQ7b`
    // marker (the batchexecute response can be wrapped in extra arrays
    // depending on the request format).
    fn find_rpc_entry(value: &Value) -> Option<&Value> {
        if let Some(arr) = value.as_array() {
            if let Some(entry) = arr.iter().find(|entry| {
                entry
                    .get(1)
                    .and_then(|v| v.as_str())
                    .map(|s| s == "otAQ7b")
                    .unwrap_or(false)
            }) {
                return Some(entry);
            }
            // No direct match: try the first element if it is itself an
            // array (extra wrapping level).
            if let Some(first) = arr.first().and_then(|v| v.as_array()) {
                return first.iter().find(|entry| {
                    entry
                        .get(1)
                        .and_then(|v| v.as_str())
                        .map(|s| s == "otAQ7b")
                        .unwrap_or(false)
                });
            }
        }
        None
    }

    let rpc_entry = find_rpc_entry(&outer).ok_or_else(|| {
        Error::parse("GetUserStatus response does not contain otAQ7b entry")
    })?;

    // The inner payload is a JSON string at index 2 or 3 depending on the
    // response shape (index 2 is the canonical location for batchexecute
    // RPC replies; some responses place it at index 3).
    let payload_str = rpc_entry
        .get(2)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| rpc_entry.get(3).and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .ok_or_else(|| Error::parse("GetUserStatus response payload missing"))?;

    let inner: Value = serde_json::from_str(payload_str).map_err(|e| {
        Error::parse(format!("failed to parse GetUserStatus inner payload: {e}"))
    })?;

    let modes = inner
        .get(15)
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::parse("GetUserStatus inner payload does not contain mode list"))?;

    let mut result = Vec::with_capacity(modes.len());
    for mode in modes {
        let Some(mode_arr) = mode.as_array() else {
            continue;
        };
        if mode_arr.is_empty() {
            continue;
        }

        let id = mode_arr
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            continue;
        }

        let title = mode_arr
            .get(1)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let description = mode_arr
            .get(2)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let versioned_name = mode_arr
            .get(11)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| mode_arr.get(19).and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
            .map(|s| s.to_string());

        let category_enum = mode_arr
            .get(17)
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| crate::models::derive_category(&id, &title).as_enum_value());

        let category = crate::models::ModelCategory::from_enum_value(category_enum)
            .unwrap_or(crate::models::ModelCategory::Auto);

        result.push(ModelInfo {
            id,
            title,
            description,
            versioned_name,
            category,
            category_enum,
        });
    }

    if result.is_empty() {
        return Err(Error::parse("GetUserStatus returned empty model list"));
    }

    Ok(result)
}

/// Parses a non-streaming `StreamGenerate` response into a [`ChatResponse`].
pub fn parse_chat_response(body: &str) -> Result<ChatResponse> {
    let parts = parse_response_parts(body)?;
    let mut texts = Vec::new();

    for part in parts {
        match part {
            ContentPart::Text(t) => texts.push(t),
            ContentPart::Image(_) => {}
        }
    }

    let text = texts.join("");

    if text.is_empty() {
        if let Some(code) = extract_bard_error_code(body) {
            return Err(Error::Api {
                status: reqwest::StatusCode::BAD_REQUEST,
                message: format!("Gemini returned BardErrorInfo [{code}]"),
            });
        }
    }

    Ok(ChatResponse::new(text))
}

/// Extracts multi-turn conversation state from a raw `StreamGenerate` response.
pub fn extract_conversation_state(body: &str) -> Result<ConversationState> {
    let mut main_entry: Option<Value> = None;
    let mut continuation_token: Option<String> = None;

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            continue;
        }
        let entry: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let mut entry_arr = match entry.as_array() {
            Some(a) => a,
            None => continue,
        };
        if entry_arr.len() == 1 {
            if let Some(inner) = entry_arr.first().and_then(|v| v.as_array()) {
                entry_arr = inner;
            } else {
                continue;
            }
        }
        let rpc_id = match entry_arr.first().and_then(|v| v.as_str()) {
            Some(s) => s,
            None => continue,
        };
        if rpc_id != "wrb.fr" {
            continue;
        }
        let payload_str = match entry_arr.get(2).and_then(|v| v.as_str()) {
            Some(s) => s,
            None => continue,
        };
        let payload: Value = match serde_json::from_str(payload_str) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let payload_arr = match payload.as_array() {
            Some(a) => a,
            None => continue,
        };

        if payload_arr.len() == 3 {
            if let Some(second) = payload_arr.get(1).and_then(|v| v.as_array()) {
                // Meta entry shape: [null, [null, <r_id>], {"<n>": <token>, ...}]
                // The continuation token may live at key "26" or, in newer
                // responses, at key "21" as a single-element array.
                if second.len() == 2 && second.get(1).and_then(|v| v.as_str()).is_some() {
                    if let Some(obj) = payload_arr.get(2).and_then(|v| v.as_object()) {
                        if let Some(token) = obj.get("26").and_then(|v| v.as_str()) {
                            continuation_token = Some(token.to_string());
                        } else if let Some(token) = obj
                            .get("21")
                            .and_then(|v| v.as_array())
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_str())
                        {
                            continuation_token = Some(token.to_string());
                        }
                    }
                }
                // First-turn meta shape: [<c_id>, <r_id>, {"26": <token>}]
                if second.len() == 2
                    && second
                        .first()
                        .and_then(|v| v.as_str())
                        .map(|s| s.starts_with("c_"))
                        .unwrap_or(false)
                    && second.get(1).and_then(|v| v.as_str()).is_some()
                {
                    if let Some(obj) = payload_arr.get(2).and_then(|v| v.as_object()) {
                        if let Some(token) = obj.get("26").and_then(|v| v.as_str()) {
                            continuation_token = Some(token.to_string());
                        }
                    }
                }
            }
            continue;
        }

        if payload_arr.len() >= 5 && payload_arr.get(4).and_then(|v| v.as_array()).is_some() {
            main_entry = Some(payload);
        }
    }

    let main = main_entry.ok_or_else(|| {
        Error::parse("StreamGenerate response missing main entry")
    })?;
    let main_arr = main.as_array().ok_or_else(|| {
        Error::parse("StreamGenerate main entry is not an array")
    })?;

    let ids = main_arr.get(1).and_then(|v| v.as_array()).ok_or_else(|| {
        Error::parse("StreamGenerate response missing conversation ids")
    })?;
    let conversation_id = ids
        .first()
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::parse("StreamGenerate response missing conversation_id"))?;
    let response_id = ids
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::parse("StreamGenerate response missing response_id"))?;

    let parts = main_arr.get(4).and_then(|v| v.as_array()).ok_or_else(|| {
        Error::parse("StreamGenerate response missing parts array")
    })?;
    let first_part = parts
        .first()
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::parse("StreamGenerate response missing first part"))?;
    let response_part_id = first_part
        .first()
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::parse("StreamGenerate response missing response_part_id"))?;

    let continuation_token = continuation_token.ok_or_else(|| {
        Error::parse("StreamGenerate response missing continuation token; cannot continue conversation")
    })?;

    Ok(ConversationState {
        conversation_id: conversation_id.to_string(),
        response_id: response_id.to_string(),
        response_part_id: response_part_id.to_string(),
        continuation_token,
    })
}

/// Parses the response parts from a `StreamGenerate` body.
pub fn parse_response_parts(body: &str) -> Result<Vec<ContentPart>> {
    let mut all_parts: Vec<ContentPart> = Vec::new();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let json_start = line.find('[').unwrap_or(0);
        let json_line = &line[json_start..];
        if json_line.is_empty() {
            continue;
        }

        let mut depth: i32 = 0;
        let mut outer_start: Option<usize> = None;
        let mut outer_end: Option<usize> = None;
        for (i, c) in json_line.char_indices() {
            if c == '[' {
                if depth == 0 {
                    outer_start = Some(i);
                }
                depth += 1;
            } else if c == ']' {
                depth -= 1;
                if depth == 0 && outer_start.is_some() {
                    outer_end = Some(i + 1);
                    break;
                }
            }
        }
        let balanced = match (outer_start, outer_end) {
            (Some(s), Some(e)) => &json_line[s..e],
            _ => json_line,
        };

        let parsed: Value = match serde_json::from_str(balanced) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let arr = match parsed.as_array() {
            Some(a) => a,
            None => continue,
        };

        for item in arr {
            let entry = match item.as_array() {
                Some(e) if e.len() >= 3 => e,
                _ => continue,
            };
            let rpc_id = entry[0].as_str().unwrap_or("");
            if rpc_id != "wrb.fr" {
                continue;
            }
            let json_str = match entry[2].as_str() {
                Some(s) => s,
                None => continue,
            };
            let inner_parsed: Value = match serde_json::from_str(json_str) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let inner_arr = match inner_parsed.as_array() {
                Some(a) => a,
                None => continue,
            };
            let parts_json = if let Some(parts) = inner_arr.get(4).and_then(|v| v.as_array()) {
                parts
            } else if let Some(first) = inner_arr.first().and_then(|v| v.as_array()) {
                match first.get(4).and_then(|v| v.as_array()) {
                    Some(parts) => parts,
                    None => continue,
                }
            } else {
                continue;
            };

            for part in parts_json {
                let part_arr = match part.as_array() {
                    Some(a) => a,
                    None => continue,
                };
                let content_list = match part_arr.get(1).and_then(|v| v.as_array()) {
                    Some(a) => a,
                    None => continue,
                };
                let mut current_text: Option<String> = None;
                for content in content_list {
                    if let Some(s) = content.as_str() {
                        if s.is_empty() || is_id_string(s) {
                            continue;
                        }
                        current_text = Some(match current_text {
                            Some(prev) => format!("{prev}{s}"),
                            None => s.to_string(),
                        });
                        continue;
                    }
                    if let Some(prev) = current_text.take() {
                        all_parts.push(ContentPart::Text(prev));
                    }
                }
                if let Some(prev) = current_text.take() {
                    all_parts.push(ContentPart::Text(prev));
                }
            }
        }
    }

    if all_parts.is_empty() {
        if let Some(code) = extract_bard_error_code(body) {
            return Err(Error::Api {
                status: reqwest::StatusCode::BAD_REQUEST,
                message: format!("Gemini returned BardErrorInfo [{code}]"),
            });
        }
        Err(Error::parse("could not parse response from Gemini web frontend"))
    } else {
        Ok(all_parts)
    }
}

fn is_id_string(s: &str) -> bool {
    (s.starts_with("r_") || s.starts_with("c_")) && s.len() > 2
}

/// Extract text from a parsed `wrb.fr` JSON response.
///
/// This is a helper for callers that receive a pre-parsed outer batchexecute or
/// StreamGenerate response and only need the plain text answer. It mirrors the
/// shape processed by [`parse_response_parts`].
pub fn extract_text_from_parsed_response(parsed: &Value) -> Option<String> {
    let arr = parsed.as_array()?;

    for item in arr {
        let entry = item.as_array()?;
        if entry.len() < 3 {
            continue;
        }
        let rpc_id = entry[0].as_str()?;
        if rpc_id != "wrb.fr" {
            continue;
        }
        let payload_str = entry[2].as_str()?;
        let payload: Value = serde_json::from_str(payload_str).ok()?;
        let payload_arr = payload.as_array()?;
        let parts = payload_arr.get(4)?.as_array()?;

        let mut combined = String::new();
        for part in parts {
            let part_arr = part.as_array()?;
            let content_list = part_arr.get(1)?.as_array()?;
            for content in content_list {
                if let Some(s) = content.as_str() {
                    if s.is_empty() || is_id_string(s) {
                        continue;
                    }
                    combined.push_str(s);
                }
            }
        }
        if !combined.is_empty() {
            return Some(combined);
        }
    }

    None
}

/// Extracts the numeric code from a `BardErrorInfo` wrapper if present.
pub fn extract_bard_error_code(body: &str) -> Option<u64> {
    let start = body.find("BardErrorInfo")?;
    let after = &body[start..];
    let open = after.find('[')?;
    let close = after[open..].find(']')?;
    let inner = &after[open + 1..open + close];
    inner.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_text_response() {
        let body = include_str!("../../tests/fixtures/chat_response_minimal.json");
        let response = parse_chat_response(body).unwrap();
        assert_eq!(response.text(), "Hello, world!");
    }

    #[test]
    fn parse_text_response_with_concatenated_strings() {
        let body = include_str!("../../tests/fixtures/chat_response_concatenated.json");
        let response = parse_chat_response(body).unwrap();
        assert_eq!(response.text(), "Hello, world!");
    }

    #[test]
    fn extract_bard_error_code_1096() {
        let body = include_str!("../../tests/fixtures/bard_error_1096.json");
        assert_eq!(extract_bard_error_code(body), Some(1096));
    }

    #[test]
    fn parse_model_list_example() {
        let body = include_str!("../../tests/fixtures/model_list_minimal.txt");
        let models = parse_model_list(body).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].display_name(), "Gemini 3.6 Flash");
        assert_eq!(models[0].category_enum, 1);
    }
}
