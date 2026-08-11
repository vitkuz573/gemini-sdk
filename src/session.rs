//! Internal session state extracted from the Gemini `/app` page.

use serde::{Deserialize, Serialize};

use crate::auth::Credentials;
use crate::chat::Conversation;
use crate::constants::rpc_ids::OTAQ7B_RPC_ID;
use crate::constants::wiz_keys::{CFB2H, FDR_FJE, F_SID, OPEP_7C, S06_GRB, SESSION_ID};
use crate::proto::slots::ConversationState as ProtoConversationState;

const DEFAULT_PUSH_ID: &str = "feeds/mcudyrk2a4khkz";
const DEFAULT_LANGUAGE: &str = "en";

/// Browser-observed base values for the per-page `_reqid` counter.
///
/// The Gemini frontend increments a single counter for every batchexecute call
/// on the page. Different RPC families appear to start from different bases
/// (likely because the counter is shifted by RPC-specific offsets). These bases
/// are derived from live HAR captures and produce `_reqid` values whose digit
/// counts match the browser.
pub(crate) const REQID_BASE_OTAQ7B: u64 = 100_000;
pub(crate) const REQID_BASE_PCCK7E: u64 = 5_000_000;
pub(crate) const REQID_BASE_OTHER: u64 = 200_000;

/// Per-client atomic counter used to generate deterministic `_reqid` values.
///
/// The counter is shared across clones of the same [`crate::GeminiClient`] so
/// the SDK emits the same monotonic sequence the browser does. It starts at a
/// base that gives the right digit length for the RPC family and increments on
/// every batchexecute call.
pub(crate) static REQID_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(REQID_BASE_OTHER);

/// Current snapshot format version for forward compatibility.
pub(crate) const SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// Extracted session values from `window.WIZ_global_data` and the consent flow.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    pub access_token: Option<String>,
    pub build_label: Option<String>,
    pub session_id: Option<String>,
    pub language: String,
    pub push_id: Option<String>,
    pub conversation_state: Option<ConversationState>,
    /// WAA token for slot 3 (may be absent if WAA acquisition fails).
    pub waa_token: Option<String>,
    /// Serialized value for the `x-goog-ext-525001261-jspb` header.
    pub waa_context: Option<String>,
    /// Model/mode fingerprint used inside the WAA context header.
    pub waa_fingerprint: Option<String>,
    /// Per-session nonce used for slot 4.
    pub nonce: Option<String>,
}

/// Multi-turn conversation state stored in the SDK session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    pub conversation_id: String,
    pub response_id: String,
    pub response_part_id: String,
    pub continuation_token: String,
}

/// A serialisable snapshot of a client session.
///
/// # Security
///
/// This snapshot contains credentials in recoverable form. Callers are
/// responsible for storing snapshot strings securely; the SDK never writes
/// snapshots to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Format version for forward compatibility.
    pub format_version: u32,
    /// Credentials (with secrets).
    pub credentials: Credentials,
    /// Extracted session state.
    pub session: SessionState,
    /// Optional conversation history.
    pub conversation: Option<Conversation>,
}

impl SessionState {
    #[must_use]
    pub fn new() -> Self {
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
        self.build_label.is_none() || self.session_id.is_none()
    }

    /// Generates a deterministic per-client `_reqid` for batchexecute calls.
    ///
    /// The browser uses a per-page counter that increments on every
    /// batchexecute request, not a wall-clock timestamp. The SDK mirrors this
    /// with a global atomic counter so retries and sequential calls do not
    /// reuse or collide on the same value.
    ///
    /// The optional `rpcid` hint selects a base offset that matches observed
    /// browser digit lengths for specific RPC families.
    pub(crate) fn generate_reqid(rpcid: Option<&str>) -> String {
        let base = match rpcid {
            Some(OTAQ7B_RPC_ID) => REQID_BASE_OTAQ7B,
            Some("PCck7e") => REQID_BASE_PCCK7E,
            _ => REQID_BASE_OTHER,
        };
        // Ensure fresh base if a prior session was initialized with a different
        // base; otherwise keep the current per-client sequence.
        let mut current = REQID_COUNTER.load(std::sync::atomic::Ordering::SeqCst);
        if current < base {
            let _ = REQID_COUNTER.compare_exchange(
                current,
                base,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            );
            current = REQID_COUNTER.load(std::sync::atomic::Ordering::SeqCst);
        }
        let next = current + 1;
        let _ = REQID_COUNTER.compare_exchange(
            current,
            next,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        );
        next.to_string()
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

/// Reasons why `/app` HTML does not look like a signed-in session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SignedInFailure {
    /// The `window.WIZ_global_data` block is missing or malformed.
    MissingWizGlobalData,
    /// `S06Grb` is empty or contains non-digit characters.
    EmptyS06Grb,
    /// `oPEP7c` is absent from the WIZ data block.
    MissingOpep7c,
    /// `oPEP7c` is present but does not look like an email address.
    InvalidEmailShape,
}

impl std::fmt::Display for SignedInFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingWizGlobalData => write!(f, "WIZ_global_data block missing or malformed"),
            Self::EmptyS06Grb => write!(f, "S06Grb empty or non-numeric"),
            Self::MissingOpep7c => write!(f, "oPEP7c missing"),
            Self::InvalidEmailShape => write!(f, "oPEP7c not email-shaped"),
        }
    }
}

