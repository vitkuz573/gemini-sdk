# Codebase Concerns

**Analysis Date:** 2026-08-08

## Tech Debt

**WAA / attestation fallback silently degrades behavior:**
- Issue: `ogads_get_async_data` errors fall back to `build_default_waa_context()` without surfacing the failure (`src/client.rs:545`).
- Files: `src/client.rs`.
- Impact: Image uploads and multi-turn state may work inconsistently; callers cannot tell whether attestation succeeded.
- Fix approach: Return a typed result, log at `warn`, and/or expose an internal flag so the SDK can skip image-dependent operations when context is synthetic.

**Blocking lock in synchronous builder methods:**
- Issue: `update_config_blocking` calls `Mutex::blocking_lock()` inside non-async `with_language`/`with_max_retries`/`with_timeout` (`src/client.rs:136`).
- Files: `src/client.rs`.
- Impact: Will panic if called inside a Tokio runtime context without a blocking thread pool.
- Fix approach: Use `tokio::sync::RwLock` or make config update fallible/async; at minimum document the restriction.

**Cookie merge drops updated values in `accept_consent_and_refresh`:**
- Issue: `merge_response_cookies` is called on a clone (`cookies`) that is never persisted back to `self.inner.cookies` (`src/client.rs:755`).
- Files: `src/client.rs`.
- Impact: New `SOCS` or refreshed cookies from the consent flow are ignored.
- Fix approach: Persist the merged clone back into `self.inner.cookies`.

**Large, multi-responsibility client methods:**
- Issue: `stream_generate_raw` and `build_stream_generate_request` combine session locking, uploads, header construction, and retry invocation.
- Files: `src/client.rs`.
- Impact: Hard to unit test in isolation; changes risk breaking the entire request path.
- Fix approach: Decompose into smaller helpers with explicit inputs/outputs.

## Known Bugs

**Fixture generation may produce invalid PNG for image upload tests:**
- Symptoms: `tests/real_cookies.rs::upload_image_works` passes a real decoded 1x1 PNG, but `examples/capture_fixtures.rs` uses `b"fake"` as inline image data (`examples/capture_fixtures.rs:53`).
- Files: `examples/capture_fixtures.rs`, `src/upload.rs`.
- Trigger: Running fixture capture for the image-attestation error path.
- Workaround: N/A; the fixture capture path is manual and not part of CI.

## Security Considerations

**API keys and fingerprints embedded in source:**
- Risk: `WAA_API_KEY`, `OGADS_API_KEY`, `X_CLIENT_DATA`, and `WAA_FINGERPRINT_DEFAULT` are hard-coded in `src/client.rs`.
- Files: `src/client.rs`.
- Current mitigation: These values are reverse-engineered public constants tied to the Gemini frontend.
- Recommendations: Document that these are public frontend constants; consider allowing runtime overrides via env vars for advanced users.

**Cookie redaction in Debug is partial:**
- Risk: `Credentials` Debug only shows the first four characters of each secret (`src/auth.rs:252`).
- Files: `src/auth.rs`.
- Current mitigation: Redaction is applied.
- Recommendations: Prefer full redaction (`"<redacted>"`) to avoid leaking secret length or prefix entropy.

**User-Agent and sec-ch-ua headers impersonate Chrome:**
- Risk: Impersonating a specific browser may violate terms of service and could be flagged.
- Files: `src/client.rs`, `src/upload.rs`.
- Current mitigation: Headers match observed browser traffic.
- Recommendations: Document the impersonation rationale and make headers configurable.

## Performance Bottlenecks

**Response parsing scans the entire body line-by-line:**
- Problem: `parse_response_parts` and `extract_conversation_state` iterate every line and perform repeated JSON parsing (`src/proto/parser.rs:342`, `src/proto/parser.rs:202`).
- Files: `src/proto/parser.rs`.
- Cause: Streaming chunks may be large; parsing every line with `serde_json::from_str` is CPU-bound.
- Improvement path: Use a streaming JSON parser or process only changed chunks, and avoid re-parsing the same part ids.

**String allocations in slot building:**
- Problem: `build_inner_req_list` allocates many `serde_json::Value` objects and serializes the full list for every request.
- Files: `src/proto/slots.rs`.
- Cause: The protocol requires a 97-element JSON array.
- Improvement path: Pre-allocate and reuse the base array; benchmark shows this path is already measured (`benches/slot_building.rs`).

