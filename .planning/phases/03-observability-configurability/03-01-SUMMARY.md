# 03-01 Summary: Request/Response Hooks API (OBS-01)

## Completed

- Added public `HttpHook` trait in `src/client.rs` with boxed-future `on_request` and `on_response` methods.
- Added `http_hook: Option<Arc<dyn HttpHook>>` and `fatal_hook_errors: bool` to `ClientConfig`.
- Added async builders `GeminiClient::with_http_hook` and `GeminiClient::with_fatal_hook_errors`.
- Added private hook invocation helpers (`run_request_hook`, `run_response_hook`) that log non-fatal hook errors as `tracing::warn!` and abort only when `fatal_hook_errors` is enabled.
- Wired request hooks into `generate_with_conversation` and `stream_generate_raw` after request preparation.
- Wired response hooks into `generate_with_conversation` after parsing and inside `stream_responses` after each yielded `ChatResponse`.
- Re-exported `HttpHook` from `src/lib.rs`.
- Added `Error::Hook(String)` variant for hook failures.
- Added `Arc<dyn HttpHook>` implementation of `HttpHook` so trait objects can be shared and observed.
- Added unit tests in `src/client.rs` under `mod client_tests` covering:
  - response hook invocation inside the streaming parser,
  - request hook receiving the prepared prompt,
  - non-fatal hook error swallowing,
  - fatal hook error aborting the operation.

## Files Modified

- `src/client.rs`
- `src/lib.rs`
- `src/errors.rs`

## Verification

- `cargo test --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.

## Notes

- Tests were placed in the existing `client_tests` module because `tests/unit/*.rs` is not configured as an integration-test target in this workspace and the streaming helper is private to `src/client.rs`.
- `from_http_client` was also added as a prerequisite for future plan tests; it is documented under 03-03.
