//! Internal session state extracted from the Gemini `/app` page.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::proto::slots::ConversationState as ProtoConversationState;

const DEFAULT_PUSH_ID: &str = "feeds/mcudyrk2a4khkz";
const DEFAULT_LANGUAGE: &str = "en";

/// Extracted session values from `window.WIZ_global_data` and the consent flow.
#[derive(Debug, Clone, Default)]
pub(crate) struct SessionState {
    pub(crate) access_token: Option<String>,
    pub(crate) build_label: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) language: String,
    pub(crate) push_id: Option<String>,
    pub(crate) conversation_state: Option<ConversationState>,
    /// WAA token for slot 3 (may be absent if WAA acquisition fails).
    pub(crate) waa_token: Option<String>,
    /// Serialized value for the `x-goog-ext-525001261-jspb` header.
    pub(crate) waa_context: Option<String>,
    /// Model/mode fingerprint used inside the WAA context header.
    pub(crate) waa_fingerprint: Option<String>,
    /// Per-session nonce used for slot 4.
    pub(crate) nonce: Option<String>,
}

/// Multi-turn conversation state stored in the SDK session.
#[derive(Debug, Clone)]
pub(crate) struct ConversationState {
    pub(crate) conversation_id: String,
    pub(crate) response_id: String,
    pub(crate) response_part_id: String,
    pub(crate) continuation_token: String,
}

impl SessionState {
    pub(crate) fn new() -> Self {
        Self {
            language: DEFAULT_LANGUAGE.to_string(),
            ..Default::default()
        }
    }

