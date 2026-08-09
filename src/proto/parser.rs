//! Response parsing for batchexecute and `StreamGenerate` WIZ frames.

use serde_json::Value;
use std::collections::HashMap;

use crate::chat::{ChatResponse, ContentPart};
use crate::errors::{Error, Result};
use crate::models::ModelInfo;
use crate::proto::slots::ConversationState;

/// Re-export of [`parse_chat_response`] for the crate root.
pub use parse_chat_response as parse_chat_response_fn;

/// Index of the accumulated answer-text chunk list within a candidate part.
///
/// Each `StreamGenerate` candidate part carries the answer text as an array of
/// string fragments; concatenating them yields the full reply. This mirrors the
/// protobuf field layout of the `assistant.lamda.BardFrontendService` request.
const PART_TEXT_INDEX: usize = 1;

/// Index of the reasoning block within a candidate part.
///
/// When the selected model reasons, the part carries the accumulated thinking
/// text as `[<fragments>, <structured-step-metadata>]`. Only the first element
/// (the plain-text fragments) is needed for extraction. Parts without reasoning
/// omit this index entirely.
const PART_THINKING_INDEX: usize = 37;

/// Text and reasoning content extracted from a single candidate part.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct PartContent {
    text: String,
    thinking: String,
}

/// Extracts answer text and reasoning from one candidate part array.
fn extract_part_content(part_arr: &[Value]) -> PartContent {
    let mut content = PartContent::default();

    if let Some(chunks) = part_arr.get(PART_TEXT_INDEX).and_then(|v| v.as_array()) {
        for c in chunks {
            if let Some(s) = c.as_str() {
                if s.is_empty() || is_id_string(s) {
                    continue;
                }
                content.text.push_str(s);
            }
        }
    }

    // Reasoning block shape: [<fragments>, <structured-step-metadata>].
    if let Some(fragments) = part_arr
        .get(PART_THINKING_INDEX)
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_array())
    {
        for c in fragments {
            if let Some(s) = c.as_str() {
                if s.is_empty() || is_id_string(s) {
                    continue;
                }
                content.thinking.push_str(s);
            }
        }
    }

    content
}

/// Parses a `GetUserStatus` batchexecute response into a list of model infos.
pub fn parse_model_list(body: &str) -> Result<Vec<ModelInfo>> {
    let payload = crate::proto::strip_xssi_prefix(body)
        .ok_or_else(|| Error::parse("GetUserStatus response does not contain a JSON array"))?;

    let outer: Value = serde_json::from_str(payload)
        .map_err(|e| Error::parse(format!("failed to parse GetUserStatus JSON: {e}")))?;

    // Batchexecute can return the RPC entry either directly as the only
    // element of the outer array, or nested one level deeper. Accept both
    // shapes by looking for the first array that contains an `otAQ7b`
    // marker (the batchexecute response can be wrapped in extra arrays
    // depending on the request format).
    fn find_rpc_entry(value: &Value) -> Option<&Value> {
        if let Some(arr) = value.as_array() {
            if let Some(entry) = arr.iter().find(|entry| {
                entry.get(1).and_then(|v| v.as_str()).map(|s| s == "otAQ7b").unwrap_or(false)
            }) {
                return Some(entry);
            }
            // No direct match: try the first element if it is itself an
            // array (extra wrapping level).
            if let Some(first) = arr.first().and_then(|v| v.as_array()) {
                return first.iter().find(|entry| {
                    entry.get(1).and_then(|v| v.as_str()).map(|s| s == "otAQ7b").unwrap_or(false)
                });
            }
        }
        None
    }

    let rpc_entry = find_rpc_entry(&outer)
        .ok_or_else(|| Error::parse("GetUserStatus response does not contain otAQ7b entry"))?;

    // The inner payload is a JSON string at index 2 or 3 depending on the
    // response shape (index 2 is the canonical location for batchexecute
    // RPC replies; some responses place it at index 3).
    let payload_str = rpc_entry
        .get(2)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| rpc_entry.get(3).and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .ok_or_else(|| Error::parse("GetUserStatus response payload missing"))?;

    let inner: Value = serde_json::from_str(payload_str)
        .map_err(|e| Error::parse(format!("failed to parse GetUserStatus inner payload: {e}")))?;

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

        let id = mode_arr.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }

        let title = mode_arr.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let description = mode_arr.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string();

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
    let mut thinkings = Vec::new();

    for part in parts {
        match part {
            ContentPart::Text(t) => texts.push(t),
            ContentPart::Thinking(t) => thinkings.push(t),
            ContentPart::Image(_) => {}
        }
    }

    let text = texts.join("");
    let thinking = thinkings.join("");

    if text.is_empty() && thinking.is_empty() {
        if let Some(code) = extract_bard_error_code(body) {
            return Err(Error::Api {
                status: reqwest::StatusCode::BAD_REQUEST,
                message: format!("Gemini returned BardErrorInfo [{code}]"),
            });
        }
    }

    Ok(ChatResponse::new(text).with_thinking(thinking))
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

    let main =
        main_entry.ok_or_else(|| Error::parse("StreamGenerate response missing main entry"))?;
    let main_arr = main
        .as_array()
        .ok_or_else(|| Error::parse("StreamGenerate main entry is not an array"))?;

    let ids = main_arr
        .get(1)
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::parse("StreamGenerate response missing conversation ids"))?;
    let conversation_id = ids
        .first()
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::parse("StreamGenerate response missing conversation_id"))?;
    let response_id = ids
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::parse("StreamGenerate response missing response_id"))?;

    let parts = main_arr
        .get(4)
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::parse("StreamGenerate response missing parts array"))?;
    let first_part = parts
        .first()
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::parse("StreamGenerate response missing first part"))?;
    let response_part_id = first_part
        .first()
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::parse("StreamGenerate response missing response_part_id"))?;

    let continuation_token = continuation_token.ok_or_else(|| {
        Error::parse(
            "StreamGenerate response missing continuation token; cannot continue conversation",
        )
    })?;

    Ok(ConversationState {
        conversation_id: conversation_id.to_string(),
        response_id: response_id.to_string(),
        response_part_id: response_part_id.to_string(),
        continuation_token,
    })
}

