---
phase: 01-stabilize-v0-1-core
reviewed: 2026-08-09T13:45:00Z
depth: deep
files_reviewed: 14
files_reviewed_list:
  - .planning/intel/API-SURFACE.md
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
  info: 3
  total: 12
status: issues_found
---

# Phase 01-stabilize-v0-1-core: Code Review Report

**Reviewed:** 2026-08-09T13:45:00Z
**Depth:** deep
**Files Reviewed:** 14
**Status:** issues_found

## Summary

Reviewed the public API surface, client/session logic, auth, chat types, retry, errors, model metadata, and the supporting test suite. The crate compiles and the majority of tests pass, but one existing unit test fails on `main` and several correctness, security, and robustness issues were found. The most serious concerns are (1) session integrity: `needs_init()` only checks `build_label` and `session_id`, so once those are populated any later upstream state change (e.g. a 401/403 during a request, cookie refresh, consent re-prompt) never triggers re-initialization, and (2) a broken WAA fingerprint extractor that fails the only test covering it and silently degrades attestation. Several smaller issues around error handling, protocol compliance, and maintainability are also noted below.

## Critical Issues

### CR-01: Failing unit test indicates broken WAA fingerprint extraction

**File:** `src/client.rs:951-993` (`extract_waa_fingerprint_from_model_list` and its unit test)
**Issue:** The unit test `extract_waa_fingerprint_anchors_to_pro_model_block` fails with `left: None, right: Some("9d8ca3786ebdfbea")`. The extractor looks for `"]]]"` as the list terminator, but the test fixture is `...]]"` (two closing brackets, then a quote). Because the search for the end of the model list fails, the code falls back to `body.len()`, then scans a much larger model string. The fingerprint does appear in that larger string, but the duplicate-count filter (`model_list.matches(token).count() > 1`) rejects a unique token, so `None` is returned. The implementation therefore does not satisfy the documented requirement of anchoring to the Pro model block and will silently omit the live fingerprint, degrading the WAA/attestation context.
**Fix:** Change the extractor to parse the actual nested JSON array returned by `otAQ7b` instead of using string heuristics. At minimum, fix the heuristic to tolerate `]]` terminators and remove or correct the duplicate-count check so that the unique Pro fingerprint is returned. Add a failing/real fixture to the test suite.

```rust
// Conceptual fix: parse the JSON payload and extract the Pro model id.
let parsed: Value = serde_json::from_str(payload)?;
// navigate to the mode list and find the entry whose name == "Pro",
// then return its hex id.
```

### CR-02: `SessionState::needs_init()` is under-specified and prevents recovery from upstream state changes

**File:** `src/session.rs:60-62`
**Issue:** `needs_init()` returns true only when both `build_label` and `session_id` are `None`. Once those two fields are populated (which happens on the first `/app` fetch), `ensure_session()` in `src/client.rs:476-482` never re-runs `init_session()` again. If Google later returns a sign-out redirect, a consent re-prompt, a 401/403, or the `SNlM0e`/`bl`/`f.sid` values rotate, the client continues using stale session state. This is the root cause class that the spike findings warn against ("do not hardcode `bl` or `f.sid`; always extract from `/app` HTML"), but the implementation does not re-extract after the first call.
**Fix:** Track a `last_init_time` or an explicit `initialized` flag, and add a method `session_is_stale()` that re-fetches `/app` when transient auth errors (`NotSignedIn`, 401/403 API errors) occur or after a configurable TTL. In the short term, call `init_session()` inside `generate_raw`/`stream_generate_raw` when the response indicates an auth/session failure, and reset `build_label`/`session_id` on `NotSignedIn`.

## Warnings

### WR-01: `extract_waa_fingerprint` in `session.rs` duplicates and conflicts with `client.rs` extractor

**File:** `src/session.rs:102-122` and `src/client.rs:951-979`
**Issue:** Two independent heuristics attempt to extract the same Pro model fingerprint. `session.rs` scans a fixed 600-byte window after `"Pro"` and requires the token appear more than once in the whole body. `client.rs` searches for `"]]]"` list terminators and also requires more than one match. The duplicate logic is inconsistent and, as shown by CR-01, the `client.rs` version is broken. Maintaining two string-based parsers for the same upstream value is a bug vector.
**Fix:** Delete one implementation and have both callers use the same parser. Prefer a JSON-based extraction from the `otAQ7b` response payload (see CR-01) so that the same code path serves WAA context construction and session bootstrap.

### WR-02: `stream_generate_raw` swallows successful response bodies on HTTP errors

