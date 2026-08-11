---
phase: 16-test-cleanup-regression
plan: 01
type: summary
status: passed
wave: 1
---

# Phase 16 Plan 01 Summary

## Objective
Clean up magic strings in tests and examples by reusing constants from `src/constants.rs` and the modules they exercise. Add a regression gate that fails if eliminated magic strings reappear in `src/`, and keep all quality gates green.

## What was done

- Created `tests/common/mod.rs` with shared test constants and helpers:
  - Re-exported public production constants (`RPC_FRAME_MARKER` as `WRB_FR`).
  - Added test-only mirrors for `TEST_LANGUAGE`, `TEST_MOCK_LANGUAGE`, `TEST_PROMPT`, `USER_ROLE`, `MODEL_ROLE`, `MIME_PNG`, `MOCK_COOKIE_HEADER`, `MINIMAL_COOKIE_HEADER`, `BATCHEXECUTE_PATH`, and `default_test_timeout()`.

- Refactored tests to use centralized constants:
  - `tests/integration_tests.rs`: role strings, MIME type, batchexecute path, mock cookie header, language, timeout, and `wrb.fr` marker now come from shared constants.
  - `tests/snapshot_tests.rs`: role strings and mock cookie header now come from `tests/common`.
  - `tests/real_cookies.rs`: PNG MIME type and default test timeout now come from `tests/common`.

- Refactored examples to use centralized constants:
  - `examples/capture_fixtures.rs`: user agent, base URL, app path, batchexecute path, form-urlencoded MIME type, PNG MIME type, `Cookie` header name, `hl`/`_reqid`/`rt` query keys, and `rt` value now come from `gemini_sdk::constants`.
  - `examples/v0_2_api_tour.rs` and `examples/live_probe.rs`: already had no eliminated inline protocol literals; verified they compile unchanged.

- Promoted a minimal public subset of `src/constants.rs` needed by examples/tests:
  - `urls::{GEMINI_BASE_URL, APP_PATH, BATCHEXECUTE_PATH}`
  - `query_keys::{HL, REQID, RT, RT_VALUE}`
  - `mime::{FORM_URLENCODED, PNG}`
  - `headers::COOKIE`
  - `user_agents::UPLOAD_BROWSER_LIKE`

- Added a regression gate in `src/constants.rs` under `#[cfg(test)]`:
  - `regression_tests::no_deny_list_literals_in_source` walks `src/` at test time.
  - Skips `src/constants.rs`.
  - Deny-list targets high-risk protocol literals:
    - `https://gemini.google.com/_/BardChatUi/data/batchexecute`
    - `application/json+protobuf`
    - `application/x-www-form-urlencoded;charset=UTF-8`
    - `bard-storage`
    - `x-goog-upload-command`
  - Prints the offending file and literal on failure.

- Fixed a remaining inline batchexecute URL in `src/har.rs` test code by introducing a `const_format`-assembled `TEST_BATCHEXECUTE_URL` constant.

- Added `const_format` to `[dependencies]` (used by `src/har.rs` tests).

## Verification

- `cargo test --all-targets` passes (including the new regression gate).
- `cargo clippy --all-targets -- -D warnings` passes.
- `cargo doc --no-deps` builds without warnings.

## Files changed

- `tests/common/mod.rs` (new)
- `tests/integration_tests.rs`
- `tests/snapshot_tests.rs`
- `tests/real_cookies.rs`
- `examples/capture_fixtures.rs`
- `src/constants.rs`
- `src/har.rs`
- `Cargo.toml`
