//! Protocol-level helpers for the Gemini web frontend WIZ transport.
//!
//! The web frontend communicates via `batchexecute` and `StreamGenerate` using a
//! 97-slot JSON array that mirrors the protobuf layout of
//! `assistant.lamda.BardFrontendService/StreamGenerate`.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use rand::Rng;
use serde_json::{json, Value};
use uuid::Uuid;

pub mod indices;
pub mod parser;
pub mod slots;

pub use indices::*;
pub use parser::*;
pub use slots::*;

/// Re-export response parsing helpers at the `proto` module level.
pub use parser::{
    extract_text_from_parsed_response, extract_thinking_from_parsed_response, parse_chat_response,
    parse_response_parts,
};

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
    build_batchexecute_body_for_rpc("otAQ7b", "[]", at)
}

/// Builds a batchexecute body for an arbitrary RPC id and inner payload.
pub fn build_batchexecute_body_for_rpc(rpcid: &str, inner: &str, at: Option<&str>) -> String {
    // The batchexecute transport expects a triple-wrapped array:
    // [[[rpcid, inner, null, "generic"]]].
    let payload = json!([[[rpcid, inner, null, "generic"]]]);
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

/// Generates a fresh 32-character lowercase hex nonce for slot 4.
pub fn fresh_request_nonce() -> String {
    let mut rng = rand::thread_rng();
    (0..32).map(|_| format!("{:x}", rng.gen_range(0..16))).collect()
}

/// Builds the URL-encoded `f.req` form body for the `sJBwce` WAA prerequisite.
pub fn build_sjbwce_body(at: Option<&str>) -> String {
    // The captured sJBwce payload is `[[[1,2]]]`, triple-wrapped like other
    // batchexecute RPCs.
    let payload = json!([[[1, 2]]]);
    let payload_str = serde_json::to_string(&payload).unwrap_or_default();
    let mut form = vec![format!("f.req={}", urlencoding::encode(&payload_str))];
    if let Some(token) = at {
        if !token.is_empty() {
            form.push(format!("at={}", urlencoding::encode(token)));
        }
    }
    form.join("&")
}

/// Builds the JSON body for the WAA `Create` RPC.
pub fn build_waa_create_body() -> String {
    serde_json::to_string(&json!([[null, "br1aemAN9owlYRs9NnsA"]])).unwrap_or_default()
}

/// Builds the JSON body for the ogads `GetAsyncData` RPC.
pub fn build_ogads_body(waa_token: &str, language: &str) -> String {
    // The captured request body is a single array with the shape Google uses
    // for AsyncDataService/GetAsyncData:
    // [658, origin, 658, language, "ch", 1, null, 0, 0, "", "", 1, 0, null,
    //  103135050, [[1,9,13],0,1,1], [1], null, 1, 0, <base64>, {"1001":0}]
    // The WAA token from Waa/Create is base64-encoded and placed at index 21.
    let encoded_waa = base64::engine::general_purpose::STANDARD.encode(waa_token.as_bytes());
    serde_json::to_string(&json!([
        658,
        "https://gemini.google.com/",
        658,
        language,
        "ch",
        1,
        null,
        0,
        0,
        "",
        "",
        1,
        0,
        null,
        103135050,
        [1, 9, 13],
        0,
        1,
        1,
        [1],
        null,
        1,
        0,
        encoded_waa,
        { "1001": 0 }
    ]))
    .unwrap_or_default()
}

/// Builds the JSON body for the `ESY5D` batchexecute RPC.
pub fn build_esy5d_body(at: Option<&str>) -> String {
    let payload = json!([["ESY5D", "[null,[5]]", null, "generic"]]);
    let payload_str = serde_json::to_string(&payload).unwrap_or_default();
    let mut form = vec![format!("f.req={}", urlencoding::encode(&payload_str))];
    if let Some(token) = at {
        if !token.is_empty() {
            form.push(format!("at={}", urlencoding::encode(token)));
        }
    }
    form.join("&")
}

/// Builds the JSON body for the `K4WWud` batchexecute RPC.
pub fn build_k4wwud_body(language: &str, at: Option<&str>) -> String {
    let inner = json!([[1], [language]]);
    build_batchexecute_body_for_rpc("K4WWud", &inner.to_string(), at)
}

/// Generates the current UTC timestamp used in legacy slot 66.
pub fn current_timestamp_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_xssi_prefix_returns_first_json_line() {
        let body = include_str!("../../tests/fixtures/xssi_prefix.txt");
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

    #[test]
    fn build_batchexecute_body_uses_otaq7b() {
        let body = build_batchexecute_body(Some("at"));
        assert!(body.contains("f.req="));
        assert!(body.contains("otAQ7b"));
        assert!(body.contains("at="));
    }

    #[test]
    fn fresh_request_nonce_is_32_hex_chars() {
        let nonce = fresh_request_nonce();
        assert_eq!(nonce.len(), 32);
        assert!(nonce.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