**File:** `src/client.rs:409-413`
**Issue:** When the upstream returns a non-2xx status, the code reads the response text and returns it as a `Parse`/`Api` error message. For some Gemini errors (e.g. `BardErrorInfo`), the body contains actionable JSON that `extract_bard_error_code` and the parser know how to handle. Returning a raw string means callers lose structured diagnostics and the retry logic cannot classify the error correctly.
**Fix:** Before returning `Error::api(status, text)`, attempt to parse the body for a `BardErrorInfo` code. If present, emit a typed `Error::Api { status, message: "Gemini returned BardErrorInfo [{code}]" }` (matching the parser's existing logic) and consider specific codes as transient or `NotSignedIn`.

### WR-03: `prepare_request` ignores `conversation_state` for multi-turn history

**File:** `src/chat.rs:283-312`
**Issue:** The function signature accepts `conversation: Option<&Conversation>` but never reads `conversation.messages`. The only state carried forward is the server-side `ConversationState` (ids + continuation token). This means the SDK cannot reconstruct a conversation from its local message history without relying entirely on Google's continuation token. If the continuation token expires or the server resets state, the local `Conversation` object becomes useless. The doc comment on `Conversation` says "callers that mutate it directly are responsible…", but the SDK itself never uses the history it stores.
**Fix:** Either implement history flattening into the prompt (with clear limits and token counting) or document that `Conversation` is only a client-side cache and that multi-turn support requires server-side state. If the latter, rename the type or make the limitation explicit to avoid user surprise.

### WR-04: `send_with_retry` retries only `reqwest::Error`, not SDK transient errors

**File:** `src/retry.rs:26-60` and `src/client.rs:852-858`
**Issue:** `with_backoff` requires `Fut: Future<Output = Result<T, reqwest::Error>>`. All callers therefore return `reqwest::Response` and check status/parsing *after* the retry wrapper exits. Any `Error::Transient` or `Error::Api { status: 5xx }` raised inside `batchexecute_rpc`, `waa_create`, etc. is never retried because it is not a `reqwest::Error`. For example, a transient "WAA Create failed" returns immediately with no retries.
**Fix:** Generalize the retry helper to accept an async closure returning `crate::Result<T>` and use `Error::is_transient()` to decide retryability. Alternatively, wrap the whole operation (including status checks) into the closure passed to `send_with_retry`.

### WR-05: `build_stream_generate_body` double-encodes the inner request list

**File:** `src/proto/mod.rs:38-49`
**Issue:** The body is built as `f.req=[null, <inner_json_string>]`, where `inner_json` is a JSON string produced by `serde_json::to_string`. The captured protocol shape (per the spike reference) sends `f.req=[null,"<inner_req_list JSON>"]`: the inner list is a JSON string inside the outer JSON array. The current implementation produces `f.req=[null, "[...]"]` after URL encoding, which is the intended shape, but it relies on the inner value being a string rather than a nested array. Verify this against a real fixture; if the inner array is serialized as an array (not a string), the server will reject it. The unit test only checks that `f.req=` and `at=` are present, not the structural correctness.
**Fix:** Add a unit test that decodes the URL-encoded `f.req` value, parses it as JSON, and asserts that index 1 is a JSON string containing the inner req list (or the array, depending on the actual protocol). Keep the implementation aligned with the captured traffic.

### WR-06: `extract_bard_error_code` parses arbitrary bracket contents after a substring match

**File:** `src/proto/parser.rs:612-619`
**Issue:** The function searches for `"BardErrorInfo"` anywhere in the body, then grabs the first `[` after it and the first `]` after that. This is fragile: a `BardErrorInfo` object that contains nested arrays or appears inside a larger JSON string can yield the wrong numeric slice. It also accepts negative numbers or overflow because `parse::<u64>()` will fail silently (returns `None`), but a malformed slice such as `"-1,100"` could be parsed as an `i64` were the signature different.
**Fix:** Parse the body as JSON and navigate to the `BardErrorInfo` field by path. If performance is a concern, use a small state machine that respects string literals and brackets. Add tests for nested-array and false-positive cases.

### WR-07: `CookieHeaderProvider::new` clones the header twice and discards parsed `Credentials`

**File:** `src/auth.rs:323-328`
**Issue:** `new` parses the header to validate it, throws away the resulting `Credentials`, stores only the raw string, and re-parses on every `credentials()` call. This is inefficient and, more importantly, means any validation/normalization performed by `Credentials::from_header` is lost. If the stored header is later mutated in memory, the provider has no defense (though `String` is immutable in safe Rust, this still reflects poor data hygiene).
**Fix:** Store the parsed `Credentials` directly in `CookieHeaderProvider`. This removes the duplicate parse, preserves validation, and simplifies the `CredentialsProvider` implementation.

## Info

### IN-01: Hard-coded API keys for WAA and ogads are checked into source

**File:** `src/client.rs:40-41`
**Issue:** `WAA_API_KEY` and `OGADS_API_KEY` are public Google API keys embedded in the source code. They are not secrets in the traditional sense (they are visible in browser traffic and are client-side keys), but they are tied to a specific Google project and may be rotated or rate-limited. Shipping them in library source reduces operational flexibility and could violate terms of service if abused.
**Fix:** Move the keys to environment-variable defaults (e.g. `const WAA_API_KEY: &str = env!("GEMINI_WAA_API_KEY", "...default...")`) or document them prominently as borrowed from captured traffic with a plan to make them configurable.

### IN-02: `attestation` module is `pub` but the feature is optional and may expose internal surface

**File:** `src/lib.rs:76-77`
**Issue:** The `attestation` module is declared `pub mod attestation` under `#[cfg(feature = "browser-attestation")]`. Because the module contents were not reviewed here, its public API surface is unknown. If it exposes constructors that accept untrusted input (e.g. a path to a Chrome binary), it could become a command-injection vector.
**Fix:** Audit the `attestation` module for public functions taking paths or shell strings. If it is not meant for direct use, mark the module `pub(crate)` or constrain its public exports.

### IN-03: `tests/api_stability.rs` has an empty assertion for `ModelInfo`

**File:** `tests/api_stability.rs:25-31`
**Issue:** The test `model_info_has_no_public_fields` does not actually assert that fields are private; it only creates an `Option<ModelInfo>` and discards it. If a future change adds public fields, this test will not catch it.
**Fix:** Add a negative compile test (e.g. using `trybuild`) or at least construct a `ModelInfo` through the intended public path and assert field access is impossible. A `compile_fail` doctest on each struct is a lightweight option.

---

_Reviewed: 2026-08-09T13:45:00Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: deep_
