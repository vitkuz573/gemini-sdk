//! Detection of transient WIZ 400 responses from the Gemini backend.
//!
//! Google occasionally rejects correct batchexecute requests with HTTP 400 and
//! a WIZ error frame containing the markers `er`, `di`, and `af.httprm`.
//! These failures are transient: retrying the same request with identical
//! parameters and cookies often succeeds. This module provides the
//! authoritative classification used by the retry helper.

use reqwest::StatusCode;

use crate::constants::transient::{DI_MARKER, ER_MARKER, HTTPRM_MARKER};

/// Returns `true` when `status` is HTTP 400 and `body` contains the transient
/// WIZ frame markers `er`, `di`, and `af.httprm`.
///
/// The check is intentionally conservative: all three markers must be present
/// before the response is considered transient. This avoids retrying permanent
/// 400s caused by malformed payloads, unknown RPC ids, or cookie rejection.
#[must_use]
pub fn is_wiz_transient_400(status: StatusCode, body: &str) -> bool {
    if status != StatusCode::BAD_REQUEST {
        return false;
    }
    body.contains(ER_MARKER) && body.contains(DI_MARKER) && body.contains(HTTPRM_MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_exact_wiz_transient_pattern() {
        let body = r#"[["er","di",null,"af.httprm"]]"#;
        assert!(is_wiz_transient_400(StatusCode::BAD_REQUEST, body));
    }

    #[test]
    fn rejects_missing_er_marker() {
        let body = r#"[["di",null,"af.httprm"]]"#;
        assert!(!is_wiz_transient_400(StatusCode::BAD_REQUEST, body));
    }

    #[test]
    fn rejects_missing_di_marker() {
        let body = r#"[["er",null,"af.httprm"]]"#;
        assert!(!is_wiz_transient_400(StatusCode::BAD_REQUEST, body));
    }

    #[test]
    fn rejects_missing_httprm_marker() {
        let body = r#"[["er","di",null]]"#;
        assert!(!is_wiz_transient_400(StatusCode::BAD_REQUEST, body));
    }

    #[test]
    fn rejects_non_400_status() {
        let body = r#"[["er","di",null,"af.httprm"]]"#;
        assert!(!is_wiz_transient_400(StatusCode::OK, body));
        assert!(!is_wiz_transient_400(StatusCode::INTERNAL_SERVER_ERROR, body));
        assert!(!is_wiz_transient_400(StatusCode::TOO_MANY_REQUESTS, body));
    }

    #[test]
    fn rejects_empty_body() {
        assert!(!is_wiz_transient_400(StatusCode::BAD_REQUEST, ""));
    }

    #[test]
    fn rejects_sign_in_redirect_html() {
        let body = r#"<html><head><title>Sign in - Google Accounts</title></head></html>"#;
        assert!(!is_wiz_transient_400(StatusCode::BAD_REQUEST, body));
    }

    #[test]
    fn rejects_normal_batchexecute_success() {
        let body = r#")] } '\n\n[["wrb.fr","otAQ7b","[]",null,null,null,"generic"]]"#;
        assert!(!is_wiz_transient_400(StatusCode::BAD_REQUEST, body));
    }

    #[test]
    fn requires_all_three_markers_in_same_body() {
        let body = r#"{"er": true} {"di": true}"#;
        assert!(!is_wiz_transient_400(StatusCode::BAD_REQUEST, body));
    }
}
