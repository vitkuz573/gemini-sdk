//! Response parsing for batchexecute and `StreamGenerate` WIZ frames.

use serde_json::{json, Value};

use crate::chat::{ChatResponse, ContentPart, FunctionCall};
use crate::errors::{Error, Result};
use crate::models::ModelInfo;
use crate::proto::slots::ConversationState;

/// Parses a `GetUserStatus` batchexecute response into a list of model infos.
pub fn parse_model_list(body: &str) -> Result<Vec<ModelInfo>> {
    let payload = crate::proto::strip_xssi_prefix(body).ok_or_else(|| {
        Error::parse("GetUserStatus response does not contain a JSON array")
    })?;

    let outer: Value = serde_json::from_str(payload).map_err(|e| {
        Error::parse(format!("failed to parse GetUserStatus JSON: {e}"))
    })?;

    let outer_array = outer.as_array().and_then(|a| a.first()).and_then(|v| v.as_array()).ok_or_else(|| {
        Error::parse("GetUserStatus response is not a JSON array")
    })?;

    let rpc_entry = outer_array.iter().find(|entry| {
        entry
            .get(1)
            .and_then(|v| v.as_str())
            .map(|s| s == "otAQ7b")
            .unwrap_or(false)
    });

    let rpc_entry = rpc_entry.ok_or_else(|| {
        Error::parse("GetUserStatus response does not contain otAQ7b entry")
    })?;

    let payload_str = rpc_entry
        .get(3)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
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
    let mut function_calls = Vec::new();

    for part in parts {
        match part {
            ContentPart::Text(t) => texts.push(t),
            ContentPart::Image(_) => {}
            ContentPart::FunctionCall { name, args } => {
                function_calls.push(FunctionCall { name, args });
            }
            ContentPart::FunctionResponse { .. } => {}
        }
    }

    if let Some(code) = extract_bard_error_code(body) {
        let message = match code {
            1096 => {
                "Gemini rejected the turn attestation (1096). If this is an image request, browser attestation is required but unavailable or failed."
            }
            1100 => {
                "Gemini rejected the image/file attestation (1100). A real browser must generate valid slot 3/4 tokens for image requests."
            }
            1155 => {
                "Gemini session/parameter mismatch (1155). Try a fresh conversation or enable browser attestation."
            }
            _ => return Err(Error::Api {
                status: reqwest::StatusCode::BAD_REQUEST,
                message: format!("Gemini returned BardErrorInfo [{code}]"),
            }),
        };
        return Err(Error::Api {
            status: reqwest::StatusCode::BAD_REQUEST,
            message: message.to_string(),
        });
    }

    Ok(ChatResponse {
        text: texts.join(""),
        function_calls,
        has_thoughts: false,
        thoughts: Vec::new(),
    })
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
            if let Some(obj) = payload_arr.get(2).and_then(|v| v.as_object()) {
                if let Some(token) = obj.get("26").and_then(|v| v.as_str()) {
                    continuation_token = Some(token.to_string());
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
                    if let Some(obj) = content.as_object() {
                        if let Some(fc) = obj.get("functionCall").and_then(|v| v.as_object()) {
                            if let Some(name) = fc.get("name").and_then(|v| v.as_str()) {
                                let args = fc.get("args").cloned().unwrap_or_else(|| json!({}));
                                all_parts.push(ContentPart::FunctionCall {
                                    name: name.to_string(),
                                    args,
                                });
                            }
                            continue;
                        }
                    }
                }
                if let Some(prev) = current_text.take() {
                    all_parts.push(ContentPart::Text(prev));
                }
            }
        }
    }

    let mut parsed_parts: Vec<ContentPart> = Vec::with_capacity(all_parts.len());
    for part in all_parts {
        match part {
            ContentPart::Text(t) => parsed_parts.extend(split_text_for_function_calls(&t)),
            other => parsed_parts.push(other),
        }
    }

    if parsed_parts.is_empty() {
        Err(Error::parse("could not parse response from Gemini web frontend"))
    } else {
        Ok(parsed_parts)
    }
}

