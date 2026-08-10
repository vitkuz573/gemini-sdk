---
phase: 10
name: Settings Pages
milestone: v0.2 API Expansion
requirements:
  - SETTINGS-01
  - SETTINGS-02
  - SETTINGS-03
---

# Phase 10 Context: Settings Pages

## Goal

Expose the two undocumented settings-page `batchexecute` RPCs (`jSf9Qc` for usage stats, `XPSWpd` for scheduled prompts) as thin typed public APIs on `GeminiClient`. Per SETTINGS-03, responses are returned as typed wrappers over `serde_json::Value` so future protocol drift does not break consumers.

## Source of Truth

Spike 001 HAR API coverage audit: `.opencode/skills/spike-findings-gemini-sdk/sources/001-har-api-coverage/README.md`

Captured inner payloads from the HAR:

| RPC | Source path | Inner payload (decoded) | SDK method |
|-----|-------------|-------------------------|------------|
| `jSf9Qc` | `/usage` | `[]` | `get_usage_stats()` |
| `XPSWpd` | `/scheduled` | `[]` | `get_scheduled_prompts()` |

## Enterprise Constraints

1. **Configurable base_url** — tests must set a mock server URL through public `ClientConfig` builders, not `#[cfg(test)]` backdoors.
2. **Wiremock tests** — each new RPC must have a mocked fixture test covering request shape and response extraction.
3. **Typed wrappers over `serde_json::Value`** — per SETTINGS-03, responses are wrapped in dedicated structs with an accessor for the inner `Value` rather than returned as bare JSON.

## Decisions

- **D-10-01**: Add a new `src/settings.rs` module with payload builders and response parsers, mirroring the `src/locale_model_config.rs` pattern.
- **D-10-02**: Each RPC sends the captured inner payload shape `[]` verbatim using the existing batchexecute request pattern with `source-path` set to the page path (`/usage` and `/scheduled` respectively).
- **D-10-03**: Return `UsageStats` and `ScheduledPrompts` wrapper structs, each containing a single `value: serde_json::Value` field with a `value()` accessor.
- **D-10-04**: Use the existing `batchexecute` transport code in `src/client.rs` (manual `send_with_retry` pattern from `get_locale_tools`, etc.) for both RPCs; no new transport code.
- **D-10-05**: Unit tests live in `src/settings.rs`; integration tests live in `tests/integration_tests.rs` using Wiremock fixtures.
- **D-10-06**: Fixtures are stored in `tests/fixtures/` and include both the XSSI prefix and a synthetic payload that exercises the parser path.

## Deferred Ideas

None.

## Risks

- Undocumented RPC payloads may drift; `serde_json::Value` wrappers mitigate consumer breakage.
- Tests rely on `inner_session_for_tests()` to inject session state; this is an existing public test helper, not a new backdoor.

## Affected Subsystems

- `src/settings.rs` (new)
- `src/client.rs` (two new public methods)
- `src/lib.rs` (module + re-exports)
- `tests/integration_tests.rs` (two new wiremock tests)
- `tests/fixtures/` (two new fixture files)

## Traceability

| Requirement | Decision | Verification |
|-------------|----------|--------------|
| SETTINGS-01 | D-10-01, D-10-04 | Unit + integration tests |
| SETTINGS-02 | D-10-01, D-10-04 | Unit + integration tests |
| SETTINGS-03 | D-10-03 | Parser unit tests assert raw Value contents |