    pub(crate) fn effective_push_id(&self) -> String {
        std::env::var("GEMINI_PUSH_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| self.push_id.clone())
            .unwrap_or_else(|| DEFAULT_PUSH_ID.to_string())
    }

    pub(crate) fn take_nonce(&mut self) -> String {
        self.nonce.take().unwrap_or_else(crate::proto::fresh_request_nonce)
    }

    pub(crate) fn needs_init(&self) -> bool {
        self.build_label.is_none() && self.session_id.is_none()
    }

    pub(crate) fn generate_reqid() -> String {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        ((ts % 900_000) + 100_000).to_string()
    }
}

/// Parsed `bard-initial-data` payload relevant to consent.
#[derive(Debug, Deserialize)]
pub(crate) struct BardInitialData {
    #[serde(rename = "qw1mtf")]
    pub(crate) reject_save_url: Option<String>,
    #[serde(rename = "acNycb")]
    pub(crate) accept_save_url: Option<String>,
}

/// Extract session parameters from the `/app` HTML body.
pub(crate) fn extract_from_app_html(body: &str) -> SessionState {
    let mut state = SessionState::new();

    if let Some(token) = extract_snlim0e(body) {
        state.access_token = Some(token);
    }
    if let Some(label) = extract_build_label(body) {
        state.build_label = Some(label);
    }
    if let Some(sid) = extract_session_id(body) {
        state.session_id = Some(sid);
    }
    if let Some(push_id) = extract_push_id(body) {
        state.push_id = Some(push_id);
    }
    if let Some(fp) = extract_waa_fingerprint(body) {
        state.waa_fingerprint = Some(fp);
    }

    state
}

fn extract_waa_fingerprint(body: &str) -> Option<String> {
    // The WAA context header contains a stable model/mode fingerprint. In the
    // captured traffic it is the Pro model id "e6fa609c3fa255c0" which appears
    // in the model list (otAQ7b) and in ESY5D feature flags. We scan the area
    // around the "Pro" model block for a 16-character hex string that appears
    // more than once in the page.
    let pro_idx = body.find("\"Pro\"")?;
    let area = &body[pro_idx..(pro_idx + 600).min(body.len())];
    for (start, _) in area.match_indices('"') {
        let inner = &area[start + 1..];
        let end = inner.find('"').unwrap_or(inner.len());
        let token = &inner[..end];
        if token.len() == 16
            && token.chars().all(|c| c.is_ascii_hexdigit())
            && body.matches(token).count() > 1
        {
            return Some(token.to_string());
        }
    }
    None
}

/// Validates the observed `SNlM0e` shape: base64-url-ish prefix, colon,
/// 13-digit Unix timestamp in milliseconds.
fn is_valid_snlim0e(token: &str) -> bool {
    let bytes = token.as_bytes();
    let Some(colon) = bytes.iter().position(|&b| b == b':') else {
        return false;
    };
    if colon == 0 || colon + 1 >= bytes.len() {
        return false;
    }
    let prefix = &bytes[..colon];
    let suffix = &bytes[colon + 1..];
    prefix.iter().all(|&b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        && suffix.len() == 13
        && suffix.iter().all(|&b| b.is_ascii_digit())
}

fn extract_snlim0e(body: &str) -> Option<String> {
    // Primary: anchor to the window.WIZ_global_data block.
    if let Some(block) = extract_wiz_global_data_block(body) {
        if let Some(token) = extract_quoted_value(block, "SNlM0e") {
            if is_valid_snlim0e(&token) {
                return Some(token);
            }
        }
    }

    // Fallback: non-anchored search for the quoted key anywhere in the body.
    if let Some(token) = extract_quoted_value(body, "SNlM0e") {
        if is_valid_snlim0e(&token) {
            return Some(token);
        }
    }

    None
}

fn extract_build_label(body: &str) -> Option<String> {
    // Primary: Google stores the build label under the key `cfb2h` inside
    // window.WIZ_global_data in the current HTML shape.
    if let Some(block) = extract_wiz_global_data_block(body) {
        if let Some(label) = extract_quoted_value(block, "cfb2h") {
            if label.starts_with("boq_assistant-bard-web-") && label.len() > 10 {
                return Some(label);
            }
        }
    }

    // Fallback: bare substring search for older or stripped responses.
    for pattern in ["boq_assistant-bard-web-server_", "boq_assistant-bard-web-frontend_"] {
        if let Some(idx) = body.find(pattern) {
            let area = &body[idx..];
            for end_char in ['"', '\\', '\'', '`'] {
                if let Some(end) = area.find(end_char) {
                    let label = &area[..end];
                    if label.len() > 10 {
                        return Some(label.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_session_id(body: &str) -> Option<String> {
    // FdrFJe is the canonical frontend session id (sent as `f.sid`).
    let search_in = extract_wiz_global_data_block(body).unwrap_or(body);
    if let Some(sid) = extract_quoted_value(search_in, "FdrFJe") {
        if looks_like_session_id(&sid) {
            return Some(sid);
        }
    }

    // Fallback for older/consent pages that may use an explicit session_id key.
    if let Some(sid) = extract_quoted_value(body, "session_id") {
        if looks_like_session_id(&sid) {
            return Some(sid);
        }
    }

    None
}

fn looks_like_session_id(sid: &str) -> bool {
    !sid.is_empty() && sid.chars().all(|c| c.is_ascii_digit() || c == '-') && sid.len() >= 3
}

fn extract_push_id(body: &str) -> Option<String> {
    let search_in = extract_wiz_global_data_block(body).unwrap_or(body);
    for key in ["qKIAYe", "KnDnFf"] {
        if let Some(feed) = extract_quoted_value(search_in, key) {
            if feed.starts_with("feeds/") {
                return Some(feed);
            }
        }
    }
    None
}

/// Returns the contents of the `window.WIZ_global_data = { ... };` block,
/// including the opening `window.WIZ_global_data = ` prefix so callers can
/// still anchor searches if needed.
fn extract_wiz_global_data_block(body: &str) -> Option<&str> {
    let start_marker = "window.WIZ_global_data";
    let idx = body.find(start_marker)?;
    let after_marker = &body[idx + start_marker.len()..];
    let eq_idx = after_marker.find('=')?;
    let block_start_in_after = eq_idx + 1;
    let rest = &after_marker[block_start_in_after..];
    let brace_idx = rest.find('{')?;
    let inner = &rest[brace_idx..];
    Some(take_balanced_braces(inner))
}

/// Returns the substring that starts at the first `{` and ends at the matching
/// `}`, respecting string literals and backslash escapes.
fn take_balanced_braces(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
        } else if b == b'"' {
            in_string = true;
        } else if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            if depth == 1 {
                return &s[..=i];
            }
            depth = depth.saturating_sub(1);
        }
    }
    s
}

/// Extracts the double-quoted value for a JSON-like key inside a text block.
/// Handles `"key":"value"` and `"key" : "value"` but does not unescape.
fn extract_quoted_value(block: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":\"");
    if let Some(idx) = block.find(&pattern) {
        let start = idx + pattern.len();
        if let Some(end) = block[start..].find('"') {
            return Some(block[start..start + end].to_string());
        }
    }
    None
}

/// Extracts the consent save URL from `/app` HTML when a consent banner is required.
pub(crate) fn extract_consent_save_url(body: &str) -> Option<String> {
    let payload_start = body.find("id=\"bard-initial-data\"")?;
    let data_start = body[payload_start..].find("data-payload=\"").map(|i| i + payload_start)?;
    let value_start = data_start + "data-payload=\"".len();
    let value_end = body[value_start..].find('"').map(|i| i + value_start)?;
    let encoded = &body[value_start..value_end];

    let decoded = encoded
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">");

    let value: BardInitialData = serde_json::from_str(&decoded).ok()?;
    value
        .reject_save_url
        .filter(|s| !s.is_empty())
        .or_else(|| value.accept_save_url.filter(|s| !s.is_empty()))
}

impl From<ConversationState> for ProtoConversationState {
    fn from(value: ConversationState) -> Self {
        ProtoConversationState {
            conversation_id: value.conversation_id,
            response_id: value.response_id,
            response_part_id: value.response_part_id,
            continuation_token: value.continuation_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_build_label_finds_label() {
        let body = include_str!("../tests/fixtures/app_build_label.txt");
        assert_eq!(
            extract_build_label(body),
            Some("boq_assistant-bard-web-server_20260804.05_p0".to_string())
        );
    }

    #[test]
    fn extract_build_label_finds_cfb2h() {
        let body = include_str!("../tests/fixtures/wiz_global_data.txt");
        assert_eq!(
            extract_build_label(body),
            Some("boq_assistant-bard-web-server_20260806.17_p0".to_string())
        );
    }

    #[test]
    fn extract_session_id_finds_fdrfje() {
        let body = include_str!("../tests/fixtures/app_session_id.txt");
        assert_eq!(extract_session_id(body), Some("4202905934864668489".to_string()));
    }

    #[test]
    fn extract_push_id_prefers_qkiaye() {
        let body = include_str!("../tests/fixtures/app_push_id.txt");
        assert_eq!(extract_push_id(body), Some("feeds/mcudyrk2a4khkz".to_string()));
    }

    #[test]
    fn extract_snlim0e_from_real_wiz_global_data() {
        let body = include_str!("../tests/fixtures/wiz_global_data.txt");
        assert_eq!(
            extract_snlim0e(body),
            Some("ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132".to_string())
        );
    }

    #[test]
    fn extract_all_from_real_wiz_global_data() {
        let body = include_str!("../tests/fixtures/wiz_global_data.txt");
        let state = extract_from_app_html(body);
        assert_eq!(
            state.access_token,
            Some("ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132".to_string())
        );
        assert_eq!(
            state.build_label,
            Some("boq_assistant-bard-web-server_20260806.17_p0".to_string())
        );
        assert_eq!(state.session_id, Some("-1594710263937718439".to_string()));
        assert_eq!(state.push_id, Some("feeds/mcudyrk2a4khkz".to_string()));
    }

    #[test]
    fn extract_snlim0e_rejects_invalid_token() {
        // Token suffix has only 12 digits, not 13.
        let body = r#"window.WIZ_global_data = {"SNlM0e":"bad:123456789012"};"#;
        assert_eq!(extract_snlim0e(body), None);
    }

    #[test]
    fn extract_push_id_rejects_non_feeds() {
        let body = r#"window.WIZ_global_data = {"qKIAYe":"not-a-feed","KnDnFf":"also-not"};"#;
        assert_eq!(extract_push_id(body), None);
    }

    #[test]
    fn extract_session_id_rejects_empty() {
        let body = r#"window.WIZ_global_data = {"FdrFJe":""};"#;
        assert_eq!(extract_session_id(body), None);
    }

    #[test]
    fn extract_consent_url_from_data_payload() {
        let body = include_str!("../tests/fixtures/bard_initial_data_payload.txt");
        assert_eq!(
            extract_consent_save_url(body),
            Some("https://consent.google.com/save?x=1".to_string())
        );
    }
}