fn is_id_string(s: &str) -> bool {
    (s.starts_with("r_") || s.starts_with("c_")) && s.len() > 2
}

fn split_text_for_function_calls(text: &str) -> Vec<ContentPart> {
    let mut parts = Vec::new();
    let mut last_end = 0;
    let start_tag = "<function_call name=\"";
    let close_tag = "</function_call>";

    while let Some(tag_start) = text[last_end..].find(start_tag) {
        let absolute_start = last_end + tag_start;
        let after_start = absolute_start + start_tag.len();
        let Some(quote_end) = text[after_start..].find('"') else {
            break;
        };
        let name = text[after_start..after_start + quote_end].to_string();
        let after_quote = after_start + quote_end + 1;
        let Some(bracket_start) = text[after_quote..].find('>') else {
            break;
        };
        let content_start = after_quote + bracket_start + 1;
        let Some(close_start) = text[content_start..].find(close_tag) else {
            break;
        };
        let content_end = content_start + close_start;
        let args_str = text[content_start..content_end].trim();
        let args = serde_json::from_str(args_str).unwrap_or_else(|_| json!({}));

        let prefix = &text[last_end..absolute_start];
        if !prefix.trim().is_empty() {
            parts.push(ContentPart::Text(prefix.to_string()));
        }
        parts.push(ContentPart::FunctionCall {
            name,
            args,
        });
        last_end = content_end + close_tag.len();
    }

    let trailing = &text[last_end..];
    if !trailing.trim().is_empty() {
        parts.push(ContentPart::Text(trailing.to_string()));
    }
    parts
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
        let body = r#"[["wrb.fr", null, "[[null, null, null, null, [[\"rc_123\", [\"Hello, world!\"]]]]]"]]"#;
        let response = parse_chat_response(body).unwrap();
        assert_eq!(response.text(), "Hello, world!");
    }

    #[test]
    fn parse_function_call_response() {
        let body = r#"[["wrb.fr", null, "[[null, null, null, null, [[\"rc_1\", [{\"functionCall\": {\"name\": \"get_weather\", \"args\": {\"city\": \"Paris\"}}}]]]]]"]]"#;
        let response = parse_chat_response(body).unwrap();
        assert!(response.has_function_calls());
        assert_eq!(response.function_calls[0].name, "get_weather");
    }

    #[test]
    fn parse_xml_function_call() {
        let body = r#"[["wrb.fr", null, "[[null, null, null, null, [[\"rc_1\", [\"<function_call name=\\\"get_weather\\\">{\\\"city\\\":\\\"Paris\\\"}</function_call>\"]]]]]"]]"#;
        let response = parse_chat_response(body).unwrap();
        assert!(response.has_function_calls());
    }

    #[test]
    fn extract_bard_error_code_1096() {
        let body = r#"[["wrb.fr",null,null,null,null,[13,null,[["type.googleapis.com/assistant.boq.bard.application.BardErrorInfo",[1096]]]]]]"#;
        assert_eq!(extract_bard_error_code(body), Some(1096));
    }

    #[test]
    fn parse_model_list_example() {
        let inner = "[[],[],[],[],[],[],[],[],[],[],[],[],[],[],[],[[\"fbb127bbb056c959\",\"3.6 Flash\",\"All-around help\",null,null,null,null,null,null,null,null,\"Gemini 3.6 Flash\",null,null,null,null,null,1]]]";
        let body = format!(
            ")] }} '\n\n[[[\"wrb.fr\",\"otAQ7b\",null,{},null,null,null,\"generic\"]]]\n58",
            serde_json::to_string(inner).unwrap()
        );
        let models = parse_model_list(&body).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].human_id(), "gemini-3.6-flash");
        assert_eq!(models[0].category_enum, 1);
    }
}
