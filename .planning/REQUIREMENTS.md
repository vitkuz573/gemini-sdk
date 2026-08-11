# Milestone v0.5 Requirements

## Authentication & Headers

- [ ] **AUTH-01**: `get_usage_stats` sends `Authorization: SAPISIDHASH <ts>_<sha1>` computed from the active credentials.
- [ ] **AUTH-02**: `get_usage_stats` sends `x-goog-authuser: 0` to match the browser request.
- [ ] **AUTH-03**: SAPISIDHASH and authuser headers are scoped only to the `jSf9Qc` RPC path; other batchexecute RPCs remain unchanged.

## Request Shape

- [ ] **REQ-01**: The inner `f.req` payload for `jSf9Qc` matches the captured browser shape in `/home/vitaly/mitm.har`.
- [ ] **REQ-02**: All new protocol literals (header names, payload values) are added to `src/constants.rs` as named `pub(crate)` constants with HAR citations where applicable.

## Response Parsing & API

- [ ] **PARSER-01**: `get_usage_stats` returns a non-empty value when the live account has usage data.
- [ ] **PARSER-02**: The parser preserves the existing null-payload → empty object behavior for accounts with no data.
- [ ] **PARSER-03**: The parser handles the HAR-observed array response shape `[2,[[...]],false]`.
- [ ] **API-01**: `UsageStats` exposes typed accessors for at least daily and total request counts.
- [ ] **API-02**: `UsageStats` keeps a raw `serde_json::Value` accessor as a protocol-drift escape hatch.

## Testing & Verification

- [ ] **TEST-01**: Wiremock fixture tests cover the array-shaped `jSf9Qc` response and the updated request payload.
- [ ] **TEST-02**: Live-cookie integration test (`tests/real_cookies.rs`) verifies `get_usage_stats` returns non-empty stats.
- [ ] **TEST-03**: HAR redaction unit test covers the new `Authorization` header.
- [ ] **TEST-04**: All quality gates pass: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps`.

## Companion CLI

- [ ] **CLI-01**: `gemini-cli usage` surfaces real usage counts after the SDK fix.
- [ ] **CLI-02**: The CLI handles the empty-data case gracefully (no panic, clear message).

## Traceability

| Requirement | Phase |
|-------------|-------|
| AUTH-01 | 1 |
| AUTH-02 | 1 |
| AUTH-03 | 1 |
| REQ-01 | 2 |
| REQ-02 | 1 |
| PARSER-01 | 2 |
| PARSER-02 | 2 |
| PARSER-03 | 2 |
| API-01 | 2 |
| API-02 | 2 |
| TEST-01 | 2 |
| TEST-02 | 3 |
| TEST-03 | 1 |
| TEST-04 | 1, 2, 3 |
| CLI-01 | 3 |
| CLI-02 | 3 |

## Future Requirements (deferred)

- Apply SAPISIDHASH auth to other settings-page RPCs if live testing shows the
  same empty-response problem (e.g., `get_scheduled_prompts`).
- Add OAuth / refresh-token auth as an alternative to cookie strings
  (post-v1.0).

## Out of Scope

- Official Google REST / Vertex AI SDK replacement.
- Telemetry / heartbeat RPCs.
- Full schema modeling of every slot in the undocumented `jSf9Qc` response
  array.
- Changes to the public API beyond the `UsageStats` accessors defined above.