/// Extract session parameters from the `/app` HTML body.
///
/// Returns `true` when the `/app` HTML body contains the signed-in markers
/// (`S06Grb` + `oPEP7c`). Because Google has stopped emitting those markers for
/// some valid signed-in sessions, this function also accepts the page as
/// authenticated when it contains a well-formed `window.WIZ_global_data` block
/// with both a `cfb2h` build label and an `FdrFJe` session id.
///
/// This is the single authoritative check used by the client to decide whether
/// the supplied cookies have been accepted by Gemini as an authenticated
/// session.
pub(crate) fn looks_like_signed_in_html(body: &str) -> bool {
    if diagnose_signed_in_html(body).is_ok() {
        return true;
    }
    looks_like_app_session_html(body)
}

/// Returns true when the `/app` HTML body is a valid Gemini app page even if
/// the legacy `S06Grb`/`oPEP7c` signed-in markers are absent.
///
/// This fallback is intentionally conservative: it requires the canonical
/// `window.WIZ_global_data` block, a `cfb2h` build label, and an `FdrFJe`
/// session id. Without these values the SDK cannot build batchexecute or
/// `StreamGenerate` requests, so proceeding would be useless anyway.
pub(crate) fn looks_like_app_session_html(body: &str) -> bool {
    let Some(block) = extract_wiz_global_data_block_safe(body) else {
        return false;
    };
    extract_quoted_value(block, CFB2H)
        .as_deref()
        .is_some_and(|v| v.starts_with("boq_assistant-bard-web-"))
        && extract_quoted_value(block, FDR_FJE)
            .as_deref()
            .is_some_and(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '-'))
}

/// Diagnoses why the `/app` HTML body does not contain signed-in markers.
///
/// Returns `Ok((S06Grb, oPEP7c))` when the markers are present and valid, or
/// `Err(SignedInFailure)` with the first failing condition. This is used for
/// logging and telemetry so callers can distinguish "cookies rejected" from
/// "HTML shape changed".
///
/// Note: `looks_like_signed_in_html` also accepts pages that pass
/// `looks_like_app_session_html` even when this function returns an error, so
/// callers that only need a boolean should use that helper instead.
pub(crate) fn diagnose_signed_in_html(body: &str) -> Result<(String, String), SignedInFailure> {
    let block =
        extract_wiz_global_data_block_safe(body).ok_or(SignedInFailure::MissingWizGlobalData)?;

    let s06grb = extract_quoted_value(block, S06_GRB).unwrap_or_default();
    if s06grb.is_empty() || !s06grb.chars().all(|c| c.is_ascii_digit()) {
        return Err(SignedInFailure::EmptyS06Grb);
    }

    let opep7c = extract_quoted_value(block, OPEP_7C).ok_or(SignedInFailure::MissingOpep7c)?;
    if !looks_like_email(&opep7c) {
        return Err(SignedInFailure::InvalidEmailShape);
    }

    Ok((s06grb, opep7c))
}

fn looks_like_email(value: &str) -> bool {
    let mut parts = value.splitn(2, '@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !local.starts_with('\'')
        && !domain.starts_with('\'')
}

