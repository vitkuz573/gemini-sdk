---
phase: 01-stabilize-v0-1-core
plan: 02
subsystem: auth
tags: [rust, credentials, redaction, async-trait, provider]

requires:
  - phase: 01-stabilize-v0-1-core
    provides: public API forward-compatibility and strict doc lints
provides:
  - CredentialsProvider trait with boxed-future async method
  - CookieHeaderProvider default implementation
  - GeminiClient::from_provider async constructor
  - Fully-redacted Credentials Debug output
  - Integration tests for provider trait and redaction
affects:
  - 01-stabilize-v0-1-core

tech-stack:
  added: []
  patterns:
    - "Async trait via Pin<Box<dyn Future>> to avoid extra dependencies"
    - "Blanket impl for Credentials so bare values satisfy CredentialsProvider"
    - "Full redaction in Debug: '<redacted>' / '(empty)' instead of prefixes"

key-files:
  created:
    - tests/auth_provider.rs
    - tests/redaction.rs
  modified:
    - src/auth.rs
    - src/client.rs
    - src/lib.rs

key-decisions:
  - "CredentialsProvider uses Pin<Box<dyn Future<...>>> to stay object-safe without async-trait"
  - "CookieHeaderProvider validates the header at construction time for fail-fast behavior"
  - "Debug redaction shows '<redacted>' for any non-empty secret and '(empty)' for empty strings"

patterns-established:
  - "Async auth trait: boxed Send future keeps object safety and avoids new runtime deps"
  - "Provider pattern preserves existing from_cookie_header / from_credentials constructors"

requirements-completed: [AUTH-01, AUTH-02, AUTH-03]

coverage:
  - id: D1
    description: "Credentials Debug output contains no secret substrings"
    requirement: AUTH-03
    verification:
      - kind: integration
        ref: "tests/redaction.rs#debug_contains_no_secret_substrings"
        status: pass
      - kind: unit
        ref: "src/auth.rs#credentials_debug_redacts_secrets"
        status: pass
    human_judgment: false
  - id: D2
    description: "Empty secrets render as '(empty)' and non-empty secrets as '<redacted>'"
    requirement: AUTH-03
    verification:
      - kind: integration
        ref: "tests/redaction.rs#debug_redacts_non_empty_secrets"
        status: pass
      - kind: integration
        ref: "tests/redaction.rs#debug_shows_empty_secrets"
        status: pass
    human_judgment: false
  - id: D3
    description: "CredentialsProvider trait exists and is implementable by downstream code"
    requirement: AUTH-02
    verification:
      - kind: integration
        ref: "tests/auth_provider.rs#custom_provider_yields_credentials"
        status: pass
      - kind: integration
        ref: "tests/auth_provider.rs#bare_credentials_satisfies_provider_trait"
        status: pass
    human_judgment: false
  - id: D4
    description: "CookieHeaderProvider parses a valid header and rejects missing PSIDCC"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "tests/auth_provider.rs#cookie_header_provider_parses_valid_header"
        status: pass
      - kind: integration
        ref: "tests/auth_provider.rs#cookie_header_provider_rejects_missing_psidcc"
        status: pass
    human_judgment: false
  - id: D5
    description: "GeminiClient::from_provider builds a client from a boxed provider"
    requirement: AUTH-02
    verification:
      - kind: integration
        ref: "tests/auth_provider.rs#client_from_provider_builds_from_boxed_provider"
        status: pass
    human_judgment: false
  - id: D6
    description: "No new runtime dependencies added; clippy and doc builds remain clean"
    requirement: AUTH-02
    verification:
      - kind: other
        ref: "cargo check --quiet && cargo clippy --all-targets -- -D warnings && cargo doc --no-deps"
        status: pass
    human_judgment: false

# Metrics
duration: 19 min
completed: 2026-08-09
status: complete
---

# Phase 1 Plan 2: Auth Ergonomics Summary

**Introduced a pluggable `CredentialsProvider` trait and fully redacted credential `Debug` output without adding runtime dependencies.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-08-09T13:33:55Z
- **Completed:** 2026-08-09T13:34:14Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- Replaced partial credential redaction with full `<redacted>` / `(empty)` output for every secret field.
- Added an async `CredentialsProvider` trait using boxed futures to avoid the `async-trait` dependency.
- Provided a `CookieHeaderProvider` default implementation that parses cookie headers on demand.
- Added `GeminiClient::from_provider` so advanced users can source auth from env/file/keyring without changing existing constructors.
- Re-exported provider types at the crate root.
- Added integration tests for provider behavior and redaction guarantees.
- Verified `cargo check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo doc --no-deps` all pass with no warnings.

## Task Commits

1. **Task 1: Implement full credential redaction with integration test (RED)** - `9fc5a38` (test)
2. **Task 1: Implement full credential redaction with integration test (GREEN)** - `9ecc953` (feat)
3. **Task 2: Introduce CredentialsProvider trait and default provider** - `f1182c6` (feat)
4. **Task 3: Wire public exports and verify no unapproved dependencies** - `b844d2c` (fix)

**Plan metadata:** pending final docs commit

## Files Created/Modified

- `src/auth.rs` - Added `CredentialsProvider` trait, `CookieHeaderProvider`, blanket `Credentials` impl, and full `Debug` redaction.
- `src/client.rs` - Added `GeminiClient::from_provider` async constructor.
- `src/lib.rs` - Re-exported `CredentialsProvider` and `CookieHeaderProvider`.
- `tests/redaction.rs` - Integration tests verifying no secret material in `Debug`.
- `tests/auth_provider.rs` - Integration tests for trait, default provider, and client constructor.

## Decisions Made

- Used `Pin<Box<dyn Future<...>>>` for `CredentialsProvider` to keep the trait object-safe and avoid adding the `async-trait` crate (matches RESEARCH.md Pattern 4).
- Validated `CookieHeaderProvider` at construction so downstream code fails fast with a typed error.
- Chose `"<redacted>"` for all non-empty secrets and `"(empty)"` for empty strings, eliminating any prefix or length leakage.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Adjusted redaction test to account for Option wrapping in Debug**
- **Found during:** Task 1
- **Issue:** The initial test expected `psidts: "<redacted>"`, but `Option<String>` renders as `Some("<redacted>")`.
- **Fix:** Updated assertions in `tests/redaction.rs` to accept either the bare or `Some(...)` form.
- **Files modified:** `tests/redaction.rs`
- **Verification:** `cargo test --test redaction --quiet` passes
- **Committed in:** `9fc5a38` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Minor test assertion adjustment; no scope or behavior change.

## Issues Encountered

- `cargo doc` initially failed because `CredentialsProvider` docs referenced an unqualified `GeminiClient::from_cookie_header` link. Fixed by using `crate::GeminiClient::from_cookie_header`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Auth ergonomics and redaction are complete.
- Phase 1 Plan 3 (chat + media tests and multi-turn example) can proceed.

---
*Phase: 01-stabilize-v0-1-core*
*Completed: 2026-08-09*

## Self-Check: PASSED

- [x] Created files exist: `tests/auth_provider.rs`, `tests/redaction.rs`
- [x] Modified files committed: `src/auth.rs`, `src/client.rs`, `src/lib.rs`
- [x] Commits found in git log for `01-02`
- [x] Plan-level verification commands pass
