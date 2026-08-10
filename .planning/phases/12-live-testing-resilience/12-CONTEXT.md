# Phase 12 Context: Live Testing & Backend Resilience

## Background

Milestone v0.2 API Expansion is complete. During live cookie testing of the new `batchexecute` RPCs, the Google backend was observed to intermittently reject correct requests with HTTP 400 and WIZ error frames containing `er`, `di`, and `af.httprm` fields. The same cookies sometimes succeed and sometimes fail, suggesting transient backend-side rejection rather than an actual invalid request or stale session. The SDK currently surfaces these as generic `Error::Api { status: 400, .. }` and does not retry them, making v0.2 API usage unreliable in live environments.

## Problem Statement

1. Transient Google 400 rejections with specific WIZ error frames (`er` + `di` + `af.httprm`) are not retried.
2. Cookie rejection / signed-in state detection can fail silently; the SDK needs to return `Error::NotSignedIn` explicitly.
3. There is no structured way to audit request/response traffic for offline analysis of transient failures.
4. There is no standalone live probe that validates all v0.2 APIs end-to-end and emits a machine-readable report.
5. The live-cookie integration test suite (`tests/real_cookies.rs`) only covers base chat/list/upload, not the new v0.2 surfaces.

## Decisions

- **D-12-01**: Add `RESIL-01`: SDK detects stale/rejected cookies and surfaces `Error::NotSignedIn` instead of generic `Api 400`.
- **D-12-02**: Add `RESIL-02`: SDK retries batchexecute RPCs when the response body matches the transient WIZ 400 pattern (`er` frame with HTTP 400 and `di`/`af.httprm` present).
- **D-12-03**: Add `RESIL-03`: SDK supports optional HAR capture for request/response auditing.
- **D-12-04**: Add `RESIL-04`: A standalone `examples/live_probe.rs` binary exercises all v0.2 APIs (and base chat/list_models) against the live backend, collects telemetry, and writes a JSON report.
- **D-12-05**: Update `TOOL-06` coverage and `TOOL-07` gates to include the new retry/HAR/session-detection tests.

## Non-Goals

- No full browser automation or headless Chrome changes.
- No new `batchexecute` RPCs beyond v0.2 scope.
- No breaking public API changes.

## Known Constraints

- HAR format must follow the W3C JSON structure with `log.version` "1.2", `creator`, and `entries[]` per captured HTTP transaction.
- HAR files must redact cookies and authorization values in `request.cookies`, `request.headers`, and `response.headers`.
- Retry policy must be bounded: max 3 attempts, exponential backoff 500 ms-8 s, cap total elapsed time per call.
- Transient 400 detection is specific to WIZ frames and must not treat all 400s as transient (e.g., malformed payloads should stay permanent).

## Success Criteria

- `cargo test` passes without live cookies using fixtures and mocked transports.
- `cargo clippy --all-targets -- -D warnings` passes.
- `cargo doc --no-deps` builds with no warnings.
- `cargo build --example live_probe` succeeds.
- New unit tests cover:
  - WIZ transient 400 detection,
  - HAR redaction,
  - cookie-rejection → `NotSignedIn` mapping,
  - retry attempt count and backoff behavior.
