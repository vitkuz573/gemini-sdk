---
phase: 01-stabilize-v0-1-core
plan: 01
subsystem: api
tags: [rust, semver, non_exhaustive, static_assertions, docs]

requires: []
provides:
  - Forward-compatible public types with #[non_exhaustive]
  - Verified Error trait bounds (Send + Sync + std::error::Error + 'static)
  - Deny-level missing_docs and broken_intra_doc_links lints
  - Documented semver policy in README.md
  - API stability integration tests
affects:
  - 01-stabilize-v0-1-core
  - 02-reliability-protocol

tech-stack:
  added:
    - static_assertions (dev-dependency)
  patterns:
    - #[non_exhaustive] on all extensible public structs/enums
    - pub(crate) fields with public accessor methods
    - Compile-time trait bound assertions in test modules
    - deny(missing_docs) + deny(rustdoc::broken_intra_doc_links)

key-files:
  created:
    - tests/api_stability.rs — runtime checks that public types cannot be built via struct literals
  modified:
    - src/lib.rs — upgraded lints from warn to deny
    - src/auth.rs — fixed broken intra-doc link to GeminiClient::verify_signed_in
    - src/errors.rs — added trait bound tests and is_transient unit tests
    - src/chat.rs — #[non_exhaustive] on ChatMessage, ChatResponse, Conversation
    - src/client.rs — #[non_exhaustive] on GeminiClient and ChatBuilder
    - src/models.rs — #[non_exhaustive] on ModelInfo + accessor methods for previously public fields
    - Cargo.toml — added static_assertions dev-dependency
    - README.md — added Semver Policy section

key-decisions:
  - "Kept runtime API stability tests instead of trybuild because the existing test suite and cargo test already enforce the contract; this avoids a heavy new dev-dependency."
  - "Privatized ChatResponse and ModelInfo fields and added accessor methods rather than only relying on #[non_exhaustive], so downstream code cannot depend on struct layout even inside the crate boundary."
  - "Used a helper function assert_static::<Error>() to verify 'static bound because static_assertions::assert_impl_all! does not accept lifetime bounds directly."

requirements-completed: [API-01, API-02, API-03, API-04]

coverage:
  - id: D1
    description: Public extensible types carry #[non_exhaustive] and block external literal construction
    requirement: API-01
    verification:
      - kind: integration
        ref: tests/api_stability.rs
        status: pass
    human_judgment: false
  - id: D2
    description: Error type is Send + Sync + std::error::Error + 'static
    requirement: API-02
    verification:
      - kind: unit
        ref: src/errors.rs#error_is_send_sync_static
        status: pass
    human_judgment: false
  - id: D3
    description: is_transient correctly classifies transient vs permanent error variants
    requirement: API-02
    verification:
      - kind: unit
        ref: src/errors.rs#is_transient_detects_transient_variants, src/errors.rs#is_transient_rejects_permanent_variants
        status: pass
    human_judgment: false
  - id: D4
    description: Documentation builds with no warnings and missing_docs is denied
    requirement: API-03
    verification:
      - kind: other
        ref: cargo doc --no-deps (0 warnings)
        status: pass
    human_judgment: false
  - id: D5
    description: README.md explains semver policy for 0.x and post-1.0 releases
    requirement: API-04
    verification:
      - kind: other
        ref: README.md Semver policy section
        status: pass
    human_judgment: true
    rationale: Policy text requires human review for clarity and accuracy; automated checks can verify presence but not correctness of prose.

duration: 13m 52s
completed: 2026-08-09
status: complete
---

# Phase 01 Plan 01: Stabilize v0.1 Core — API Surface Summary

**Public API surface locked with #[non_exhaustive] types, deny-level doc lints, compile-time Error trait checks, and a documented semver policy.**

## Performance

- **Duration:** 13m 52s
- **Started:** 2026-08-09T13:04:51Z
- **Completed:** 2026-08-09T13:18:43Z
- **Tasks:** 3
- **Files modified:** 9

## Accomplishments

- Applied `#[non_exhaustive]` to all extensible public structs and enums (`ChatMessage`, `ChatResponse`, `Conversation`, `GeminiClient`, `ChatBuilder`, `ModelInfo`, plus existing enums).
- Privatized fields of `ChatResponse` and `ModelInfo` and added stable public accessor methods so downstream code cannot depend on struct layout.
- Created `tests/api_stability.rs` integration tests verifying public types cannot be constructed via struct literals.
- Upgraded `src/lib.rs` lints to `#![deny(missing_docs)]` and `#![deny(rustdoc::broken_intra_doc_links)]`.
- Fixed the broken intra-doc link in `src/auth.rs` by fully qualifying `crate::client::GeminiClient::verify_signed_in`.
- Added `static_assertions` compile-time checks for `Error: Send + Sync + std::error::Error` and a helper-based `'static` check.
- Added unit tests for `Error::is_transient` covering transient and permanent variants.
- Added a Semver Policy section to `README.md` explaining breaking-change rules before and after v1.0.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add #[non_exhaustive] and API stability tests** — `ef24f51` (test), `b1c0bd4` (feat)
2. **Task 2: Verify Error trait bounds and document semver policy** — `117b59b` (feat), `a4273d9` (docs)
3. **Task 3: Tighten documentation lints and fix broken intra-doc link** — `bdab927` (feat)

**Plan metadata:** `3493561` (docs: complete plan)

_Note: Task 1 followed TDD discipline — test commit precedes implementation commit._

## Files Created/Modified

- `src/lib.rs` — changed `warn(missing_docs)` to `deny(missing_docs)` and added `deny(rustdoc::broken_intra_doc_links)`.
- `src/auth.rs` — fixed fully-qualified intra-doc link to `crate::client::GeminiClient::verify_signed_in`.
- `src/errors.rs` — added `#[cfg(test)]` assertions for `Send + Sync + std::error::Error + 'static` and `is_transient` tests.
- `src/chat.rs` — added `#[non_exhaustive]` to `ChatMessage`, `ChatResponse`, `Conversation`; made `ChatResponse` fields `pub(crate)`.
- `src/client.rs` — added `#[non_exhaustive]` to `GeminiClient` and `ChatBuilder`.
- `src/models.rs` — added `#[non_exhaustive]` to `ModelInfo`, made fields `pub(crate)`, added public accessors.
- `tests/api_stability.rs` — new integration tests for API stability.
- `tests/proto_tests.rs`, `tests/real_cookies.rs`, `tests/integration_tests.rs` — updated to use `ModelInfo` accessor methods.
- `Cargo.toml` — added `static_assertions` dev-dependency.
- `README.md` — added Semver Policy section.

## Decisions Made

- Followed the plan's instruction to prefer runtime tests over `trybuild` compile-fail tests; this keeps the dev-dependency footprint minimal while still enforcing the public-surface contract.
- Chose to add public accessor methods for `ModelInfo` rather than keeping fields public, strengthening forward compatibility beyond the `#[non_exhaustive]` marker alone.
- Verified `'static` via a helper function because `static_assertions::assert_impl_all!` does not accept lifetime bounds as trait arguments.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Privatizing `ModelInfo` fields broke existing tests and the ignored live-cookie tests that accessed `.id`, `.title`, and `.category_enum` directly. Updated all call sites to use the new accessor methods.
- The initial attempt to assert `'static` with `assert_impl_all!(Error: 'static)` failed because the macro expects trait paths; replaced with a compile-time helper function `assert_static::<Error>()`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Public API surface is now stable and forward-compatible.
- All lints, tests, docs, and clippy pass.
- Ready for Plan 01-02 (cookie/auth redaction hardening).

## Self-Check: PASSED

- [x] Created files exist: `tests/api_stability.rs`, `01-01-SUMMARY.md`
- [x] Commits exist: `ef24f51`, `b1c0bd4`, `bdab927`, `117b59b`, `a4273d9`
- [x] Final metadata commit verified: `3493561`
- [x] All verification commands pass: `cargo test --lib`, `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps` (0 warnings), `cargo build --examples`

---
*Phase: 01-stabilize-v0-1-core*
*Completed: 2026-08-09*
