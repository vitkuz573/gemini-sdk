# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- New `waa` module for browserless WAA slot-3 token generation, with
  `WaaGenerator`, `Signature`, and `WrapperFragment` types.
- Bundled default wrapper cache at `src/waa/data/default_wrappers.json`.
- Documentation at `docs/WAA.md` covering layout, API, cache format, and
  operational limitations.

## [0.1.0] - 2026-08-10

### Added

- Cookie-based authentication with `Credentials`, `Cookies`, and `CredentialsProvider` (Phase 01-02).
- Injectable `reqwest::Client` via `GeminiClient::from_http_client` (Phase 03-03).
- Request/response `HttpHook` trait for observability (Phase 03-01).
- `tracing` spans on all public async operations with secret-safe fields (Phase 03-02).
- Upload progress stream via `GeminiClient::upload_with_progress` and `UploadEvent` (Phase 03-04).
- Audio and video upload support alongside inline images (Phase 04-01).
- Session save/restore helpers (`Snapshot`, `save_session`, `restore_session`) and `Conversation::save`/`restore` (Phase 04-02).
- Function-calling API: `Tool` trait, `ToolCall`, `ToolResult`, `ToolError`, and `generate_with_tools` (Phase 05-01, 05-02).
- Automatic credential refresh and retry-on-auth-error via `with_provider` and `with_refresh_on_auth_error` (Phase 05-03).
- Feature-gated metrics facade (`MetricsRecorder`, `NoOpMetricsRecorder`, `OpenTelemetryRecorder` behind the `metrics` feature) (Phase 05-03).
- Optional browser attestation module behind the `browser-attestation` feature.

### Changed

- **Breaking:** `GeminiClient::with_language`, `with_max_retries`, and `with_timeout` are now `async` and use `tokio::sync::RwLock` (Phase 02-01).
- **Breaking:** `Error::AttestationFailed` is now raised when WAA/ogads attestation fails instead of silently falling back to synthetic context (Phase 02-01).
- `Error::is_transient` now inspects `reqwest::Error::status()` so transport-level 429/5xx errors are retried.

### Deprecated

- None.

### Removed

- Leftover `clippy::too_many_lines` from the crate-level allow list after adding explanatory `REASON` comments to the remaining suppressions.

### Fixed

- Consent cookie merging now persists `SOCS` cookies returned by the consent save flow into the client cookie jar (Phase 02-01).
- Robust HTML extraction fallbacks for `SNlM0e`, build label, session id, and push id across multiple Google page shapes (Phase 03-05).

### Security

- Credential `Debug` output is fully redacted (`<redacted>` / `(empty)`) with no secret prefixes or lengths leaked (Phase 01-02).
- `Snapshot` strings contain recoverable credentials; callers are responsible for secure storage (Phase 04-02).

## Release checklist

To publish this crate after this phase:

1. Run all verification commands:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-targets --all-features
   cargo doc --no-deps --all-features
   cargo publish --dry-run --all-features
   ```
2. Log in to crates.io:
   ```bash
   cargo login <YOUR_CRATES_IO_TOKEN>
   ```
3. Publish:
   ```bash
   cargo publish --all-features
   ```
4. Tag the release and push to GitHub:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
5. Verify documentation builds on [docs.rs](https://docs.rs/gemini-sdk).

## Migration guide

See [`docs/migration-v0-to-v1.md`](docs/migration-v0-to-v1.md) for breaking changes introduced on the path to v1.0.

[Unreleased]: https://github.com/vitkuz573/gemini-sdk/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/vitkuz573/gemini-sdk/releases/tag/v0.1.0
