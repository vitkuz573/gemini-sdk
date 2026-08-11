//! Shared test constants and helpers.
//!
//! This module is intended for use by integration tests only. It mirrors the
//! production constants from `gemini_sdk::constants` where those constants are
//! not public, and re-exports the public subset where available.

use std::time::Duration;

// ---------------------------------------------------------------------------
// Re-exports of public production constants
// ---------------------------------------------------------------------------

/// WIZ frame marker used in batchexecute / StreamGenerate responses.
#[allow(unused_imports)]
pub use gemini_sdk::constants::transport::RPC_FRAME_MARKER as WRB_FR;

// ---------------------------------------------------------------------------
// Test-only constants mirroring production values
// ---------------------------------------------------------------------------

/// Default test language.
#[allow(dead_code)]
pub const TEST_LANGUAGE: &str = "en";

/// A non-default language used by tests that exercise language switching.
#[allow(dead_code)]
pub const TEST_MOCK_LANGUAGE: &str = "ru";

/// Simple prompt used across tests.
#[allow(dead_code)]
pub const TEST_PROMPT: &str = "hello";

/// User role string.
#[allow(dead_code)]
pub const USER_ROLE: &str = "user";

/// Model role string.
#[allow(dead_code)]
pub const MODEL_ROLE: &str = "model";

/// PNG MIME type.
#[allow(dead_code)]
pub const MIME_PNG: &str = "image/png";

/// Cookie header value used by most mocked integration tests.
#[allow(dead_code)]
pub const MOCK_COOKIE_HEADER: &str = "__Secure-1PSID=abc; __Secure-1PSIDCC=def; __Secure-1PAPISID=papi; SID=s; HSID=h; SSID=s";

/// Minimal cookie header used by tests that only need PSID/PSIDCC.
#[allow(dead_code)]
pub const MINIMAL_COOKIE_HEADER: &str = "__Secure-1PSID=abc; __Secure-1PSIDCC=def";

/// Path used by batchexecute RPC endpoints.
#[allow(dead_code)]
pub const BATCHEXECUTE_PATH: &str = "/_/BardChatUi/data/batchexecute";

/// Returns the default timeout used by async tests.
#[allow(dead_code)]
pub fn default_test_timeout() -> Duration {
    Duration::from_secs(30)
}