## Fragile Areas

**HTML extraction regex/string parsing:**
- Files: `src/session.rs`, `src/client.rs`.
- Why fragile: Google can change `window.WIZ_global_data` keys, ordering, or quoting at any time.
- Safe modification: Add new fallback extractors and more fixture variants; avoid making any single key mandatory.
- Test coverage: Good fixture coverage exists, but live HTML shape changes will break tests.

**Slot indices in `build_inner_req_list` and parser:**
- Files: `src/proto/slots.rs`, `src/proto/parser.rs`.
- Why fragile: Magic indices (e.g., `PART_TEXT_INDEX = 1`, `PART_THINKING_INDEX = 37`, slot 80 for thinking level) are tied to an undocumented protocol.
- Safe modification: Centralize all indices as named constants and add parser tests for each new shape.
- Test coverage: Unit tests cover current shapes; new Google response variants require new fixtures.

**Attestation module depends on DOM selectors:**
- Files: `src/attestation.rs`.
- Why fragile: `data-test-id="send-button"` and `aria-label*="Send"` selectors can change.
- Safe modification: Add multiple selector fallbacks and fail loudly if no send button is found.
- Test coverage: No automated tests for attestation (requires Chrome and live cookies).

## Scaling Limits

**Single `reqwest` client per `GeminiClient`:**
- Current capacity: One connection pool per client; `pool_max_idle_per_host(20)`.
- Limit: High-throughput applications would need multiple clients.
- Scaling path: Allow callers to inject a shared `reqwest::Client` or configure pool size.

**Sequential attachment uploads:**
- Current capacity: Inline images are uploaded one at a time (`src/upload.rs:115`).
- Limit: Latency grows linearly with image count.
- Scaling path: Upload attachments concurrently with `futures::future::try_join_all`.

## Dependencies at Risk

**`backoff` is lightly maintained:**
- Risk: The crate has limited recent activity and may not receive updates for newer Tokio/reqwest versions.
- Impact: Retry logic would need to be reimplemented.
- Migration plan: Replace with `tokio-retry2` or a small internal backoff helper.

**`tokio-tungstenite` and `serde_urlencoded` are optional but feature-locked:**
- Risk: `serde_urlencoded` is pulled in by the `browser-attestation` feature but is not referenced in `src/attestation.rs`.
- Impact: Unnecessary dependency when attestation is enabled.
- Migration plan: Remove `serde_urlencoded` from the feature if unused, or use it for CDP cookie/form encoding.

## Missing Critical Features

**Streaming response parser:**
- Problem: `stream_generate` returns the raw `reqwest::Response` and leaves chunk parsing to the caller.
- Blocks: Consumers cannot easily consume incremental text/thinking tokens.
- Fix approach: Add an async stream adapter that yields parsed `ContentPart` deltas.

**Configurable proxy / custom HTTP client:**
- Problem: `GeminiClient` builds its own `reqwest::Client` with hard-coded settings.
- Blocks: Users behind corporate proxies or with custom TLS requirements.
- Fix approach: Accept a `reqwest::Client` or builder in `with_config`/`from_credentials`.

**Retry configuration / per-request override:**
- Problem: `max_retries` is global and the backoff parameters are hard-coded in `src/retry.rs`.
- Blocks: Fine-tuning for different network conditions.
- Fix approach: Add `ExponentialBackoff` config to `ClientConfig` and expose per-operation retry policies.

## Test Coverage Gaps

**Attestation module:**
- What's not tested: `BrowserAttestationClient::capture_payload` and CDP helpers.
- Files: `src/attestation.rs`.
- Risk: Breaking changes to CDP or DOM selectors go unnoticed until manual testing.
- Priority: Medium.

**Upload error paths:**
- What's not tested: Failed start/finalize responses from `push.clients6.google.com`.
- Files: `src/upload.rs`.
- Risk: Error messages and retry classification may be incorrect.
- Priority: Low.

**Real-cookie integration tests are skipped in CI:**
- What's not tested: `tests/real_cookies.rs` requires `/tmp/opencode/gemini_cookies.env`.
- Files: `tests/real_cookies.rs`.
- Risk: Live API drift is caught late.
- Priority: Medium — consider a scheduled CI job with rotated secrets or contract tests.

---

*Concerns audit: 2026-08-08*
