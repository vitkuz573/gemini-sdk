---
phase: 10-settings-pages
plan: 01
subsystem: api
tags: [rust, gemini-sdk, batchexecute, serde_json, wiremock, settings]

requires:
  - phase: 09-locale-model-config
    provides: Value-wrapper pattern and batchexecute transport precedent for thin typed RPC facades

provides:
  - `src/settings.rs` module with payload builders, parsers, and wrapper types
  - `GeminiClient::get_usage_stats()` sending RPC `jSf9Qc` to `/usage`
  - `GeminiClient::get_scheduled_prompts()` sending RPC `XPSWpd` to `/scheduled`
  - Wiremock integration tests with synthetic fixtures for both RPCs
  - Re-exports of `UsageStats` and `ScheduledPrompts` in `src/lib.rs`

affects:
  - phase 11 protocol-drift-and-integration (final quality gate and example coverage)

tech-stack:
  added: []
  patterns:
    - "Mirrored `locale_model_config.rs` wrapper/payload/parser layout for new RPC surface"
    - "Batchexecute request construction via `send_with_retry` and `build_batchexecute_body_for_rpc`"
    - "Configurable `base_url` + Wiremock fixtures for integration tests"

key-files:
  created:
    - src/settings.rs
    - tests/fixtures/jSf9Qc_usage_stats.txt
    - tests/fixtures/XPSWpd_scheduled_prompts.txt
    - .planning/phases/10-settings-pages/10-01-SUMMARY.md
  modified:
    - src/client.rs
    - src/lib.rs
    - tests/integration_tests.rs

key-decisions:
  - "Reused the exact `locale_model_config.rs` module pattern to keep new RPC surfaces consistent."
  - "Returned opaque `serde_json::Value` wrappers (`UsageStats`, `ScheduledPrompts`) per SETTINGS-03 to tolerate undocumented shape drift."
  - "Sent captured inner payload shape `[]` verbatim for both RPCs, matching spike 001 HAR observations."
  - "Used `/usage` and `/scheduled` source paths respectively, distinct from `/app/{id}` and `/`."

requirements-completed:
  - SETTINGS-01
  - SETTINGS-02
  - SETTINGS-03

coverage:
  - id: D1
    description: "SDK exposes get_usage_stats() sending RPC jSf9Qc with source-path /usage"
    requirement: SETTINGS-01
    verification:
      - kind: integration
        ref: "tests/integration_tests.rs#get_usage_stats_returns_value"
        status: pass
      - kind: unit
        ref: "src/settings.rs#parse_usage_stats_extracts_payload"
        status: pass
    human_judgment: false

  - id: D2
    description: "SDK exposes get_scheduled_prompts() sending RPC XPSWpd with source-path /scheduled"
    requirement: SETTINGS-02
    verification:
      - kind: integration
        ref: "tests/integration_tests.rs#get_scheduled_prompts_returns_value"
        status: pass
      - kind: unit
        ref: "src/settings.rs#parse_scheduled_prompts_extracts_payload"
        status: pass
    human_judgment: false

  - id: D3
    description: "Settings responses are returned as typed wrappers over serde_json::Value"
    requirement: SETTINGS-03
    verification:
      - kind: unit
        ref: "src/settings.rs#parse_usage_stats_extracts_payload"
        status: pass
      - kind: unit
        ref: "src/settings.rs#parse_scheduled_prompts_extracts_payload"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-08-10
status: complete
---

# Phase 10 Plan 01: Settings Pages Summary

**Added `get_usage_stats` and `get_scheduled_prompts` as thin typed batchexecute RPC facades with Wiremock fixtures and opaque `serde_json::Value` wrappers.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-08-10T09:22:00Z
- **Completed:** 2026-08-10T09:37:00Z
- **Tasks:** 4
- **Files modified:** 5

## Accomplishments
- Created `src/settings.rs` with `UsageStats` and `ScheduledPrompts` wrappers, payload builders, and batchexecute response parsers.
- Wired public `GeminiClient::get_usage_stats()` and `GeminiClient::get_scheduled_prompts()` methods using the existing transport pattern.
- Added two Wiremock integration tests with synthetic fixtures verifying request RPC IDs and response parsing.
- Passed `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo doc --no-deps`.

## Task Commits

1. **Task 1: Create settings module with wrappers, builders, and parsers** — `208bb00` (feat)
2. **Task 2: Wire two public methods on GeminiClient** — `fb3db12` (feat)
3. **Task 3: Add wiremock fixtures and integration tests** — `64cc9f1` (test)
4. **Task 4: Quality gates and final verification** — verified at `64cc9f1`

## Files Created/Modified
- `src/settings.rs` — New module: wrappers, payload builders, parsers, and unit tests.
- `src/client.rs` — Added `get_usage_stats()` and `get_scheduled_prompts()` methods.
- `src/lib.rs` — Declared `pub mod settings` and re-exported `UsageStats`/`ScheduledPrompts`.
- `tests/integration_tests.rs` — Added Wiremock integration tests.
- `tests/fixtures/jSf9Qc_usage_stats.txt` — Fixture for usage-stats RPC.
- `tests/fixtures/XPSWpd_scheduled_prompts.txt` — Fixture for scheduled-prompts RPC.

## Decisions Made
- Followed the `locale_model_config.rs` pattern exactly to maintain consistency across v0.2 RPC surfaces.
- Kept responses as opaque `serde_json::Value` wrappers per SETTINGS-03 to avoid brittle typed structs for undocumented shapes.
- Sent the captured inner payload `[]` verbatim for both RPCs, matching the spike 001 HAR capture.
- Set `source-path` to `/usage` and `/scheduled` respectively, not `/` or `/app/{id}`.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## Threat Flags

None. No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond reusing the existing batchexecute transport and cookie handling.

## Known Stubs

None. Both fixtures contain synthetic but non-empty payloads, and parsers return the parsed `Value`; no hardcoded empty values or placeholder text flow to consumers.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 10 is complete and ready for Phase 11 (Protocol Drift & Integration).
- Phase 11 should update `x-client-data` from `CI7yygE=` to `CNeOywE=`, add a runnable example for new APIs, and run the final quality gate.

## Self-Check: PASSED

- `src/settings.rs` exists.
- `tests/fixtures/jSf9Qc_usage_stats.txt` exists.
- `tests/fixtures/XPSWpd_scheduled_prompts.txt` exists.
- Commits `208bb00`, `fb3db12`, `64cc9f1` exist in git history.

---
*Phase: 10-settings-pages*
*Completed: 2026-08-10*
