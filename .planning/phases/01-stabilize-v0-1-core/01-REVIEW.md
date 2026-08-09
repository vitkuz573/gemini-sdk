---
phase: 01-stabilize-v0-1-core
reviewed: 2026-08-09T18:15:00Z
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

**Reviewed:** 2026-08-09T18:15:00Z
**Depth:** deep
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Reviewed the post-fix source of the v0.1 stabilization phase for `gemini-sdk`. The earlier review surfaced and fixed several critical and warning-tier issues (captured in `01-REVIEW-FIX.md`). A deep re-review of the resulting code still finds material correctness, security, and API-stability concerns that should be addressed before this phase ships, including a **secret leakage path via `Cookies` Display**, **SSRF / open redirect in upload consent handling**, **poisoned-mutex unwrapping that can panic**, **conversation state still not propagated from `ChatBuilder` through the public `generate` path**, and several smaller issues in parsing, retry semantics, and public API evolution.

## Critical Issues

### CR-01: `Cookies` and `Credentials` leak secrets through `Display` / `to_header_value`

**File:** `src/auth.rs:441-444`, `src/auth.rs:196-227`
**Issue:** `Credentials` redacts secrets in `Debug` but provides a fully-revealing `to_header_value()` that is used by `Cookies` to implement `fmt::Display`. `Cookies` itself implements `Display` by calling `to_header_value()` with raw cookie values. Any code that formats a `Cookies` or `Credentials` value with `{}` instead of `{:?}` (e.g., error messages, logs, tracing spans, serde-style string formatting) will emit the complete session cookie header. This undermines the redaction design and is a realistic secret-exposure vector in async Rust code where `tracing::info!(cookies = %client.cookies())` is a natural mistake.

**Fix:** Remove `impl fmt::Display for Cookies` (it is not required by any caller in the reviewed code) and make `to_header_value()` crate-private or require an explicit method name such as `to_cookie_header()`. If `Display` must be kept for compatibility, make it redact the same fields as `Debug`. Add a test that asserts `{}` formatting does not contain secret substrings.

### CR-02: `accept_consent_and_refresh` follows an unvalidated URL from HTML and posts cookies to it

**File:** `src/client.rs:758-789`
**Issue:** `extract_consent_save_url` parses a URL from the `/app` HTML and `accept_consent_and_refresh` POSTs the user's full cookie header to it with no validation that the URL belongs to `consent.google.com` or the expected Google origin. A compromised `/app` response (MITM, malicious redirect, or HTML injection) can redirect the session cookies to an attacker-controlled URL. This is an SSRF/open-redirect credential-exfiltration risk.

**Fix:** Validate the extracted URL before sending credentials to it:

```rust
fn is_trusted_consent_origin(url: &str) -> bool {
    url.starts_with("https://consent.google.com/")
        || url.starts_with("https://accounts.google.com/")
}
```

Return an error if the URL does not match an allow-list of known Google consent origins. Additionally, consider limiting the forwarded cookies to the minimum required (e.g., `SOCS` flow cookies) rather than the entire signed-in header.

## Warnings

### WR-01: `Inner::cookies` mutex unwrap can panic on poison

**File:** `src/client.rs:168`, `src/client.rs:785`
**Issue:** `cookies()` and `accept_consent_and_refresh` use `lock().unwrap_or_else(std::sync::PoisonError::into_inner)`. While this recovers the data, any future panic in a thread that holds the mutex will still propagate a poison error to every caller; the `unwrap_or_else` only helps if the panic already happened. More importantly, if a task holding `cookies` panics, subsequent code that also uses `cookies()` silently continues with potentially corrupted data. The `cookies` mutex is synchronous and shared with async code, so a panic in one `await` point can leave all future cookie reads returning recovered-but-stale data.

**Fix:** Prefer `tokio::sync::RwLock` or `tokio::sync::Mutex` for the cookie jar so poisoning is not a concern, or model `Cookies` as an `Arc<tokio::sync::RwLock<Cookies>>` with explicit recovery. If `std::sync::Mutex` is kept, document the panic-recovery contract and add a test that verifies poisoned locks do not silently corrupt state.

### WR-02: `ChatBuilder` still does not use `Conversation` state when callers go through `GeminiClient::generate`

**File:** `src/client.rs:276-284`, `src/client.rs:1057-1070`
**Issue:** The fix for CR-02 changed `ChatBuilder::send_message_with_content` to call `generate_raw` with `self.conversation.as_ref()`, which is good. However, the public `GeminiClient::generate` method still hardcodes `None` for the conversation parameter. Callers who build a `Conversation` manually and then call `client.generate(&message, category, config)` instead of using the builder will still lose multi-turn state. The builder-level fix does not close the public API hole.

