---
phase: 15-infrastructure-constants
plan: 01
type: summary
completed: 2026-08-11
---

# Phase 15 Plan 01 Summary: Infrastructure Constants

## Objective
Centralize infrastructure-level strings (HTTP headers/values, user agents, HAR/redaction strings, transient WIZ 400 markers, tracing/metric names, browser attestation CDP strings, and tool schema keys) into `src/constants.rs`, then refactor all infrastructure modules to consume them.

## What Changed

### New `src/constants.rs` modules
- `headers` — header names, `sec-ch-ua` values, WAA/OGADS header names, `x-goog-ext-*` prefix.
- `user_agents` — `BROWSER_LIKE` (Chrome 146) and `UPLOAD_BROWSER_LIKE` (Chrome 133).
- `har` — HAR version/creator/MIME types, redaction value, cookie/auth pattern names.
- `transient` — WIZ 400 markers (`"er"`, `"di"`, `"af.httprm"`) and sign-in redirect title substring.
- `tracing_names` — span names for every public client method, metric names, attribute keys.
- `attestation` — CDP method names, Chrome domain/path, navigation template, selectors, flags, profile path, timeouts.
- `tool_schema` — JSON Schema keys (`type`, `object`, `properties`, `required`, `name`, `parameters`).
- `http_methods` — `GET`, `POST`.
- `auth` — `MISSING_LEGACY_COOKIES_ADVICE` diagnostic suffix.

### Refactored modules
- `src/client.rs`
  - Replaced local `USER_AGENT`, `X_CLIENT_DATA`, `WEB_BASE_URL`, `WAA_BASE_URL`, `OGADS_BASE_URL` with constants.
  - `build_headers` now uses `constants::headers` for all names/values.
  - `waa_create` and `ogads_get_async_data` use header/MIME constants.
  - `fetch_app_page` and `accept_consent_and_refresh` use header constants and `APP_LANGUAGE_PATH_TEMPLATE`.
  - `build_not_signed_in_error` uses `auth::MISSING_LEGACY_COOKIES_ADVICE`.
  - All `#[tracing::instrument]` span names and `operation` fields use `tracing_names` constants.
  - HAR recording uses `http_methods::GET`/`POST`.
  - Retry metric uses `tracing_names::METRIC_RETRIES` and `OPERATION`.
- `src/upload.rs`
  - Replaced local `USER_AGENT` with `user_agents::UPLOAD_BROWSER_LIKE`.
  - Replaced header literals with `constants::headers` items.
- `src/har.rs`
  - Replaced HAR version/creator/MIME/redaction literals with `constants::har`.
  - `is_secret_header` and cookie redaction use `constants::headers` and `constants::har`.
- `src/transient_400.rs`
  - Uses `transient::ER_MARKER`, `DI_MARKER`, `HTTPRM_MARKER`.
- `src/metrics.rs`
  - Tests use `tracing_names::METRIC_REQUESTS`, `METRIC_REQUEST_LATENCY`, `STATUS`, `OPERATION`.
- `src/attestation.rs`
  - CDP method names, domain/path, navigation URL, selectors, Chrome flags, profile path, and timeouts now use `constants::attestation`.
- `src/tool.rs`
  - `tool_declaration` and schema tests use `constants::tool_schema` keys.

## Public API Changes
The user explicitly allowed public API changes if they improved constant centralization. The only public API change is the addition of one hidden test helper:

- `GeminiClient::build_headers_for_test` — `pub(crate)` test helper for header validation.

No existing public method signatures, struct fields, or enum variants were changed or removed.

## Verification
- `cargo test --all-targets` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo doc --no-deps` — passed with no warnings.

## Artifacts
- Updated `src/constants.rs` with centralized infrastructure constants.
- Refactored `src/client.rs`, `src/upload.rs`, `src/har.rs`, `src/transient_400.rs`, `src/metrics.rs`, `src/attestation.rs`, `src/tool.rs`.
- This summary and `15-01-VERIFICATION.md`.
