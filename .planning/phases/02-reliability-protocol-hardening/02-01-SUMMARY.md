# Phase 02 Plan 01 Summary — Reliability & Protocol Hardening

## Objectives

- Replace blocking `StdMutex<ClientConfig>` with `tokio::sync::RwLock<ClientConfig>` and make `GeminiClient::with_language`, `with_max_retries`, and `with_timeout` async.
- Add `Error::AttestationFailed` and propagate WAA/ogads failures instead of silently falling back to synthetic context.
- Fix `accept_consent_and_refresh` so merged response cookies persist into `self.inner.cookies`.

## Changes Made

### `src/client.rs`

- Replaced `use std::sync::Mutex as StdMutex` with `use tokio::sync::RwLock`.
- Changed `Inner::config` from `StdMutex<ClientConfig>` to `RwLock<ClientConfig>`.
- Converted `with_language`, `with_max_retries`, and `with_timeout` to `pub async fn` methods using `self.inner.config.write().await`.
- Removed `update_config_blocking` helper.
- Updated `accept_consent_and_refresh` to merge `response.cookies()` directly into the locked `self.inner.cookies`, eliminating the intermediate clone that could drop values.
- Updated `init_session` to propagate `run_waa_init_chain` errors instead of logging and continuing.
- Updated `run_waa_init_chain` to map WAA Create and ogads GetAsyncData failures to `Error::AttestationFailed` and removed the `build_default_waa_context` fallback.
- Updated doc comments on `run_waa_init_chain` and `init_session` to reflect the new behavior.

### `src/errors.rs`

- Added new public variant `AttestationFailed { reason: String }` with display text "attestation failed: {reason}".
- Added a doc comment for the `reason` field to satisfy `#![deny(missing_docs)]`.
- Added an assertion in `is_transient_rejects_permanent_variants` that `Error::AttestationFailed` is not transient.

### `tests/integration_tests.rs`

- Added `config_builder_async_sets_language_retries_and_timeout` to verify the async builder methods run inside a Tokio runtime and return `Self`.
- Added `attestation_failed_error_is_not_transient` to verify `Error::AttestationFailed` is classified as non-transient.
- Added `consent_cookie_merge_persists_socs_cookie` to simulate a consent save response with a `Set-Cookie: SOCS=...` header and assert the merged cookie jar retains the SOCS value.

## Verification

- `cargo test --all-targets` — passed (all non-ignored tests).
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo doc --no-deps` — passed with no warnings.
- `cargo build --examples` — passed.
- `cargo build --examples --features browser-attestation` — passed.

## Files Modified

- `src/client.rs`
- `src/errors.rs`
- `tests/integration_tests.rs`
- `.planning/phases/02-reliability-protocol-hardening/02-01-SUMMARY.md` (this file)

## Notes

- The async builder methods are a breaking public API change for a pre-1.0 crate, as approved in phase context.
- `build_default_waa_context` and `build_waa_context_header` remain available as helpers for callers that choose to proceed without live attestation.
