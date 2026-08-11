---
phase: 19-payload-parser-alignment
plan: 01
subsystem: parser
tags: [parser, usage-stats, serde, tdd]

requires:
  - phase: 18-auth-header-parity-for-usage-stats
    plan: 01
    provides: auth header plumbing
  - phase: 18-auth-header-parity-for-usage-stats
    plan: 02
    provides: get_usage_stats auth header wiring
provides:
  - UsageStats array-shaped response parser
  - UsageStats::requests_today() and requests_total() accessors
  - HAR-derived jSf9Qc fixture
affects:
  - 19-payload-parser-alignment
  - 20-live-verification-cli-contract

tech-stack:
  added: []
  patterns:
    - "TDD: RED fixture/tests, GREEN implementation, REFACTOR constants"
    - "Option<u64> accessors tolerate undocumented response drift"

key-files:
  created: []
  modified:
    - src/settings.rs
    - tests/fixtures/jSf9Qc_usage_stats.txt
    - tests/integration_tests.rs

key-decisions:
  - "Mapped array bucket index 1 to requests_total and index 2 to requests_today based on HAR observation"
  - "Preserved UsageStats::value() as raw escape hatch"
  - "Kept build_get_usage_stats_payload() unchanged because it already matches HAR"

patterns-established:
  - "Named constants for undocumented response array indices with HAR citations"

requirements-completed: [REQ-01, PARSER-01, PARSER-02, PARSER-03, API-01, API-02, TEST-01, TEST-04]

coverage:
  - id: D1
    description: "Parser handles array-shaped jSf9Qc response"
    requirement: PARSER-03
    verification:
      - kind: unit
        ref: "src/settings.rs#parse_usage_stats_array_response_returns_typed_counts"
        status: pass
    human_judgment: false
  - id: D2
    description: "UsageStats exposes requests_today and requests_total accessors"
    requirement: API-01
    verification:
      - kind: integration
        ref: "tests/integration_tests.rs#get_usage_stats_returns_value"
        status: pass
    human_judgment: false
  - id: D3
    description: "Null payloads still return empty object"
    requirement: PARSER-02
    verification:
      - kind: unit
        ref: "src/settings.rs#parse_usage_stats_null_payload_returns_empty_object"
        status: pass
    human_judgment: false
  - id: D4
    description: "Fixture matches captured browser response"
    requirement: REQ-01
    verification:
      - kind: integration
        ref: "tests/fixtures/jSf9Qc_usage_stats.txt"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-08-11
status: complete
---

# Phase 19 Plan 01: Payload & Parser Alignment Summary

**Aligned the `jSf9Qc` response parser with the live array-shaped response and added typed `requests_today()` / `requests_total()` accessors while preserving the raw `value()` escape hatch.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-08-11T00:00:00Z
- **Completed:** 2026-08-11T00:15:00Z
- **Tasks:** 3 (RED/GREEN/REFACTOR)
- **Files modified:** 3

## Accomplishments
- Replaced the synthetic `jSf9Qc_usage_stats.txt` fixture with a HAR-derived array payload from `/home/vitaly/mitm.har` entry 508.
- Added RED unit and integration tests asserting `requests_total() == Some(47284)` and `requests_today() == Some(2333)`.
- Implemented `UsageStats::requests_total()` and `requests_today()` using safe `Option`-chaining over the array shape.
- Added named constants `USAGE_STATS_DATA_INDEX`, `USAGE_STATS_TOTAL_BUCKET`, and `USAGE_STATS_TODAY_BUCKET` with doc comments.
- Preserved null-payload → empty object behavior and the existing `value()` accessor.
- Verified `cargo test --all-targets` and `cargo clippy --all-targets -- -D warnings` pass.

## Task Commits

1. **Task 1: RED — Update fixture and add failing tests** - `c42e1b7` (test)
2. **Task 2: GREEN — Implement UsageStats accessors and array parser** - `fb0c0c1` (feat)
3. **Task 3: REFACTOR — Named constants (done inline with GREEN)** - N/A

## Files Created/Modified
- `src/settings.rs` - Added typed accessors, parser helper, and named constants.
- `tests/fixtures/jSf9Qc_usage_stats.txt` - Replaced with HAR-derived array response.
- `tests/integration_tests.rs` - Updated assertions to use typed accessors.

## Decisions Made
- Mapped bucket index 1 to total and index 2 to today based on the captured HAR response; accessors return `Option<u64>` to tolerate future drift.
- Did not change `build_get_usage_stats_payload()` because the decoded request (`[]`) already matches the browser.

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 20 can perform live verification against real cookies and update the companion CLI.

---
*Phase: 19-payload-parser-alignment*
*Completed: 2026-08-11*
