---
phase: 01-stabilize-v0-1-core
fixed_at: 2026-08-09T22:20:00Z
review_path: .planning/phases/01-stabilize-v0-1-core/01-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 8
skipped: 1
status: partial
---

# Phase 01: Code Review Fix Report

**Fixed at:** 2026-08-09T22:20:00Z
**Source review:** .planning/phases/01-stabilize-v0-1-core/01-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 9
- Fixed: 8
- Skipped: 1

## Fixed Issues

### CR-01: `Cookies` and `Credentials` leak secrets through `Display` / `to_header_value`

**Files modified:** `src/auth.rs`
**Commit:** `a9de263`
**Applied fix:**
- Changed `impl fmt::Display for Cookies` to print `<N cookies>` instead of the full header value.
- Made `Credentials::to_header_value` `pub(crate)` so external callers cannot accidentally reveal secrets.
- Added a test asserting `{}` formatting does not contain secret substrings.

### CR-02: `accept_consent_and_refresh` follows an unvalidated URL from HTML and posts cookies to it

**Files modified:** `src/session.rs`
**Commit:** `df4dd18`
**Applied fix:**
- Added `is_trusted_consent_origin()` helper allowing only `https://consent.google.com/` and `https://accounts.google.com/`.
- `extract_consent_save_url` now filters both `reject_save_url` and `accept_save_url` through the allow-list before returning them.
- Added unit tests covering trusted origins and rejection of untrusted / non-HTTPS origins.

### WR-01: Broken rustdoc intra-doc link fails `cargo doc`

**Files modified:** `src/client.rs`
**Commit:** `330fb73`
**Applied fix:**
- Replaced the non-existent `[`GeminiClient::parse_stream_body`]` link with `[`GeminiClient::ingest_conversation_state`]` in the `stream_generate_raw` docs.
- Verified with `cargo doc --no-deps --all-features`.

### WR-02: `Inner::cookies` mutex unwrap can panic on poison

**Files modified:** `src/client.rs`
**Commit:** `22f73b7`
**Applied fix:**
- Replaced `std::sync::Mutex<Cookies>` with `tokio::sync::Mutex<Cookies>`.
- Made `cookies()` async and updated all call sites (list_models, build_stream_generate_request, run_waa_init_chain, fetch_app_page, accept_consent_and_refresh).
- Removed all `std::sync::PoisonError::into_inner` recovery paths.

### WR-03: `ChatBuilder` fixes do not close the public `GeminiClient::generate` hole for conversation state

**Files modified:** `src/client.rs`
**Commit:** `3da8c51`
**Applied fix:**
- Refactored `generate()` to delegate to a new public `generate_with_conversation()` method that accepts `Option<&Conversation>`.
- Preserved the existing `generate` signature for semver stability while exposing the new API for callers managing `Conversation` manually.

### WR-05: `parse_response_parts` silently ignores all parse failures

**Files modified:** `src/proto/parser.rs`
**Commit:** `0b0df97`
**Applied fix:**
- Added a `last_error` accumulator capturing the specific failure reason for each skipped branch.
- When no parts are parsed and no Bard error code is found, the returned parse error now includes `last_error` and a short, redacted body snippet (via a new `redact_body_snippet` helper).

### WR-06: `extract_waa_fingerprint_from_model_list` can return an arbitrary 16-character hex token

**Files modified:** `src/client.rs`
**Commit:** `d83a3cc`
**Applied fix:**
- Anchored the search to the Pro model block by locating `"Pro"` and limiting the scan to the enclosing model-list array.
- Added unit tests confirming the correct token is selected and decoy tokens outside the model list are ignored.

### WR-07: `upload_file` follows an arbitrary upload URL from the upstream response

**Files modified:** `src/upload.rs`
**Commit:** `1901271`
**Applied fix:**
- Parsed the `x-goog-upload-url` header and validated scheme (`https`) and host (ends with `.google.com`) using `reqwest::Url` before following it.
- Returns a parse error with an untrusted-origin message if validation fails.

## Skipped Issues

### WR-04: `with_backoff` discards response bodies for error classification

**File:** `src/retry.rs:26-60`
**Reason:** Changing `with_backoff` to inspect response bodies would require a broader refactor of all three call sites in `src/client.rs` (list_models, stream_generate_raw, batchexecute_rpc) and the retry test suite. The current closure signature is baked into `send_with_retry`, and a safe change would need either response-body buffering inside the retry loop or a new generic `is_transient` predicate. The suggested classification logic (inspecting `BardErrorInfo` payloads for permanent codes) is also not fully aligned with the existing `Error::is_transient` implementation. Skipped to avoid a risky structural change without a targeted design; recommend addressing in a dedicated retry refactor.
**Original issue:** `with_backoff` retries `reqwest::Error`s that carry 5xx/429 status codes, but it never sees the response body. If Gemini returns a transient HTTP status with a `BardErrorInfo` payload indicating a permanent failure, the SDK retries blindly. Conversely, permanent `reqwest` errors that wrap a 4xx status are classified as permanent without reading the body.

---

_Fixed: 2026-08-09T22:20:00Z_
_Fixer: the agent (gsd-code-fixer)_
_Iteration: 1_
