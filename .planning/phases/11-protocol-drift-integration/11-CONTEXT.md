---
phase: 11
name: Protocol Drift & Integration
milestone: v0.2 API Expansion
requirements:
  - DRIFT-01
  - TOOL-06
  - TOOL-07
created: 2026-08-10
---

# Phase 11 Context: Protocol Drift & Integration

## Goal

Close the v0.2 API Expansion milestone by applying the one known protocol drift fix, adding a runnable example that demonstrates the new v0.2 APIs, and ensuring the final quality gates are green.

## Locked Decisions

- D-01: The default `x-client-data` header constant MUST be updated from `CI7yygE=` to `CNeOywE=` per spike 001 (`~/mitm.har`, 135 MB capture). The constant is declared in `src/client.rs` as `X_CLIENT_DATA` and is used in `build_headers`, `waa_create`, and `ogads_get_async_data`.
- D-02: At least one runnable example binary MUST be added under `examples/` demonstrating the new v0.2 public APIs (`get_user_info`, `get_last_selected_mode`, `get_locale_tools`, `get_model_config`, `get_usage_stats`, etc.). The example MUST compile with `cargo build --examples` and be wired into `Cargo.toml`.
- D-03: Every new RPC exposed in v0.2 Phases 7-10 MUST have a mocked fixture test. The audit list is derived from REQUIREMENTS.md traceability and existing integration tests.
- D-04: Final quality gates MUST pass: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo doc --no-deps`.

## Deferred Ideas

- Adding a separate `CHANGELOG.md` update for v0.2 is deferred to milestone closure.
- Expanding examples to cover image upload, streaming, or tools is deferred; this phase focuses on v0.2 RPC coverage.
- Live HAR-based fixture refresh automation remains a future research item.

## the agent's Discretion

- Which specific v0.2 APIs to include in the example (recommended: a read-only tour of user info, locale tools, model config, and usage stats).
- Whether to add one combined example or multiple focused examples; one combined example is recommended to keep the phase scope contained.
- How to structure the fixture-test audit table in RESEARCH.md.
