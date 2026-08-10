---
phase: 9
name: Locale & Model Config
milestone: v0.2 API Expansion
requirements:
  - LOCALE-01
  - LOCALE-02
  - LOCALE-03
  - LOCALE-04
  - LOCALE-05
---

# Phase 9 Context: Locale & Model Config

## Goal

Expose the four undocumented locale/model configuration `batchexecute` RPCs (`cYRIkd`, `whPPme`, `Te6DCf`, `ku4Jyf`) as thin typed public APIs on `GeminiClient`. All responses are returned as `serde_json::Value` wrappers so future protocol drift does not break consumers.

## Source of Truth

Spike 001 HAR API coverage audit: `.opencode/skills/spike-findings-gemini-sdk/sources/001-har-api-coverage/README.md`

Captured inner payloads from the HAR:

| RPC | Captured inner payload | SDK method |
|-----|------------------------|------------|
| `cYRIkd` | `["ru"]` | `get_locale_tools()` |
| `whPPme` | `["ru", null, [4]]` | `get_model_config()` |
| `Te6DCf` | `[["ru"], [1,2]]` | `get_locale_config()` |
| `ku4Jyf` | `["ru",null,null,null,4,null,null,[1,3,7,17],null,[]]` | `get_tools_config()` |

## Enterprise Constraints

1. **Configurable base_url** — tests must set a mock server URL through public `ClientConfig` builders, not `#[cfg(test)]` backdoors.
2. **Wiremock tests** — every new RPC must have a mocked fixture test covering request shape and response extraction.
3. **No brittle structs** — per `LOCALE-05` all responses are exposed as `serde_json::Value` wrappers to tolerate undocumented shape drift.

## Decisions

- D-09-01: Add a new `src/locale_model_config.rs` module with payload builders and response parsers, mirroring the `src/user_profile.rs` pattern.
- D-09-02: Each RPC receives the client language from `ClientConfig` / session state and constructs the captured inner payload shape verbatim.
- D-09-03: Return `LocaleTools`, `ModelConfig`, `LocaleConfig`, and `ToolsConfig` wrapper structs, each containing a single `value: serde_json::Value` field with an accessor.
- D-09-04: Use the existing `batchexecute_rpc` helper (or the manual batchexecute request pattern used by user_profile.rs) for transport; no new transport code.
- D-09-05: Unit tests live in `src/locale_model_config.rs`; integration tests live in `tests/integration_tests.rs` using Wiremock fixtures.
- D-09-06: Fixtures are stored in `tests/fixtures/` and include both the XSSI prefix and a synthetic payload that exercises the parser path.

## Deferred Ideas

None.

## Risks

- Undocumented RPC payloads may drift; `serde_json::Value` wrappers mitigate consumer breakage.
- Tests rely on `inner_session_for_tests()` to inject session state; this is an existing public test helper, not a new backdoor.

## Affected Subsystems

- `src/locale_model_config.rs` (new)
- `src/client.rs` (four new public methods)
- `src/lib.rs` (module + re-exports)
- `tests/integration_tests.rs` (four new wiremock tests)
- `tests/fixtures/` (four new fixture files)

## Traceability

| Requirement | Decision | Verification |
|-------------|----------|--------------|
| LOCALE-01 | D-09-01, D-09-04 | Unit + integration tests |
| LOCALE-02 | D-09-01, D-09-04 | Unit + integration tests |
| LOCALE-03 | D-09-01, D-09-04 | Unit + integration tests |
| LOCALE-04 | D-09-01, D-09-04 | Unit + integration tests |
| LOCALE-05 | D-09-03 | Parser unit tests assert raw Value contents |