pub(crate) fn extract_from_app_html(body: &str) -> SessionState {
    let mut state = SessionState::new();

    // If the WIZ_global_data block is missing or malformed, still try the
    // fallback extractions rather than bailing out entirely.
    let _block = extract_wiz_global_data_block_safe(body);

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

/// Tries to extract a value from `window.WIZ_global_data` first, then from the
/// whole body, using each key in `keys` in order. The first value that passes
/// `validate` is returned.
fn try_extract_value(
    body: &str,
    keys: &[&str],
    validate: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    let block = extract_wiz_global_data_block(body);
    for key in keys {
        if let Some(b) = block {
            if let Some(value) = extract_quoted_value(b, key).and_then(|v| validate(&v)) {
                return Some(value);
            }
        }
        if let Some(value) = extract_quoted_value(body, key).and_then(|v| validate(&v)) {
            return Some(value);
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
    // Alias order observed in the wild: canonical `SNlM0e`, case variants
    // `SnlM0e` and `snlM0e` (spikes 005, 008).
    try_extract_value(body, &["SNlM0e", "SnlM0e", "snlM0e"], |value| {
        is_valid_snlim0e(value).then(|| value.to_string())
    })
}

fn is_valid_build_label(label: &str) -> Option<String> {
    if label.starts_with("boq_assistant-bard-web-") && label.len() > 10 {
        Some(label.to_string())
    } else {
        None
    }
}

fn extract_build_label(body: &str) -> Option<String> {
    // Primary: Google stores the build label under `cfb2h` inside
    // window.WIZ_global_data. Fallback keys: `build_label`.
    if let Some(value) = try_extract_value(body, &[CFB2H, "build_label"], is_valid_build_label) {
        return Some(value);
    }

    // Fallback: bare substring search for older or stripped responses.
    // Require the same prefix so we never pick up a JS bundle name such as
    // `boq-bard-web...`.
    for pattern in ["boq_assistant-bard-web-server_", "boq_assistant-bard-web-frontend_"] {
        if let Some(idx) = body.find(pattern) {
            let area = &body[idx..];
            for end_char in ['"', '\\', '\'', '`'] {
                if let Some(end) = area.find(end_char) {
                    let label = &area[..end];
                    if let Some(valid) = is_valid_build_label(label) {
                        return Some(valid);
                    }
                }
            }
        }
    }
    None
}

fn looks_like_session_id(sid: &str) -> bool {
    !sid.is_empty() && sid.chars().all(|c| c.is_ascii_digit() || c == '-') && sid.len() >= 3
}

fn extract_session_id(body: &str) -> Option<String> {
    // Canonical key `FdrFJe` is sent as `f.sid`. Older/consent pages may use
    // `session_id`.
    try_extract_value(body, &[FDR_FJE, F_SID, SESSION_ID], |value| {
        looks_like_session_id(value).then(|| value.to_string())
    })
}

fn extract_push_id(body: &str) -> Option<String> {
    // `qKIAYe` is the canonical key; `KnDnFf` and `push_id` are observed
    // aliases in older HTML shapes.
    try_extract_value(body, &["qKIAYe", "KnDnFf", "push_id"], |value| {
        value.starts_with("feeds/").then(|| value.to_string())
    })
}

/// Returns the contents of the `window.WIZ_global_data = { ... };` block,
/// including the opening `window.WIZ_global_data = ` prefix so callers can
/// still anchor searches if needed.
pub(crate) fn extract_wiz_global_data_block(body: &str) -> Option<&str> {
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

/// Returns the contents of the `window.WIZ_global_data` block without the
/// leading prefix, or `None` if the block is missing or malformed.
///
/// This is a defensive variant used when callers only need the raw block and
/// want to avoid panic on malformed braces.
pub(crate) fn extract_wiz_global_data_block_safe(body: &str) -> Option<&str> {
    let start_marker = "window.WIZ_global_data";
    let idx = body.find(start_marker)?;
    let after_marker = &body[idx + start_marker.len()..];
    let eq_idx = after_marker.find('=')?;
    let block_start_in_after = eq_idx + 1;
    let rest = &after_marker[block_start_in_after..];
    let brace_idx = rest.find('{')?;
    let inner = &rest[brace_idx..];
    // If braces are malformed, take_balanced_braces returns the whole slice;
    // that is still safe, but here we prefer to return None so callers fall
    // back to the full-body search.
    let taken = take_balanced_braces(inner);
    if taken.len() < 2 || !taken.starts_with('{') {
        return None;
    }
    Some(taken)
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
pub(crate) fn extract_quoted_value(block: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":\"");
    if let Some(idx) = block.find(&pattern) {
        let start = idx + pattern.len();
        if let Some(end) = block[start..].find('"') {
            return Some(block[start..start + end].to_string());
        }
    }
    None
}

/// True if the consent save URL belongs to a trusted Google origin.
pub(crate) fn is_trusted_consent_origin(url: &str) -> bool {
    url.starts_with("https://consent.google.com/")
        || url.starts_with("https://accounts.google.com/")
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
        .filter(|s| !s.is_empty() && is_trusted_consent_origin(s))
        .or_else(|| value.accept_save_url.filter(|s| !s.is_empty() && is_trusted_consent_origin(s)))
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
    fn extract_build_label_falls_back_to_build_label_key() {
        let body = r#"window.WIZ_global_data = {"build_label":"boq_assistant-bard-web-server_20260804.05_p0"};"#;
        assert_eq!(
            extract_build_label(body),
            Some("boq_assistant-bard-web-server_20260804.05_p0".to_string())
        );
    }

    #[test]
    fn extract_session_id_finds_fdrfje() {
        let body = include_str!("../tests/fixtures/app_session_id.txt");
        assert_eq!(extract_session_id(body), Some("4202905934864668489".to_string()));
    }

    #[test]
    fn extract_session_id_falls_back_to_session_id_key() {
        let body = r#"window.WIZ_global_data = {"session_id":"1234567890123456789"};"#;
        assert_eq!(extract_session_id(body), Some("1234567890123456789".to_string()));
    }

    #[test]
    fn extract_push_id_prefers_qkiaye() {
        let body = include_str!("../tests/fixtures/app_push_id.txt");
        assert_eq!(extract_push_id(body), Some("feeds/mcudyrk2a4khkz".to_string()));
    }

    #[test]
    fn extract_push_id_falls_back_to_push_id_key() {
        let body = r#"window.WIZ_global_data = {"push_id":"feeds/fallback-push-id"};"#;
        assert_eq!(extract_push_id(body), Some("feeds/fallback-push-id".to_string()));
    }

    #[test]
    fn extract_push_id_rejects_non_feeds() {
        let body = r#"window.WIZ_global_data = {"qKIAYe":"not-a-feed","KnDnFf":"also-not"};"#;
        assert_eq!(extract_push_id(body), None);
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
    fn extract_snlim0e_falls_back_to_case_variants() {
        let body =
            r#"window.WIZ_global_data = {"SnlM0e":"ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132"};"#;
        assert_eq!(
            extract_snlim0e(body),
            Some("ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132".to_string())
        );

        let body2 =
            r#"window.WIZ_global_data = {"snlM0e":"ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132"};"#;
        assert_eq!(
            extract_snlim0e(body2),
            Some("ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132".to_string())
        );
    }

    #[test]
    fn extract_snlim0e_prefers_canonical_when_both_present() {
        let body = r#"window.WIZ_global_data = {"snlM0e":"bad:1786124577132","SNlM0e":"ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132"};"#;
        assert_eq!(
            extract_snlim0e(body),
            Some("ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132".to_string())
        );
    }

    #[test]
    fn extract_snlim0e_rejects_invalid_token() {
        // Token suffix has only 12 digits, not 13.
        let body = r#"window.WIZ_global_data = {"SNlM0e":"bad:123456789012"};"#;
        assert_eq!(extract_snlim0e(body), None);
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
    fn generate_reqid_uses_per_client_counter() {
        // Capture current counter state and reset to a known base for the test.
        let _saved = REQID_COUNTER.swap(REQID_BASE_OTHER, std::sync::atomic::Ordering::SeqCst);
        let first = SessionState::generate_reqid(None);
        let second = SessionState::generate_reqid(None);
        assert_ne!(first, second, "reqid must increment each call");
        let n1: u64 = first.parse().unwrap();
        let n2: u64 = second.parse().unwrap();
        assert_eq!(n2, n1 + 1, "reqid must increment by exactly one");
    }

    #[test]
    fn generate_reqid_otaq7b_base_matches_browser_digit_length() {
        let _saved = REQID_COUNTER.swap(REQID_BASE_OTAQ7B, std::sync::atomic::Ordering::SeqCst);
        let reqid = SessionState::generate_reqid(Some(OTAQ7B_RPC_ID));
        assert_eq!(reqid.len(), 6, "otAQ7b reqid must be 6 digits");
        let n: u64 = reqid.parse().unwrap();
        assert!((100_000..1_000_000).contains(&n));
    }

    #[test]
    fn generate_reqid_pcck7e_base_matches_browser_digit_length() {
        let _saved = REQID_COUNTER.swap(REQID_BASE_PCCK7E, std::sync::atomic::Ordering::SeqCst);
        let reqid = SessionState::generate_reqid(Some("PCck7e"));
        assert_eq!(reqid.len(), 7, "PCck7e reqid must be 7 digits");
        let n: u64 = reqid.parse().unwrap();
        assert!((5_000_000..10_000_000).contains(&n));
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

    #[test]
    fn extract_consent_url_rejects_untrusted_origin() {
        let body = r#"<div id="bard-initial-data" data-payload="{&quot;acNycb&quot;:&quot;https://evil.example.com/save&quot;}"></div>"#;
        assert_eq!(extract_consent_save_url(body), None);
    }

    #[test]
    fn is_trusted_consent_origin_allow_lists_google() {
        assert!(is_trusted_consent_origin("https://consent.google.com/save?x=1"));
        assert!(is_trusted_consent_origin("https://accounts.google.com/save"));
        assert!(!is_trusted_consent_origin("https://evil.example.com/save"));
        assert!(!is_trusted_consent_origin("http://consent.google.com/save"));
    }

    #[test]
    fn extract_signed_in_state_ignores_servicelogin_menu_link() {
        // Regression: a signed-in /app page may contain a sign-in link in the
        // OneGoogle account menu. `verify_signed_in` used to reject the session
        // because it matched the "ServiceLogin" substring anywhere in the body.
        let signed_in_with_login_link = include_str!("../tests/fixtures/app_signed_in.txt")
            .to_string()
            + r#"<a href="https://accounts.google.com/ServiceLogin?passive=1209600">Sign in</a>"#;
        let state = crate::client::extract_signed_in_state(&signed_in_with_login_link);
        assert_eq!(
            state,
            Some(("111628289675248526498".to_string(), "vitkuz573@gmail.com".to_string()))
        );
    }

    #[test]
    fn extract_signed_in_state_rejects_empty_gaia() {
        let body = include_str!("../tests/fixtures/app_not_signed_in.txt");
        assert!(crate::client::extract_signed_in_state(body).is_none());
    }

    #[test]
    fn diagnose_signed_in_html_reports_empty_s06grb() {
        let body = r#"window.WIZ_global_data = {"S06Grb":"","oPEP7c":"user@example.com"};"#;
        let result = diagnose_signed_in_html(body);
        assert_eq!(result.unwrap_err(), SignedInFailure::EmptyS06Grb);
    }

    #[test]
    fn diagnose_signed_in_html_reports_missing_opep7c() {
        let body = r#"window.WIZ_global_data = {"S06Grb":"123456"};"#;
        let result = diagnose_signed_in_html(body);
        assert_eq!(result.unwrap_err(), SignedInFailure::MissingOpep7c);
    }

    #[test]
    fn diagnose_signed_in_html_reports_invalid_email() {
        let body = r#"window.WIZ_global_data = {"S06Grb":"123456","oPEP7c":"not-an-email"};"#;
        let result = diagnose_signed_in_html(body);
        assert_eq!(result.unwrap_err(), SignedInFailure::InvalidEmailShape);
    }

    #[test]
    fn diagnose_signed_in_html_reports_missing_wiz_block() {
        let body = r#"<html></html>"#;
        let result = diagnose_signed_in_html(body);
        assert_eq!(result.unwrap_err(), SignedInFailure::MissingWizGlobalData);
    }

    #[test]
    fn looks_like_signed_in_html_accepts_app_session_fallback() {
        // Modern /app responses no longer emit S06Grb/oPEP7c for some sessions,
        // but still contain enough WIZ data to run batchexecute.
        let body = r#"window.WIZ_global_data = {"cfb2h":"boq_assistant-bard-web-server_20260807.00_p0","FdrFJe":"-1234567890123456789"};"#;
        assert!(looks_like_signed_in_html(body));
    }

    #[test]
    fn looks_like_app_session_html_rejects_without_build_label() {
        let body = r#"window.WIZ_global_data = {"FdrFJe":"-1234567890123456789"};"#;
        assert!(!looks_like_app_session_html(body));
    }

    #[test]
    fn looks_like_app_session_html_rejects_without_session_id() {
        let body =
            r#"window.WIZ_global_data = {"cfb2h":"boq_assistant-bard-web-server_20260807.00_p0"};"#;
        assert!(!looks_like_app_session_html(body));
    }
}
