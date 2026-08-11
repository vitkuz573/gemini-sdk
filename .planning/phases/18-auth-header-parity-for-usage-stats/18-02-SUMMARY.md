---
phase: 18-auth-header-parity-for-usage-stats
plan: 02
subsystem: auth
tags: [auth, sapisidhash, x-goog-authuser, usage-stats, tdd]

requires:
  - phase: 18-auth-header-parity-for-usage-stats
    plan: 01
    provides: build_headers auth_user plumbing and constants
provides:
  - get_usage_stats emits Authorization and x-goog-authuser headers
  - wiremock integration test verifying the headers
affects:
  - 18-auth-header-parity-for-usage-stats

tech-stack:
  added: []
  patterns:
    - "TDD: RED test first, then minimal implementation"
    - "Scoped auth: only get_usage_stats passes auth headers"

key-files:
  created: []
  modified:
    - src/client.rs
    - tests/integration_tests.rs

key-decisions:
  - "Reused existing Credentials::sapisid_hash via credentials_to_sapisid_hash helper"
  - "Passed X_GOOG_AUTHUSER_VALUE constant instead of inline literal"

patterns-established:
  - "RPC-scoped auth headers computed from cookies at request time"

requirements-completed: [AUTH-01, AUTH-02, TEST-04]

coverage:
  - id: D1
    description: "get_usage_stats sends Authorization: SAPISIDHASH <ts>_<sha1>"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "tests/integration_tests.rs#get_usage_stats_sends_auth_headers"
        status: pass
    human_judgment: false
  - id: D2
    description: "get_usage_stats sends x-goog-authuser: 0"
    requirement: AUTH-02
    verification:
      - kind: integration
        ref: "tests/integration_tests.rs#get_usage_stats_sends_auth_headers"
        status: pass
    human_judgment: false

duration: 12min
completed: 2026-08-11
status: complete
---

# Phase 18 Plan 02: Wire Auth Headers into get_usage_stats Summary

**Wired `Authorization: SAPISIDHASH ...` and `x-goog-authuser: 0` into `GeminiClient::get_usage_stats` only, using TDD, and verified with a wiremock integration test.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-08-11T00:08:00Z
- **Completed:** 2026-08-11T00:20:00Z
- **Tasks:** 3 (RED/GREEN/REFACTOR)
- **Files modified:** 2

## Accomplishments
- Added a RED integration test `get_usage_stats_sends_auth_headers` that asserted the two auth headers; it failed as expected.
- Modified `get_usage_stats` to compute `Authorization` from `credentials_to_sapisid_hash` and pass it plus `X_GOOG_AUTHUSER_VALUE` to `build_headers`.
- Verified the test passes and all other tests/clippy pass.
- No refactor commit was needed because imports were already clean and clippy passed.

## Task Commits

1. **Task 1: RED — Add failing integration test** - `19ef739` (test)
2. **Task 2: GREEN — Wire auth headers into get_usage_stats** - `eec8dbf` (feat)
3. **Task 3: REFACTOR — No cleanup needed** - N/A

## Files Created/Modified
- `src/client.rs` - `get_usage_stats` now computes and passes auth headers.
- `tests/integration_tests.rs` - New `get_usage_stats_sends_auth_headers` test.

## Decisions Made
- Used `credentials_to_sapisid_hash(&cookies, &base_url)` to keep auth computation centralized and avoid duplicating origin-stripping logic.

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
- One flaky unrelated test (`refresh_credentials_replaces_cookies_and_clears_session` in `tests/auth_provider.rs`) failed during the first `--all-targets` run but passed on rerun in isolation and again in the full suite. The failure is out of scope for this plan.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 18-03 will add isolation tests proving other RPCs don't get the headers and a HAR redaction test for Authorization.

---
*Phase: 18-auth-header-parity-for-usage-stats*
*Completed: 2026-08-11*
