//! Error types for the Gemini SDK.

use std::fmt;

use reqwest::StatusCode;
use thiserror::Error;

/// A specialized [`Result`] type for Gemini SDK operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The set of errors that can occur when using the Gemini SDK.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The provided configuration is invalid or incomplete.
    #[error("configuration error: {0}")]
    Config(String),

    /// An HTTP request failed at the transport layer.
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// The upstream Gemini web frontend returned an HTTP error status.
    #[error("Gemini API error (HTTP {status}): {message}")]
    Api {
        /// HTTP status code returned by the upstream service.
        status: StatusCode,
        /// Human-readable error message.
        message: String,
    },

    /// The response body could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),

    /// An error occurred while (de)serializing JSON.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The request payload is malformed.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// The request was rate-limited by the upstream service.
    #[error("rate limited: {0}")]
    RateLimited(String),

    /// A transient error that should be retried.
    #[error("transient error: {0}")]
    Transient(String),

    /// An operation timed out.
    #[error("timeout: {0}")]
    Timeout(String),

    /// The session is not signed in or the cookies have expired.
    #[error("not signed in: {0}")]
    NotSignedIn(String),

    /// WAA / ogads attestation initialization failed.
    #[error("attestation failed: {reason}")]
    AttestationFailed {
        /// Human-readable reason for the attestation failure.
        reason: String,
    },

    /// Cookie / credentials validation failed.
    #[error("credentials error: {0}")]
    Credentials(#[from] crate::auth::CredentialsError),

    /// An attestation-related error (browser attestation feature).
    #[cfg(feature = "browser-attestation")]
    #[error("attestation error: {0}")]
    Attestation(String),

    /// A generic internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Returns `true` if the error is considered transient and the request may
    /// be retried.
    #[must_use]
    pub fn is_transient(&self) -> bool {
        matches!(self, Self::Transient(_) | Self::RateLimited(_) | Self::Timeout(_))
            || matches!(self, Self::Api { status, .. } if status.is_server_error() || status.as_u16() == 429)
            || matches!(self, Self::Request(e) if e.status().is_some_and(|s| s.is_server_error() || s.as_u16() == 429))
    }

    /// Creates an API error from an HTTP status and a message.
    pub(crate) fn api(status: StatusCode, message: impl fmt::Display) -> Self {
        Self::Api {
            status,
            message: message.to_string(),
        }
    }

    /// Creates a parse error.
    pub(crate) fn parse(message: impl fmt::Display) -> Self {
        Self::Parse(message.to_string())
    }

    /// Creates a bad-request error.
    pub(crate) fn bad_request(message: impl fmt::Display) -> Self {
        Self::BadRequest(message.to_string())
    }

    /// Creates a transient error.
    pub(crate) fn transient(message: impl fmt::Display) -> Self {
        Self::Transient(message.to_string())
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use super::*;

    #[test]
    fn error_is_send_sync_static() {
        assert_impl_all!(Error: Send, Sync, std::error::Error);
        fn assert_static<T: 'static>() {}
        assert_static::<Error>();
    }

    #[test]
    fn is_transient_detects_transient_variants() {
        assert!(Error::Transient("network".to_string()).is_transient());
        assert!(Error::RateLimited("too many".to_string()).is_transient());
        assert!(Error::Timeout("deadline".to_string()).is_transient());
        assert!(Error::api(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            "server error"
        )
        .is_transient());
        assert!(Error::api(reqwest::StatusCode::TOO_MANY_REQUESTS, "rate limited").is_transient());
    }

    #[test]
    fn is_transient_rejects_permanent_variants() {
        assert!(!Error::Config("bad config".to_string()).is_transient());
        assert!(!Error::Parse("bad json".to_string()).is_transient());
        assert!(!Error::BadRequest("invalid".to_string()).is_transient());
        assert!(!Error::NotSignedIn("expired".to_string()).is_transient());
        assert!(!Error::AttestationFailed { reason: "waa".to_string() }.is_transient());
        assert!(!Error::api(reqwest::StatusCode::BAD_REQUEST, "client error").is_transient());
    }
}
