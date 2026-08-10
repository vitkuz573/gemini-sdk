---
phase: 10
name: Settings Pages
---

# Phase 10 Research: Settings Pages

## Existing Patterns to Reuse

### Transport Pattern (`src/client.rs`)

`get_locale_tools()`, `get_model_config()`, `get_locale_config()`, and `get_tools_config()` in `src/client.rs` demonstrate the exact request pattern for `/` source-path `batchexecute` RPCs. For the settings-page RPCs captured on `/usage` and `/scheduled`, the `source-path` parameter should be set to the respective path to match the HAR capture:

- Acquire `base_url` from `ClientConfig`.
- Build params `rpcids`, `source-path`, `hl`, `_reqid`, `rt="c"`, optional `bl`, optional `f.sid`.
  - `get_usage_stats`: `source-path = "/usage"`
  - `get_scheduled_prompts`: `source-path = "/scheduled"`
- Build body with `crate::proto::build_batchexecute_body_for_rpc(rpc_id, "[]", access_token)`.
- Send via `self.send_with_retry` with cookie header.
- Parse text response.

This pattern should be reused for both settings-page RPCs.

### Module Pattern (`src/locale_model_config.rs`)

`src/locale_model_config.rs` shows the established module layout for `serde_json::Value` wrappers:

- `pub(crate) const {RPC}_RPC_ID: &str` constants.
- Public payload builder functions returning `serde_json::Value`.
- Public response parser functions returning typed wrapper structs.
- `extract_rpc_entry` / `extract_payload_str` helpers to unwrap batchexecute envelopes.
- `#[cfg(test)] mod tests` with payload-shape and parser unit tests.

The settings module should mirror this exactly.

### Fixture Pattern (`tests/integration_tests.rs`)

Wiremock tests:

- Start `MockServer`, mount a `POST /_/BardChatUi/data/batchexecute` responder returning a fixture file.
- Build a client with `.with_base_url(&mock_uri)` and `.with_max_retries(0)`.
- Inject `build_label`, `session_id`, and `access_token` via `inner_session_for_tests()` to skip live `/app` init.
- Assert on parsed response contents and request body.

Fixture files live in `tests/fixtures/` and include the `)] } '\n\n` XSSI prefix followed by the batchexecute JSON array.

## Standard Stack

- `serde_json::Value` for all response payloads (per SETTINGS-03).
- `reqwest` + `wiremock` for integration tests.
- `tracing` for instrument spans.

## Don't Hand-roll

- Do not invent new transport code; reuse `send_with_retry` and `build_batchexecute_body_for_rpc`.
- Do not add new dependencies.
- Do not create deep typed structs for the undocumented responses; wrappers only.

## Common Pitfalls

- Source path for these RPCs is `/usage` and `/scheduled` respectively, not `/app/{conversation_id}` or `/`.
- The XSSI prefix must be stripped before parsing the batchexecute envelope.
- `batchexecute` responses may be wrapped in an extra array; `extract_rpc_entry` already handles both shapes.
- `serde_json::Value` wrappers must expose a clear accessor (`value()`) but not leak internal mutability.

## Package Legitimacy Audit

No new packages are required for this phase. Existing dev-dependencies (`wiremock`, `tokio-test`) are already validated.

| Package | Status | Notes |
|---------|--------|-------|
| wiremock | existing | Used in Phases 7, 8, and 9 |
| tokio-test | existing | Used throughout |

## Architectural Responsibility Map

| Layer | Responsibility | Phase 10 addition |
|-------|---------------|-------------------|
| `src/settings.rs` | Payload builders, response parsers, wrapper types | New module |
| `src/client.rs` | Public async methods on `GeminiClient` | Two methods |
| `src/lib.rs` | Module declaration and re-exports | Add `settings` |
| `tests/` | Fixture-based integration tests | Two tests + fixtures |
