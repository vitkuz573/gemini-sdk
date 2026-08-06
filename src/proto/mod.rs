//! Protocol-level helpers for the Gemini web frontend WIZ transport.
//!
//! The web frontend communicates via `batchexecute` and `StreamGenerate` using a
//! 97-slot JSON array that mirrors the protobuf layout of
//! `assistant.lamda.BardFrontendService/StreamGenerate`.

use serde_json::{json, Value};
use uuid::Uuid;

pub mod parser;
pub mod slots;

pub use parser::*;
pub use slots::*;

/// WIZ anti-XSSI prefix used by `batchexecute` and `StreamGenerate` responses.
pub const ANTI_XSSI_PREFIX: &str = ")] } ' \n\n";

/// Strips the anti-XSSI prefix and returns the first JSON line from a response.
pub fn strip_xssi_prefix(body: &str) -> Option<&str> {
    body.find('[').map(|idx| {
        let after = &body[idx..];
        after.find('\n').map_or(after, |end| &after[..end])
    })
}

/// Builds the URL-encoded `f.req` form body for `StreamGenerate`.
pub fn build_stream_generate_body(inner_req_list: &[Value], at: Option<&str>) -> String {
    let inner_json = serde_json::to_string(inner_req_list).unwrap_or_default();
    let f_req = json!([null, inner_json]);
    let f_req_str = serde_json::to_string(&f_req).unwrap_or_default();
    let mut form = vec![format!("f.req={}", urlencoding::encode(&f_req_str))];
    if let Some(token) = at {
        if !token.is_empty() {
            form.push(format!("at={}", urlencoding::encode(token)));
        }
    }
    form.join("&")
}

/// Builds the URL-encoded `f.req` form body for batchexecute `GetUserStatus`.
pub fn build_batchexecute_body(at: Option<&str>) -> String {
    let payload = json!([[["otAQ7b", "[]", null, "generic"]]]);
    let payload_str = serde_json::to_string(&payload).unwrap_or_default();
    let mut form = vec![format!("f.req={}", urlencoding::encode(&payload_str))];
    if let Some(token) = at {
        if !token.is_empty() {
            form.push(format!("at={}", urlencoding::encode(token)));
        }
    }
    form.join("&")
}

/// Generates a fresh uppercase request UUID.
pub fn fresh_request_uuid() -> String {
    Uuid::new_v4().to_string().to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_xssi_prefix_returns_first_json_line() {
        let body = ")] } ' \n\n[[\"wrb.fr\",\"x\"]]\n58";
        assert_eq!(strip_xssi_prefix(body), Some("[[\"wrb.fr\",\"x\"]]"));
    }

    #[test]
    fn build_stream_generate_body_includes_at_when_present() {
        let body = build_stream_generate_body(&[], Some("token"));
        assert!(body.contains("f.req="));
        assert!(body.contains("at="));
    }

    #[test]
    fn build_stream_generate_body_omits_empty_at() {
        let body = build_stream_generate_body(&[], Some(""));
        assert!(body.contains("f.req="));
        assert!(!body.contains("&at="));
    }
}
