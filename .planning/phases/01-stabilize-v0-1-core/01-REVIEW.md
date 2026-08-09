---
phase: 01-stabilize-v0-1-core
reviewed: 2026-08-09T21:45:00Z
depth: deep
files_reviewed: 13
files_reviewed_list:
  - examples/multi_turn_chat.rs
  - src/auth.rs
  - src/chat.rs
  - src/client.rs
  - src/errors.rs
  - src/lib.rs
  - src/models.rs
  - src/retry.rs
  - tests/api_stability.rs
  - tests/auth_provider.rs
  - tests/integration_tests.rs
  - tests/proto_tests.rs
  - tests/redaction.rs
findings:
  critical: 2
  warning: 7
  info: 4
  total: 13
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-08-09T21:45:00Z
**Depth:** deep
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Reviewed the post-fix source of the v0.1 stabilization phase for `gemini-sdk`. Earlier review feedback was partially addressed (see `01-REVIEW-FIX.md`), but a deep re-review of the current tree still finds material correctness, security, and API-stability concerns. The most serious remaining issues are a **secret-leakage path through `Cookies` `Display` / `Credentials::to_header_value`**, **SSRF / open-redirect credential exfiltration in the consent flow**, and a **broken rustdoc intra-doc link that fails `cargo doc`**. Several public-API holes, retry-semantics gaps, and parser robustness issues remain as warnings.

## Critical Issues

### CR-01: `Cookies` and `Credentials` leak secrets through `Display` / `to_header_value`

**File:** `src/auth.rs:196-227`, `src/auth.rs:441-444`
**Issue:** `Credentials` redacts secrets in `Debug` but provides a fully-revealing `to_header_value()` that is used by `Cookies` to implement `fmt::Display`. Any code that formats a `Cookies` or `Credentials` value with `{}` (e.g., `tracing::info!(cookies = %client.cookies())`, error messages, serde-style string formatting) will emit the complete signed-in cookie header. This undermines the redaction design and is a realistic secret-exposure vector.

**Fix:** Remove `impl fmt::Display for Cookies` (no reviewed caller requires it) and make `to_header_value()` crate-private, or rename it to an explicit `to_cookie_header()` method. If `Display` must be kept, make it redact the same fields as `Debug`. Add a test asserting `{}` formatting does not contain secret substrings.

```rust
// Remove or change to:
impl fmt::Display for Cookies {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} cookies>", self.inner.len())
    }
}

// Make to_header_value crate-private on both types.
```

### CR-02: `accept_consent_and_refresh` follows an unvalidated URL from HTML and posts cookies to it

**File:** `src/client.rs:758-789`
**Issue:** `extract_consent_save_url` parses a URL from the `/app` HTML and `accept_consent_and_refresh` POSTs the user's full cookie header to it with no validation that the URL belongs to a trusted Google origin. A compromised `/app` response (MITM, malicious redirect, or HTML injection) can redirect the session cookies to an attacker-controlled URL. This is an SSRF / open-redirect credential-exfiltration risk.

**Fix:** Validate the extracted URL before sending credentials:

```rust
fn is_trusted_consent_origin(url: &str) -> bool {
    url.starts_with("https://consent.google.com/")
        || url.starts_with("https://accounts.google.com/")
}
```

Return an error if the URL does not match an allow-list of known Google consent origins. Consider limiting forwarded cookies to the minimum required for the consent flow rather than the entire signed-in header.

## Warnings

### WR-01: Broken rustdoc intra-doc link fails `cargo doc`

**File:** `src/client.rs:326`, `src/lib.rs:46`
**Issue:** The doc comment on `stream_generate_raw` references `[`GeminiClient::parse_stream_body`]`, but no such method exists. Because the crate denies broken intra-doc links, `cargo doc --no-deps --all-features` fails, which blocks documentation publishing and CI doc builds.

**Fix:** Correct the doc link to the existing helper (`ingest_conversation_state`) or expose a `parse_stream_body` helper. If no such helper is intended, update the comment to direct users to `generate_raw` / `parse_chat_response` instead.

```rust
/// ... After the stream is consumed, callers should use
/// [`GeminiClient::ingest_conversation_state`] to persist state, or call
/// [`GeminiClient::generate_raw`] which does both.
```

