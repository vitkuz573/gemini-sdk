---
phase: 01-stabilize-v0-1-core
plan: 04
subsystem: reliability
tags: [rust, retry, backoff, clippy, docs, publish, cargo]

requires:
  - phase: 01-stabilize-v0-1-core
    provides: public API surface, auth provider trait, chat/media tests

provides:
  - Verified retry/backoff behavior with unit tests.
  - Extended Error::is_transient to cover reqwest transport errors with transient HTTP statuses.
  - Green clippy gate on all targets.
  - Warning-free rustdoc build.
  - Publishable Cargo.toml with readme field and exclude list.

affects:
  - 02-reliability-protocol-hardening
  - 06-v1-0-release

tech-stack:
  added:
    - http 1.5 (dev-dependency for test reqwest error construction)
  patterns:
    - Documented retry backoff constants in retry.rs.
    - Centralized transient classification in Error::is_transient.

key-files:
  created:
    - .planning/intel/API-SURFACE.md
  modified:
    - src/retry.rs
    - src/errors.rs
    - Cargo.toml

key-decisions:
  - Extended Error::is_transient to inspect reqwest::Error::status() so transport-level 429/5xx errors are retried, matching existing Api variant behavior.
  - Added http as a dev-dependency only; no new runtime dependency was required.
  - Used Cargo.toml exclude array to keep .planning, .opencode, docs, benches, tests, examples, and tooling config files out of the published crate.

patterns-established:
  - "Retry test harness: use http::Response to build reqwest errors with specific statuses and assert retry count via AtomicUsize."
  - "Transient classification covers both structured Api errors and raw reqwest transport errors."

requirements-completed:
  - REL-01
  - TOOL-01
  - TOOL-02
  - TOOL-03
  - TOOL-04

coverage:
  - id: D1
    description: "Public Error::is_transient classification returns true for 429, 5xx, Transient, RateLimited, Timeout and false for 400/404/BadRequest."
    requirement: REL-01
    verification:
      - kind: unit
        ref: "src/retry.rs#is_transient_public_api"
        status: pass
      - kind: unit
        ref: "src/errors.rs#is_transient_detects_transient_variants"
        status: pass
      - kind: unit
        ref: "src/errors.rs#is_transient_rejects_permanent_variants"
        status: pass
    human_judgment: false
  - id: D2
    description: "with_backoff retries a transient operation at least once before succeeding."
    requirement: REL-01
    verification:
      - kind: unit
        ref: "src/retry.rs#with_backoff_retries_transient_errors"
        status: pass
    human_judgment: false
  - id: D3
    description: "with_backoff does not retry a permanent 4xx (non-429) operation."
    requirement: REL-01
    verification:
      - kind: unit
        ref: "src/retry.rs#with_backoff_does_not_retry_permanent_4xx"
        status: pass
    human_judgment: false
  - id: D4
    description: "cargo clippy --all-targets -- -D warnings passes with zero warnings."
    requirement: TOOL-02
    verification:
      - kind: other
        ref: "cargo clippy --all-targets -- -D warnings"
        status: pass
    human_judgment: false
  - id: D5
    description: "cargo doc --no-deps builds with no warnings."
    requirement: TOOL-03
    verification:
      - kind: other
        ref: "cargo doc --no-deps"
        status: pass
    human_judgment: false
  - id: D6
    description: "cargo test --all-targets passes without live cookies (ignored tests remain skipped)."
    requirement: TOOL-01
    verification:
      - kind: other
        ref: "cargo test --all-targets --quiet"
        status: pass
    human_judgment: false
  - id: D7
    description: "cargo publish --dry-run succeeds and excludes planning/tooling files."
    requirement: TOOL-04
    verification:
      - kind: other
        ref: "cargo publish --dry-run --allow-dirty"
        status: pass
    human_judgment: false

# Metrics
duration: 12min
completed: 2026-08-09
status: complete
---

# Phase 1 Plan 4: Reliability verification and tooling/publish gates Summary

**Locked retry/backoff behavior, fixed clippy/doc gates, and made the crate publishable with a reviewed manifest.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-08-09T13:51:51Z
- **Completed:** 2026-08-09T14:04:23Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- Added unit tests for `Error::is_transient` covering 429, 5xx, and permanent 4xx statuses.
- Extended `Error::is_transient` to detect transient HTTP status codes inside `reqwest::Error` so transport-level failures are retried consistently.
- Documented backoff constants (500 ms initial, 8 s max, 30 s elapsed) in `src/retry.rs`.
- Added unit tests proving `with_backoff` retries transient errors and skips permanent 4xx errors.
- Verified `cargo clippy --all-targets -- -D warnings` is green.
- Verified `cargo doc --no-deps` produces no warnings.
- Verified `cargo test --all-targets` passes without live cookies.
- Verified `cargo publish --dry-run` succeeds.
- Added `readme = "README.md"` and a comprehensive `exclude` list to `Cargo.toml`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Verify public transient classification and retry behavior** - `b55a543` (test)
2. **Task 2: Fix clippy warnings across all source files** - `b55a543` (included in Task 1 commit; no separate clippy changes were required)
3. **Task 3: Finalize docs, examples, and publishability** - `22abd96` (docs)

**Plan metadata:** *(pending final docs commit after this file)*

## Files Created/Modified

- `src/retry.rs` - Documented backoff parameters and added retry unit tests.
- `src/errors.rs` - Extended `is_transient` to inspect `reqwest::Error::status()`.
- `Cargo.toml` - Added `readme` field and `exclude` array for publishability; added `http` dev-dependency.
- `.planning/intel/API-SURFACE.md` - Generated API surface placeholder.

## Decisions Made

- Extended `Error::is_transient` to inspect `reqwest::Error::status()` so transport-level 429/5xx errors are retried, aligning with existing `Api` variant behavior.
- Added `http` as a dev-dependency only; no runtime dependency increase.
- Used `Cargo.toml` `exclude` to prevent packaging `.planning`, `.opencode`, `docs`, `benches`, `tests`, `examples`, and tooling config files.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Error::is_transient ignored transient HTTP status codes inside reqwest::Error**
- **Found during:** Task 1
- **Issue:** `with_backoff` wraps raw `reqwest::Error` in `Error::Request`, but `is_transient` only matched structured `Error::Api` status codes. Transport-level 429/5xx `reqwest` errors were classified as permanent and not retried.
- **Fix:** Extended `is_transient` to check `reqwest::Error::status()` for server errors and 429.
- **Files modified:** `src/errors.rs`
- **Verification:** New unit tests pass; `cargo test --lib retry` and `cargo test --lib errors` pass.
- **Committed in:** `b55a543` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** The fix is required for correct retry behavior on transport-level rate limits and server errors. No scope creep.

## Issues Encountered

- Constructing `reqwest::Error` with a specific HTTP status in tests is not directly supported; resolved by building an `http::Response`, converting to `reqwest::Response`, and using `error_for_status()`. This required adding `http` as a dev-dependency.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 1 is complete. All four plans are executed and committed.
- The crate is now publishable (dry-run succeeds) and all tooling gates are green.
- Ready to move to Phase 2: Reliability & Protocol Hardening.

## Self-Check: PASSED

- [x] `src/retry.rs` modified and tests pass.
- [x] `src/errors.rs` modified and tests pass.
- [x] `Cargo.toml` modified and publish dry-run succeeds.
- [x] Commits `b55a543` and `22abd96` exist.

---
*Phase: 01-stabilize-v0-1-core*
*Completed: 2026-08-09*