/// Parses the response parts from a `StreamGenerate` body.
///
/// Each stream chunk carries the *accumulated* answer and thinking text so far,
/// so later chunks supersede earlier ones: the most complete version per
/// response-part id wins. Reasoning content, when present, is emitted as a
/// [`ContentPart::Thinking`] following its [`ContentPart::Text`], ordered by
/// first appearance in the body.
pub fn parse_response_parts(body: &str) -> Result<Vec<ContentPart>> {
    // Maps a response-part id to the most complete content seen so far.
    let mut accs: Vec<(String, PartContent)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut last_error: Option<String> = None;

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let json_start = line.find('[').unwrap_or(0);
        // `find` returns a byte offset; ensure we start slicing on a char
        // boundary so subsequent `char_indices()`-based slicing is valid.
        let json_start = line[..json_start].chars().map(|c| c.len_utf8()).sum();
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
            Err(e) => {
                last_error = Some(format!("invalid outer JSON: {e}"));
                continue;
            }
        };
        let arr = match parsed.as_array() {
            Some(a) => a,
            None => {
                last_error = Some("outer value is not an array".to_string());
                continue;
            }
        };

        for item in arr {
            let entry = match item.as_array() {
                Some(e) if e.len() >= 3 => e,
                _ => {
                    last_error = Some("wrb.fr entry is not an array of length >= 3".to_string());
                    continue;
                }
            };
            let rpc_id = entry[0].as_str().unwrap_or("");
            if rpc_id != "wrb.fr" {
                continue;
            }
            let json_str = match entry[2].as_str() {
                Some(s) => s,
                None => {
                    last_error = Some("wrb.fr payload is not a string".to_string());
                    continue;
                }
            };
            let inner_parsed: Value = match serde_json::from_str(json_str) {
                Ok(v) => v,
                Err(e) => {
                    last_error = Some(format!("invalid wrb.fr payload JSON: {e}"));
                    continue;
                }
            };
            let inner_arr = match inner_parsed.as_array() {
                Some(a) => a,
                None => {
                    last_error = Some("wrb.fr payload is not an array".to_string());
                    continue;
                }
            };
            let parts_json = if let Some(parts) = inner_arr.get(4).and_then(|v| v.as_array()) {
                parts
            } else if let Some(first) = inner_arr.first().and_then(|v| v.as_array()) {
                match first.get(4).and_then(|v| v.as_array()) {
                    Some(parts) => parts,
                    None => {
                        last_error = Some("candidate parts array not found at index 4".to_string());
                        continue;
                    }
                }
            } else {
                last_error = Some("candidate wrapper is not an array".to_string());
                continue;
            };

            for part in parts_json {
                let part_arr = match part.as_array() {
                    Some(a) => a,
                    None => {
                        last_error = Some("candidate part is not an array".to_string());
                        continue;
                    }
                };
                let id = part_arr.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let content = extract_part_content(part_arr);
                if content.text.is_empty() && content.thinking.is_empty() {
                    continue;
                }

                let slot = match index.get(&id) {
                    Some(&i) => &mut accs[i],
                    None => {
                        let i = accs.len();
                        accs.push((id.clone(), PartContent::default()));
                        index.insert(id, i);
                        &mut accs[i]
                    }
                };
                // Streaming chunks are cumulative; keep the most complete state.
                if content.text.len() >= slot.1.text.len() && !content.text.is_empty() {
                    slot.1.text = content.text;
                }
                if content.thinking.len() >= slot.1.thinking.len() && !content.thinking.is_empty() {
                    slot.1.thinking = content.thinking;
                }
            }
        }
    }

    let mut all_parts: Vec<ContentPart> = Vec::new();
    for (_, acc) in accs {
        if !acc.text.is_empty() {
            all_parts.push(ContentPart::Text(acc.text));
        }
        if !acc.thinking.is_empty() {
            all_parts.push(ContentPart::Thinking(acc.thinking));
        }
    }

    if all_parts.is_empty() {
        if let Some(code) = extract_bard_error_code(body) {
            return Err(Error::Api {
                status: reqwest::StatusCode::BAD_REQUEST,
                message: format!("Gemini returned BardErrorInfo [{code}]"),
            });
        }
        let snippet = redact_body_snippet(body, 200);
        Err(Error::parse(format!(
            "could not parse response from Gemini web frontend (last error: {:?}; snippet: {})",
            last_error, snippet
        )))
    } else {
        Ok(all_parts)
    }
}

