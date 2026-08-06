//! Internal session state extracted from the Gemini `/app` page.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::auth::Cookies;
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
    pub(crate) cookies: Cookies,
    pub(crate) conversation_state: Option<ConversationState>,
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
    pub(crate) fn new(cookies: Cookies) -> Self {
        Self {
            language: DEFAULT_LANGUAGE.to_string(),
            cookies,
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

    pub(crate) fn needs_init(&self) -> bool {
        self.build_label.is_none() && self.session_id.is_none()
    }

    pub(crate) fn generate_reqid() -> String {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
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
    let mut state = SessionState {
        language: DEFAULT_LANGUAGE.to_string(),
        ..SessionState::default()
    };

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

    state
}

fn extract_snlim0e(body: &str) -> Option<String> {
    if let Some(idx) = body.find("\"SNlM0e\":\"") {
        let start = idx + "\"SNlM0e\":\"".len();
        if let Some(end) = body[start..].find('"') {
            let token = &body[start..start + end];
            if token.len() > 10 {
                return Some(token.to_string());
            }
        }
    }
    if let Some(idx) = body.find("SNlM0e") {
        let search = &body[idx..];
        if let Some(eq) = search.find("=\"") {
            let start = eq + 2;
            if let Some(end) = search[start..].find('"') {
                let token = &search[start..start + end];
                if token.len() > 10 {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

fn extract_build_label(body: &str) -> Option<String> {
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
    for pattern in ["\"FdrFJe\":\"", "session_id\":\""] {
        if let Some(idx) = body.find(pattern) {
            let start = idx + pattern.len();
            if let Some(end) = body[start..].find('"') {
                let sid = &body[start..start + end];
                if !sid.is_empty() {
                    return Some(sid.to_string());
                }
            }
        }
    }
    None
}

fn extract_push_id(body: &str) -> Option<String> {
    for key in ["\"qKIAYe\":\"", "\"KnDnFf\":\""] {
        if let Some(idx) = body.find(key) {
            let start = idx + key.len();
            if let Some(end) = body[start..].find('"') {
                let feed = &body[start..start + end];
                if feed.starts_with("feeds/") {
                    return Some(feed.to_string());
                }
            }
        }
    }
    None
}

/// Extracts the consent save URL from `/app` HTML when a consent banner is required.
pub(crate) fn extract_consent_save_url(body: &str) -> Option<String> {
    let payload_start = body.find("id=\"bard-initial-data\"")?;
    let data_start = body[payload_start..]
        .find("data-payload=\"")
        .map(|i| i + payload_start)?;
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
    fn extract_session_id_finds_fdrfje() {
        let body = include_str!("../tests/fixtures/app_session_id.txt");
        assert_eq!(
            extract_session_id(body),
            Some("4202905934864668489".to_string())
        );
    }

    #[test]
    fn extract_push_id_prefers_qkiaye() {
        let body = include_str!("../tests/fixtures/app_push_id.txt");
        assert_eq!(
            extract_push_id(body),
            Some("feeds/mcudyrk2a4khkz".to_string())
        );
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
