---
phase: 18-auth-header-parity-for-usage-stats
plan: 01
subsystem: auth
tags: [auth, headers, sapisid, x-goog-authuser]

requires:
  - phase: 17-streamgenerate-slot-hardening
    provides: stable batchexecute client plumbing
provides:
  - x-goog-authuser constants in src/constants.rs
  - build_headers opt-in parameter for auth_user
  - all internal call sites updated with no behavior change
affects:
  - 18-auth-header-parity-for-usage-stats

tech-stack:
  added: []
  patterns:
    - "Centralize new header literals in src/constants.rs before wiring to RPCs"
    - "Opt-in header emission via explicit build_headers parameters"

key-files:
  created: []
  modified:
    - src/constants.rs
    - src/client.rs

key-decisions:
  - "Placed X_GOOG_AUTHUSER constants in pub mod headers next to other x-goog-* header names"
  - "Added auth_user as fourth positional parameter in build_headers before endpoint to minimize churn at call sites"
  - "Did not add constants to deny-list because values are internal protocol literals, not magic strings eliminated from source"

patterns-established:
  - "Opt-in auth header emission: build_headers only emits Authorization/x-goog-authuser when caller passes Some(...)"
  - "Header name and value centralized in constants.rs with doc comments citing browser observation"

requirements-completed: [AUTH-03, REQ-02, TEST-04]

coverage:
  - id: D1
    description: "x-goog-authuser constants exist in src/constants.rs"
    requirement: REQ-02
    verification:
      - kind: unit
        ref: "cargo check"
        status: pass
    human_judgment: false
  - id: D2
    description: "build_headers accepts optional auth_user and emits x-goog-authuser when provided"
    requirement: AUTH-03
    verification:
      - kind: unit
        ref: "cargo check"
        status: pass
      - kind: unit
        ref: "cargo clippy --all-targets -- -D warnings"
        status: pass
      - kind: unit
        ref: "cargo test --all-targets"
        status: pass
    human_judgment: false

duration: 8min
completed: 2026-08-11
status: complete
---

# Phase 18 Plan 01: Auth Header Plumbing Summary

**Added `x-goog-authuser` constants and extended `GeminiClient::build_headers` with an opt-in `auth_user` parameter, updating all call sites without changing any request behavior.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-08-11T00:00:00Z
- **Completed:** 2026-08-11T00:08:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Added `X_GOOG_AUTHUSER` and `X_GOOG_AUTHUSER_VALUE` constants in `src/constants.rs` under `pub mod headers`.
- Extended `GeminiClient::build_headers` signature to accept `auth_user: Option<&str>`.
- Implemented conditional emission of the `x-goog-authuser` header when `auth_user` is `Some`.
- Updated `build_headers_for_test` and all 16 internal `build_headers` call sites to pass `None` for the new parameter.
- Verified with `cargo check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`.

## Task Commits

1. **Task 1: Add x-goog-authuser constants to src/constants.rs + Task 2: Extend build_headers to emit Authorization + x-goog-authuser when opted in** - `ae26297` (feat)

## Files Created/Modified
- `src/constants.rs` - Added `X_GOOG_AUTHUSER` and `X_GOOG_AUTHUSER_VALUE` constants with doc comments.
- `src/client.rs` - Updated `build_headers` signature/body and all call sites; updated `build_headers_for_test` wrapper.

## Decisions Made
- Followed the plan's recommended signature placement: `auth_user` as the fourth positional parameter, before `endpoint`, keeping call-site changes minimal and preserving the existing authorization/endpoint relationship.

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 18-02 can now wire `Some(...)` values for `authorization` and `auth_user` into `get_usage_stats`.
- The opt-in mechanism ensures no other RPC is affected.

---
*Phase: 18-auth-header-parity-for-usage-stats*
*Completed: 2026-08-11*
