# Phase 18 Research: Auth Header Parity for Usage Stats

**Date:** 2026-08-11
**Phase:** 18 — Auth Header Parity for Usage Stats

## User Constraints (from CONTEXT.md)

- Scope SAPISIDHASH + `x-goog-authuser: 0` **only** to the `jSf9Qc` usage-stats RPC (AUTH-03).
- Other batchexecute RPCs (`get_user_info`, `list_models`, etc.) must remain unchanged.
- Reuse existing `Credentials::sapisid_hash(origin)` for `Authorization: SAPISIDHASH <ts>_<sha1>`.
- Add named constants for `x-goog-authuser` header name and value `0` in `src/constants.rs` (REQ-02).
- Do not change public `GeminiClient` signatures.
- Add wiremock-style integration test verifying headers on `jSf9Qc` and absence on other RPCs.
- Add HAR redaction unit test covering the `Authorization` header (TEST-03).

## Standard Stack

- Rust 1.80+, Tokio, reqwest, serde_json, sha1 (existing dependencies).
- `wiremock` (already in dev-dependencies) for integration tests.
- Existing test patterns in `tests/integration_tests.rs` and inline `#[cfg(test)]` modules.

## Architecture Patterns

- `GeminiClient::build_headers(reqid, waa_context, authorization, endpoint)` returns `Vec<(String, String)>`.
- All batchexecute RPC methods call `build_headers(None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))`.
- `ogads_get_async_data` manually builds reqwest request and passes `Authorization` when available.
- `Credentials::sapisid_hash(origin)` computes `SHA1(<ts> <sapisid> <origin>)` with origin trailing slash stripped.
- HAR redaction in `src/har.rs` already flags `Authorization`, `Cookie`, `Set-Cookie`, and `x-goog-ext-*` as secret headers.

## Don't Hand-Roll

- Do not invent a new SAPISIDHASH computation — use `Credentials::sapisid_hash` via `credentials_to_sapisid_hash`.
- Do not add authorization to `build_headers` generically for all batchexecute RPCs; scope via the existing `authorization` parameter or an RPC-specific opt-in.
- Do not change `UsageStats` public API in this phase (deferred to Phase 19).

## Common Pitfalls

- `sapisid_hash` uses `SystemTime::now()`; tests that assert an exact header value will flake. Assert prefix and non-emptiness instead.
- The `Authorization` header is currently only used by WAA/ogads flows on `clients6.google.com`; applying it to `gemini.google.com` batchexecute must be opt-in per RPC.
- Other RPCs (`get_user_info`/`o30O0e`, `list_models`/`otAQ7b`, locale/model config RPCs) must not receive these headers.
- `x-goog-authuser` must be sent as header name exactly as captured; add constants to avoid magic strings.

## Code Examples

- `credentials_to_sapisid_hash` helper: `src/client.rs:2969`
- `build_headers` signature and body: `src/client.rs:2639`
- `get_usage_stats` request construction: `src/client.rs:1389`
- HAR `is_secret_header`: `src/har.rs:164`
- Existing `Authorization` redaction test pattern: `src/har.rs` inline tests.

## Confidence Levels

- Reuse of `Credentials::sapisid_hash`: HIGH (already used by ogads flow).
- Header scoping via `build_headers` `authorization` parameter: HIGH (existing parameter).
- HAR redaction already covers `Authorization`: HIGH (verified in source).
- Exact `x-goog-authuser` value `0`: MEDIUM (matches REQUIREMENTS.md; no HAR access in this phase).
