# Milestones

## v0.3 Magic String Elimination (Shipped: 2026-08-11)

**Phases completed:** 4 phases (13-16), 4 plans
**Git range:** `cc8e7c7..39cbd1d`
**Closeout type:** verified_closeout
**Requirements:** 19/19 MAINT requirements validated (see [archive](milestones/v0.3-REQUIREMENTS.md))

**Key accomplishments:**

- Created a single `src/constants.rs` source of truth for protocol literals: URL paths, batchexecute query/transport markers, WIZ/session keys, RPC identifiers, model/category strings, chat roles, MIME types, upload headers, static headers, HAR/redaction values, transient WIZ markers, tracing/metric names, CDP attestation strings, and tool schema keys.
- Refactored `src/client.rs`, `src/session.rs`, `src/proto/`, `src/upload.rs`, `src/har.rs`, `src/transient_400.rs`, `src/metrics.rs`, `src/attestation.rs`, `src/tool.rs`, `src/models.rs`, and `src/chat.rs` to consume the new constants without changing public API signatures.
- Promoted a minimal public subset of constants so examples and integration tests stay DRY, added `tests/common/mod.rs`, and refactored all tests/examples to reuse centralized constants.
- Added a `#[cfg(test)]` regression gate in `src/constants.rs` that walks `src/` and fails if high-risk denied literals reappear in production code.
- Kept all quality gates green: `cargo test --all-targets` (164 lib + 31 integration + 32 doctests), `cargo clippy --all-targets -- -D warnings`, and `cargo doc --no-deps`.

---

## v0.2 API Expansion (Shipped: 2026-08-11)

**Phases completed:** 6 phases (7-12), 6 plans, 18 tasks
**Git range:** `704fc92..d1b734a`
**Closeout type:** verified_closeout

**Key accomplishments:**

- Exposed conversation turn actions (`regenerate_turn`, `rate_turn`, `delete_turn`) on `GeminiClient` via RPC `PCck7e`, backed by Wiremock fixtures and a configurable `base_url`.
- Added typed user profile (`o30O0e`) and last-selected mode preference (`L5adhe`) APIs with null-tolerant parsers and fixture tests.
- Wrapped four undocumented locale/model configuration RPCs (`cYRIkd`, `whPPme`, `Te6DCf`, `ku4Jyf`) in thin `serde_json::Value` facades to tolerate protocol drift.
- Added settings-page APIs for usage stats (`jSf9Qc`) and scheduled prompts (`XPSWpd`) using the same Value-wrapper pattern.
- Updated the drifted `x-client-data` constant to `CNeOywE=` and shipped a runnable `examples/v0_2_api_tour.rs` plus full fixture coverage for all nine new RPCs.
- Hardened live backend resilience with `Error::NotSignedIn` detection, conservative transient WIZ 400 retries, redacted HAR capture, a `live_probe` telemetry binary, and expanded real-cookie integration tests passing 14/14.

---

## v0.4 StreamGenerate Slot Hardening (In Progress)

**Goal:** Eliminate every raw numeric slot index in the 97-slot `StreamGenerate` request builder by introducing semantically named, HAR-backed constants.

**Key target features:**

- Rename misleading legacy constants to match HAR-observed semantics.
- Add named constants for all remaining raw indices used in `src/proto/slots.rs`.
- Refactor builder and fallback base to use only named constants.
- Add a regression gate that fails if raw `inner[\d+]` assignments reappear in production builder code.
- Keep all quality gates green.

---

## v0.1 Core (Shipped: 2026-08-10)

**Phases completed:** 6 phases (1-6), 19 plans, 12 tasks

**Key accomplishments:**

- Locked the public API surface with `#[non_exhaustive]` types, deny-level doc lints, compile-time `Error` trait checks, and a documented semver policy.
- Introduced a pluggable `CredentialsProvider` trait and fully redacted credential `Debug` output without adding runtime dependencies.
- Shipped fixture-driven tests for text chat, multi-turn state, model category slots, and inline image encoding, plus a multi-turn example binary.
- Locked retry/backoff behavior, fixed clippy/doc gates, and made the crate publishable with a reviewed manifest.
- Added tools/function calling round-trip, auto cookie refresh, session persistence, audio/video upload support, request/response hooks, `tracing` integration, and metrics facade.

---
