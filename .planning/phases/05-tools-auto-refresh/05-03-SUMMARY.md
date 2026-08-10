# Phase 5, Plan 3 Summary: Auto Cookie Refresh and Metrics Facade

**Phase:** 05-tools-auto-refresh
**Plan:** 05-03
**Status:** Completed
**Date:** 2026-08-10

## Objective
Implement explicit credential refresh, automatic retry on auth failure, and a feature-gated metrics facade. This plan delivers ADV-03 and OBS-03 while keeping the SDK lightweight by default.

## Files Changed
- `src/client.rs`
  - Added `provider: Mutex<Option<Arc<dyn CredentialsProvider>>>` to `Inner`.
  - Added `metrics_recorder: Option<Arc<dyn MetricsRecorder>>` to `ClientConfig`.
  - Added `GeminiClient::from_provider` provider storage.
  - Added `GeminiClient::with_provider` async builder.
  - Added `GeminiClient::with_metrics` async builder.
  - Added `GeminiClient::refresh_credentials` (replaces cookies, resets session, re-runs `init_session`).
  - Added `execute_generate` helper and retry-on-`NotSignedIn` path gated by `refresh_on_auth_error`.
  - `generate_with_conversation` and `ChatBuilder::send_message_with_content` now route through `execute_generate`.
- `src/auth.rs`
  - Implemented `CredentialsProvider` for `Arc<dyn CredentialsProvider>` so stored providers can be passed to `refresh_credentials`.
- `src/lib.rs`
  - Declared `pub mod metrics`.
  - Re-exported `MetricsRecorder`, `NoOpMetricsRecorder`, and `OpenTelemetryRecorder` behind the `metrics` feature.
- `src/metrics.rs` (new)
  - `MetricsRecorder` trait (object-safe, `Send + Sync`).
  - `NoOpMetricsRecorder` (default, zero overhead).
  - `OpenTelemetryRecorder` gated by `#[cfg(feature = "metrics")]`.
  - Unit tests with a counting mock recorder.
- `Cargo.toml`
  - Added optional `opentelemetry = "0.32"` dependency.
  - Added `metrics = ["dep:opentelemetry"]` feature.
- `tests/auth_provider.rs`
  - Added `refresh_credentials_replaces_cookies_and_clears_session` test.
- `tests/metrics.rs` (new)
  - Integration tests for `with_metrics` and `NoOpMetricsRecorder`.

## Verification
- `cargo test --all-targets` — passed
- `cargo test --all-targets --features metrics` — passed
- `cargo clippy --all-targets -- -D warnings` — passed
- `cargo clippy --all-targets --features metrics -- -D warnings` — passed
- `cargo doc --no-deps` — passed
- `cargo doc --no-deps --features metrics` — passed

## Commit
`feat(auth,metrics): refresh_credentials, retry-on-auth-error, MetricsRecorder facade (05-03)`

## Notes
- `refresh_credentials` is explicit; callers schedule refresh when needed.
- `ChatBuilder::with_refresh_on_auth_error(true)` enables one retry on `NotSignedIn` when a provider is registered.
- Metrics are no-op by default. The `metrics` feature enables the OpenTelemetry-backed recorder without adding the dependency to default builds.