**Fix:** Add a `generate_with_conversation` public method, or change `generate` to accept an optional `Conversation` parameter. If semver stability is required, add a new method and deprecate the old one, or make `generate` delegate to a new internal helper that accepts the conversation.

### WR-03: `with_backoff` discards response bodies for error classification

**File:** `src/retry.rs:26-60`
**Issue:** `with_backoff` retries `reqwest::Error`s that carry 5xx/429 status codes, but it never sees the response body. If Gemini returns a transient HTTP status with a `BardErrorInfo` payload indicating a permanent failure, the SDK retries blindly. Conversely, permanent `reqwest` errors that wrap a 4xx status are classified as permanent without reading the body. This was acknowledged as skipped in `01-REVIEW-FIX.md`, but the issue remains in the shipped code.

**Fix:** Change the retry operation to return `Result<reqwest::Response, crate::Error>` and classify status/body inside the retry loop. Only convert to `reqwest::Error` after deciding whether the failure is transient. Alternatively, make `with_backoff` generic over an `is_transient` predicate supplied by each call site.

### WR-04: `parse_response_parts` silently ignores parse failures and empty results

**File:** `src/proto/parser.rs:342-476`
**Issue:** The function silently `continue`s on every JSON parse error, missing `wrb.fr` marker, and malformed part. If the entire body is garbage, it eventually returns either a `BardErrorInfo` error or a generic "could not parse response" error with no context about what failed. This makes debugging production failures difficult and can mask protocol drift.

**Fix:** Collect the last parse error (or a count of skipped lines) and include it in the final error message. When returning `Err(Error::parse(...))`, include a short snippet of the input body (truncated and with secrets redacted) so operators can diagnose protocol changes.

### WR-05: `extract_waa_fingerprint_from_model_list` can return an arbitrary 16-character hex token

**File:** `src/client.rs:933-949`
**Issue:** The heuristic scans the model-list body for any 16-character all-hex string that appears more than once and returns the first one. The body can contain many such tokens (hashes, IDs, experiment labels). The function can pick the wrong fingerprint, causing the WAA context header to be incorrect and the streaming request to fail or be flagged.

**Fix:** Anchor the search to the known Pro model block or to the structure around the mode ID field. Validate that the candidate appears in a position that matches the captured protocol (e.g., immediately after a known model-name marker) before returning it.

### WR-06: `upload_file` follows an arbitrary upload URL from the upstream response

**File:** `src/upload.rs:61-65`, `src/upload.rs:68-69`
**Issue:** The second upload step POSTs the file bytes to the URL returned in `x-goog-upload-url` from the first step. While this is part of the resumable-upload protocol, the URL is not validated against an expected origin (`push.clients6.google.com` or a known Google upload host). A malicious or compromised initial response could redirect the bytes to an attacker-controlled host, exfiltrating image data.

**Fix:** Validate that the upload URL has a host matching `*.google.com` or `*.clients6.google.com` before following it. Parse the URL and reject unexpected schemes or hosts.

### WR-07: `generate_raw` drops non-UTF-8 bytes instead of surfacing the error

**File:** `src/client.rs:303`
**Issue:** `String::from_utf8_lossy` replaces invalid UTF-8 with the replacement character and silently continues. If Gemini ever returns binary or malformed UTF-8, the SDK will pass a lossy string into the parser, which may then produce confusing errors or silently corrupt the conversation state.

**Fix:** Use `String::from_utf8` and return a `Parse` error when the body is not valid UTF-8, so the caller knows the response was malformed rather than receiving a best-effort corrupted string.

## Info

### IN-01: `PreparedRequest` is publicly exported but `#[doc(hidden)]` with public fields

**File:** `src/chat.rs:268-280`, `src/lib.rs:88-89`
**Issue:** `PreparedRequest` is re-exported from the crate root (the comment says it is "intentionally public for benchmarks and advanced use") but hidden from docs and has public fields. This creates a semver hazard: adding or removing fields is a breaking change for any downstream code that uses it, yet it is not documented as part of the stable surface. The `api_stability.rs` test does not cover it.

**Fix:** Either remove the public re-export and expose it only under a `benches` or `unstable` feature, or document it fully, mark it `#[non_exhaustive]`, and add it to the API-stability tests.

### IN-02: `tests/real_cookies.rs` uses a non-standard cookie source and skips upload tests unnecessarily

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
