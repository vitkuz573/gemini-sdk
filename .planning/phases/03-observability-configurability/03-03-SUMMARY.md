# 03-03 Summary: Injectable `reqwest::Client` (REL-04)

## Completed

- Added public constructor `GeminiClient::from_http_client(client, credentials)`.
- Added private constructor `with_http_client` that stores the supplied `reqwest::Client` in `Inner::http` without rebuilding it.
- Refactored `with_config` to build the default client and delegate to `with_http_client`, removing duplicated `Inner` construction.
- Existing constructors (`from_cookie_header`, `from_credentials`, `from_cookies`, `from_hashmap`) continue to build a default client.
- Added integration tests in `tests/http_client.rs`:
  - `injected_client_is_stored`: uses a custom DNS resolver to prove the injected client is used for requests.
  - `from_http_client_rejects_missing_cookies`: validates cookie requirements still apply.

## Files Modified

- `src/client.rs`
- `tests/http_client.rs`

## Verification

- `cargo test --test http_client` passes.
- `cargo test --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