/// Returns a short, redacted prefix of a response body for diagnostics.
fn redact_body_snippet(body: &str, max_len: usize) -> String {
    let end = body.char_indices().map(|(i, _)| i).nth(max_len).unwrap_or(body.len());
    let snippet = &body[..end];
    // Redact values that look like cookie values or long tokens.
    let mut out = String::with_capacity(snippet.len());
    let mut in_value = false;
    let mut name_start = 0usize;
    let mut prev = '\0';
    for (i, c) in snippet.char_indices() {
        if c == '=' && !in_value && i > name_start {
            in_value = true;
            out.push(c);
        } else if in_value && (c == ';' || c == ',' || c.is_whitespace()) {
            if prev != '=' {
                out.push_str("<redacted>");
            }
            in_value = false;
            name_start = i + c.len_utf8();
            out.push(c);
        } else if !in_value {
            out.push(c);
        }
        prev = c;
    }
    if in_value {
        out.push_str("<redacted>");
    }
    out
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
    parsed_parts_content(parsed)
        .into_iter()
        .find_map(|c| (!c.text.is_empty()).then_some(c.text))
}

/// Extract reasoning / thinking text from a parsed `wrb.fr` JSON response.
///
/// This is the counterpart of [`extract_text_from_parsed_response`] for the
/// model's thinking block. It returns the accumulated reasoning markdown, or
/// `None` when the response contains no thinking block.
pub fn extract_thinking_from_parsed_response(parsed: &Value) -> Option<String> {
    parsed_parts_content(parsed)
        .into_iter()
        .find_map(|c| (!c.thinking.is_empty()).then_some(c.thinking))
}

/// Iterates the candidate parts of a parsed `wrb.fr` response entry.
///
/// Returns `None` when the parsed value is not a batchexecute/StreamGenerate
/// array containing a `wrb.fr` entry with a parts list.
fn parsed_parts_content(parsed: &Value) -> Vec<PartContent> {
    let mut out = Vec::new();
    let Some(arr) = parsed.as_array() else {
        return out;
    };

    for item in arr {
        let Some(entry) = item.as_array() else {
            continue;
        };
        if entry.len() < 3 {
            continue;
        }
        if entry[0].as_str() != Some("wrb.fr") {
            continue;
        }
        let Some(payload_str) = entry[2].as_str() else {
            continue;
        };
        let Ok(payload) = serde_json::from_str::<Value>(payload_str) else {
            continue;
        };
        let Some(payload_arr) = payload.as_array() else {
            continue;
        };
        let Some(parts) = payload_arr.get(4).and_then(|v| v.as_array()) else {
            continue;
        };

        for part in parts {
            if let Some(part_arr) = part.as_array() {
                let content = extract_part_content(part_arr);
                if !content.text.is_empty() || !content.thinking.is_empty() {
                    out.push(content);
                }
            }
        }
    }

    out
}