### WR-02: `Inner::cookies` mutex unwrap can panic on poison

**File:** `src/client.rs:168`, `src/client.rs:785`
**Issue:** `cookies()` and `accept_consent_and_refresh` use `lock().unwrap_or_else(std::sync::PoisonError::into_inner)`. This recovers the data only after a panic already poisoned the lock. In async code, a panic in one task holding the mutex will propagate poison to every subsequent caller, including across await points. The recovered data may be stale or corrupted because the panicking task could have been mid-mutation. The synchronous mutex is also held across async operations indirectly through `cookies()` clones.

**Fix:** Prefer `tokio::sync::Mutex` or `tokio::sync::RwLock` for the cookie jar so poisoning is not a concern, or model the jar as an `Arc<tokio::sync::RwLock<Cookies>>` with explicit recovery. If `std::sync::Mutex` is kept, document the panic-recovery contract and add a test that verifies poisoned locks do not silently corrupt state.

### WR-03: `ChatBuilder` fixes do not close the public `GeminiClient::generate` hole for conversation state

**File:** `src/client.rs:276-284`, `src/client.rs:1057-1070`
**Issue:** `ChatBuilder::send_message_with_content` now passes `self.conversation.as_ref()` into `generate_raw`, which is correct. However, the public `GeminiClient::generate` method still hardcodes `None` for the conversation parameter. Callers who build a `Conversation` manually and then call `client.generate(&message, category, config)` instead of using the builder will still lose multi-turn state.

**Fix:** Add a public `generate_with_conversation` method, or change `generate` to accept an optional `Conversation` parameter. If semver stability is required, add a new method and deprecate the old one, or make `generate` delegate to a new internal helper that accepts the conversation.

```rust
pub async fn generate_with_conversation(
    &self,
    message: &ChatMessage,
    conversation: Option<&Conversation>,
    category: ModelCategory,
    config: Option<GenerationConfig>,
) -> Result<ChatResponse> {
    let body = self.generate_raw(message, conversation, category, config).await?;
    parse_chat_response(&body)
}
```

### WR-04: `with_backoff` discards response bodies for error classification

**File:** `src/retry.rs:26-60`
**Issue:** `with_backoff` retries `reqwest::Error`s that carry 5xx/429 status codes, but it never sees the response body. If Gemini returns a transient HTTP status with a `BardErrorInfo` payload indicating a permanent failure, the SDK retries blindly. Conversely, permanent `reqwest` errors that wrap a 4xx status are classified as permanent without reading the body.

**Fix:** Change the operation closure to return `Result<reqwest::Response, crate::Error>` and classify status/body inside the retry loop. Only convert to a permanent `reqwest::Error` after deciding whether the failure is transient. Alternatively, make `with_backoff` generic over an `is_transient` predicate supplied by each call site.

### WR-05: `parse_response_parts` silently ignores all parse failures

**File:** `src/proto/parser.rs:342-476`
**Issue:** The function silently `continue`s on every JSON parse error, missing `wrb.fr` marker, and malformed part. If the entire body is garbage, it eventually returns either a `BardErrorInfo` error or a generic "could not parse response" error with no context about what failed. This makes debugging production failures difficult and can mask protocol drift.

**Fix:** Collect the last parse error (or a count of skipped lines) and include it in the final error message. When returning `Err(Error::parse(...))`, include a short snippet of the input body (truncated and with secrets redacted) so operators can diagnose protocol changes.

```rust
let mut last_error: Option<String> = None;
// ... in each Err branch: last_error = Some(format!("..."));
if all_parts.is_empty() {
    return Err(Error::parse(format!(
        "could not parse response from Gemini web frontend (last error: {:?})",
        last_error
    )));
}
```

### WR-06: `extract_waa_fingerprint_from_model_list` can return an arbitrary 16-character hex token

**File:** `src/client.rs:933-949`
**Issue:** The heuristic scans the model-list body for any 16-character all-hex string that appears more than once and returns the first one. The body can contain many such tokens (hashes, IDs, experiment labels). The function can pick the wrong fingerprint, causing the WAA context header to be incorrect and the streaming request to fail or be flagged.

