---
phase: 09
plan: 01
subsystem: Locale & Model Config
tags: [locale, model-config, batchexecute, wiremock, serde_json]
requires:
  - 08-01-PLAN.md
provides:
  - 10-01-PLAN.md
affects:
  - src/client.rs
  - src/lib.rs
  - tests/integration_tests.rs
tech_stack:
  added: []
  patterns:
    - Thin typed facade over batchexecute transport
    - serde_json::Value wrappers for protocol drift tolerance
    - Wiremock fixture tests with configurable base_url
key_files:
  created:
    - src/locale_model_config.rs
    - tests/fixtures/cYRIkd_locale_tools.txt
    - tests/fixtures/whPPme_model_config.txt
    - tests/fixtures/Te6DCf_locale_config.txt
    - tests/fixtures/ku4Jyf_tools_config.txt
  modified:
    - src/client.rs
    - src/lib.rs
    - tests/integration_tests.rs
decisions:
  - Reused the user_profile.rs module pattern for payload builders, response parsers, and wrapper types.
  - Returned opaque serde_json::Value wrappers for all four RPCs to tolerate undocumented shape drift.
  - Used existing batchexecute transport (build_batchexecute_body_for_rpc, send_with_retry) instead of adding new transport code.
metrics:
  duration: 11m
  completed_date: 2026-08-10
  tasks: 4
  files: 8
status: complete
---

# Phase 9 Plan 1: Locale & Model Config Summary

Added four thin public APIs on `GeminiClient` for the undocumented locale/model configuration batchexecute RPCs (`cYRIkd`, `whPPme`, `Te6DCf`, `ku4Jyf`). All responses are wrapped in `serde_json::Value` containers to avoid brittle structs and preserve forward compatibility against protocol drift.

## What Was Delivered

- `src/locale_model_config.rs` with:
  - `LocaleTools`, `ModelConfig`, `LocaleConfig`, `ToolsConfig` wrapper structs.
  - Payload builders matching the captured inner shapes from spike 001.
  - Response parsers that strip the XSSI prefix, locate the RPC entry, and return the inner payload as `serde_json::Value`.
  - Unit tests covering payload shapes, parser extraction, and wrapped-array envelopes.
- Four public async methods on `GeminiClient`:
  - `get_locale_tools()`
  - `get_model_config()`
  - `get_locale_config()`
  - `get_tools_config()`
- Module and type re-exports in `src/lib.rs`.
- Four Wiremock integration tests with fixture files in `tests/fixtures/`.
- Quality gates pass: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps`.

## Verification

| Gate | Command | Result |
|------|---------|--------|
| Unit tests | `cargo test --lib locale_model_config` | 12 passed |
| Integration tests | `cargo test --test integration_tests locale` | 4 passed |
| Full test suite | `cargo test --all-targets` | 222 passed, 2 ignored |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean |
| Docs | `cargo doc --no-deps` | clean |

## Commits

- `8b2e0ba` — feat(09-01): add locale and model config module with builders, parsers, and wrappers
- `36180a6` — feat(09-01): wire get_locale_tools, get_model_config, get_locale_config, get_tools_config on GeminiClient
- `0fe273c` — test(09-01): add wiremock integration tests and fixtures for locale/model config RPCs

## Deviations from Plan

None - plan executed exactly as written.

## Threat Flags

No new threat surface beyond the existing batchexecute transport. Responses remain opaque JSON values; no PII is parsed or logged at info level.

## Known Stubs

None.

## Self-Check: PASSED

- `src/locale_model_config.rs` exists.
- Fixture files exist.
- Commits `8b2e0ba`, `36180a6`, `0fe273c` exist.