/// Extracts the error code from a `BardErrorInfo` wrapper if present.
///
/// The bracket contents are returned as a string even when they are not
/// purely numeric, so callers can surface structured upstream error details
/// instead of silently discarding them.
pub fn extract_bard_error_code(body: &str) -> Option<String> {
    let start = body.find("BardErrorInfo")?;
    let after = &body[start..];
    let open = after.find('[')?;
    let close = after[open..].find(']')?;
    let inner = &after[open + 1..open + close];
    let code = inner.trim();
    if code.is_empty() {
        None
    } else {
        Some(code.to_string())
    }
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
        assert_eq!(extract_bard_error_code(body), Some("1096".to_string()));
    }

    #[test]
    fn extract_bard_error_code_preserves_non_numeric_code() {
        let body = r#"[["wrb.fr","BardErrorInfo",null,null,["AUTHENTICATION_ERROR"],null]]"#;
        assert_eq!(
            extract_bard_error_code(body),
            Some("\"AUTHENTICATION_ERROR\"".to_string())
        );
    }

    #[test]
    fn extract_bard_error_code_ignores_empty_code() {
        let body = r#"[["wrb.fr","BardErrorInfo",null,null,[],null]]"#;
        assert_eq!(extract_bard_error_code(body), None);
    }

    #[test]
    fn parse_model_list_example() {
        let body = include_str!("../../tests/fixtures/model_list_minimal.txt");
        let models = parse_model_list(body).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].display_name(), "Gemini 3.6 Flash");
        assert_eq!(models[0].category_enum, 1);
    }

    #[test]
    fn parse_thinking_response_extracts_reasoning() {
        let body = include_str!("../../tests/fixtures/thinking_response_raw.txt");
        let response = parse_chat_response(body).unwrap();

        assert!(response.text().contains("идентичный скриншот"));
        assert!(response.thinking().contains("**Comparing Images**"));
        assert!(response.thinking().contains("**Confirming Identity**"));
        assert!(!response.text().contains("Comparing Images"));
    }

    #[test]
    fn parse_thinking_stream_deduplicates_chunks() {
        let body = include_str!("../../tests/fixtures/thinking_response_raw.txt");
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
        assert_eq!(text_parts[0], response_text_of(&parts));
    }

    fn response_text_of(parts: &[ContentPart]) -> &str {
        match parts.first() {
            Some(ContentPart::Text(t)) => t,
            _ => panic!("expected first part to be text"),
        }
    }

    #[test]
    fn extract_part_content_reads_text_and_thinking() {
        let body = include_str!("../../tests/fixtures/thinking_single_part.json");
        let parsed: Value = serde_json::from_str(body).unwrap();
        let entry = parsed.as_array().and_then(|a| a.first()).and_then(|v| v.as_array()).unwrap();
        let payload: Value = serde_json::from_str(entry[2].as_str().unwrap()).unwrap();
        let part = payload[4][0].as_array().unwrap();
        let content = extract_part_content(part);
        assert_eq!(content.text, "hello ");
        assert_eq!(content.thinking, "think step 1");
    }

    #[test]
    fn extract_part_content_skips_id_strings() {
        let body = include_str!("../../tests/fixtures/thinking_id_strings.json");
        let parsed: Value = serde_json::from_str(body).unwrap();
        let entry = parsed.as_array().unwrap().first().unwrap().as_array().unwrap();
        let payload: Value = serde_json::from_str(entry[2].as_str().unwrap()).unwrap();
        let part = payload[4][0].as_array().unwrap();
        let content = extract_part_content(part);
        assert_eq!(content.text, "real");
        assert_eq!(content.thinking, "thought");
    }

    #[test]
    fn parsed_helpers_extract_text_and_thinking() {
        let body = include_str!("../../tests/fixtures/thinking_single_part.json");
        let parsed: Value = serde_json::from_str(body).unwrap();
        assert_eq!(extract_text_from_parsed_response(&parsed).as_deref(), Some("hello "));
        assert_eq!(extract_thinking_from_parsed_response(&parsed).as_deref(), Some("think step 1"));
    }

    #[test]
    fn parse_response_parts_keeps_longest_chunk() {
        let body = include_str!("../../tests/fixtures/thinking_dedup.txt");
        let parts = parse_response_parts(body).unwrap();
        assert_eq!(parts.len(), 2);
        match (&parts[0], &parts[1]) {
            (ContentPart::Text(t), ContentPart::Thinking(tk)) => {
                assert_eq!(t, "much longer answer");
                assert_eq!(tk, "thinking b\nthinking c");
            }
            other => panic!("unexpected parts: {other:?}"),
        }
    }

    #[test]
    fn parse_response_parts_handles_thinking_before_text() {
        let body = include_str!("../../tests/fixtures/thinking_before_text.txt");
        let parts = parse_response_parts(body).unwrap();
        assert_eq!(parts.len(), 2);
        match (&parts[0], &parts[1]) {
            (ContentPart::Text(t), ContentPart::Thinking(tk)) => {
                assert_eq!(t, "answer");
                assert_eq!(tk, "think first\nthink second");
            }
            other => panic!("unexpected parts: {other:?}"),
        }
    }
}