**Fix:** Anchor the search to the known Pro model block or to the structure around the mode ID field. Validate that the candidate appears in a position that matches the captured protocol (e.g., immediately after a known model-name marker) before returning it.

### WR-07: `upload_file` follows an arbitrary upload URL from the upstream response

**File:** `src/upload.rs:61-69`
**Issue:** The second upload step POSTs the file bytes to the URL returned in `x-goog-upload-url` from the first step. While this is part of the resumable-upload protocol, the URL is not validated against an expected origin (`push.clients6.google.com` or a known Google upload host). A malicious or compromised initial response could redirect the bytes to an attacker-controlled host, exfiltrating image data.

**Fix:** Validate that the upload URL has a host matching `*.google.com` or `*.clients6.google.com` before following it. Parse the URL and reject unexpected schemes or hosts.

```rust
let upload_url = start_response
    .headers()
    .get("x-goog-upload-url")
    .and_then(|v| v.to_str().ok())
    .ok_or_else(|| Error::parse("file upload start response missing X-Goog-Upload-URL"))?;

let parsed = url::Url::parse(upload_url)
    .map_err(|e| Error::parse(format!("invalid upload URL: {e}")))?;
if parsed.scheme() != "https" || !matches!(parsed.host_str(), Some(host) if host.ends_with(".google.com")) {
    return Err(Error::parse("upload URL has untrusted origin"));
}
```

## Info

### IN-01: `PreparedRequest` is publicly visible but `#[doc(hidden)]` with public fields

**File:** `src/chat.rs:268-280`, `src/lib.rs:88-89`
**Issue:** `PreparedRequest` is not re-exported from `lib.rs`, but it is used by the benchmark in `benches/slot_building.rs` via `gemini_sdk::chat::PreparedRequest`. It is `#[doc(hidden)]` yet has public fields. This creates a semver hazard: adding or removing fields is a breaking change for any downstream code that uses it, yet it is not documented as part of the stable surface. The `api_stability.rs` test does not cover it.

**Fix:** Either fully document `PreparedRequest` and commit to its stability (add `#[non_exhaustive]`, include it in API-stability tests, and export it explicitly), or keep it crate-private and expose a benchmark-only interface. The current "hidden but public fields" state is the worst of both worlds.

### IN-02: `tests/real_cookies.rs` uses a non-standard cookie source

**File:** `tests/real_cookies.rs:12-26`
**Issue:** The live-cookie test loads cookies from `/tmp/opencode/gemini_cookies.env` instead of the `GEMINI_COOKIES` environment variable used by `integration_tests.rs` and the examples. This creates friction and inconsistency. It also skips `upload_image_works` unless `GEMINI_PUSH_ID` is set, even though `SessionState::effective_push_id` already falls back to a hardcoded default.

**Fix:** Unify live-cookie loading to use `std::env::var("GEMINI_COOKIES")` and remove the `GEMINI_PUSH_ID` skip so the upload path is exercised by the default test run.

### IN-03: Several protocol magic numbers and array indices are undocumented

**File:** `src/proto/parser.rs:19-27`, `src/proto/slots.rs:72-92`
**Issue:** Indices such as `PART_TEXT_INDEX = 1`, `PART_THINKING_INDEX = 37`, slot numbers 30, 80, 96, etc., are hardcoded with only brief comments. When Google changes the protocol, maintainers will struggle to map these indices back to the captured traffic.

**Fix:** Add a `docs/protocol.md` or inline comments that map each index to the captured field name or protobuf field number from the HAR captures. This is especially important for the 97-slot request list.

### IN-04: `GeminiClient::from_hashmap` and `from_cookies` duplicate `from_cookie_header` semantics with no added value

**File:** `src/client.rs:99-117`
**Issue:** `from_cookies`, `from_hashmap`, and `from_cookie_header` all validate the same required cookies and build the same client. Having three nearly identical constructors increases API surface area without clear differentiation. `from_hashmap` is a trivial alias for `from_cookies`.

**Fix:** Deprecate or remove `from_hashmap`, or give it distinct semantics (e.g., do not validate required cookies, allowing lazy validation). Document the intended use case for each constructor.

---

_Reviewed: 2026-08-09T21:45:00Z_
_Reviewer: gsd-code-reviewer_
_Depth: deep_
