# Gemini SDK

## What This Is

A production-ready Rust SDK for the Google Gemini / Bard web frontend (`gemini.google.com`). It exposes an ergonomic async API for text and image chat, streaming responses, multi-turn conversations, model listing, file uploads, function calling, user profile and preference access, locale/model configuration, settings-page data, and optional browser attestation for advanced use cases.

The SDK targets Rust developers who want a typed, tested, crate-published client for the undocumented Gemini web protocol without managing cookie headers and WIZ slot payloads by hand.

## Core Value

Developers can reliably integrate Gemini into Rust applications using a stable, documented, semver-respecting SDK that handles auth, protocol quirks, retries, and common content types out of the box.

## Business Context

<!-- This is an open-source library, not a monetized product. -->

- **Customer**: Rust developers building bots, CLI tools, automation, or experimental integrations with Gemini.
- **Revenue model**: None — open source under MIT.
- **Success metric**: Clean `cargo add gemini-sdk` experience, passing CI, positive API ergonomics feedback, and crates.io downloads.
- **Strategy notes**: Track toward a stable v1.0 API while preserving flexibility for Google's breaking protocol changes.

## Requirements

### Validated

- ✓ Cookie-based authentication using `__Secure-1PSID` and `__Secure-1PSIDCC` — v0.1
- ✓ Text-only chat completions — v0.1
- ✓ Inline image data uploads — v0.1
- ✓ Streaming and non-streaming response handling — v0.1
- ✓ Multi-turn `Conversation` state — v0.1
- ✓ Model listing via `batchexecute` (`GetUserStatus` / `Fd0Qje`) — v0.1
- ✓ Retry logic with exponential backoff and rate-limit handling — v0.1
- ✓ Strongly-typed error enum with transient detection — v0.1
- ✓ Optional browser attestation via headless Chrome CDP — v0.1
- ✓ `CredentialsProvider` trait for pluggable auth sources — v0.1
- ✓ Request/response hooks and `tracing` integration — v0.1
- ✓ Tools / function calling round-trip — v0.1
- ✓ Audio and video upload support — v0.1
- ✓ Session persistence helpers — v0.1
- ✓ Conversation actions (`regenerate_turn`, `rate_turn`, `delete_turn`) — v0.2
- ✓ User profile retrieval (`get_user_info`) — v0.2
- ✓ Last-selected mode preferences (`get_last_selected_mode`, `set_last_selected_mode`) — v0.2
- ✓ Locale and model configuration RPCs — v0.2
- ✓ Usage stats and scheduled prompts — v0.2
- ✓ Transient WIZ 400 retry and `Error::NotSignedIn` detection — v0.2
- ✓ Opt-in redacted HAR capture — v0.2
- ✓ Live probe and real-cookie integration tests — v0.2
- ✓ Centralized protocol, transport, model, MIME, header, HAR, tracing, attestation, and tool-schema constants — v0.3
- ✓ Regression gate preventing reintroduction of eliminated magic strings in `src/` — v0.3
- ✓ Cleaned-up tests and examples reusing centralized constants — v0.3

### Active

- [ ] Final API audit and deprecation cleanup for v1.0
- [ ] Document and verify MSRV policy
- [ ] crates.io publication with changelog and release notes
- [ ] Migration guide from v0.x to v1.0
- [ ] OAuth / refresh-token flow as an alternative to cookie strings (post-v1.0)
- [ ] Resumable upload with explicit chunk size control (post-v1.0)

### Out of Scope

- Official Google REST / Vertex AI SDK replacement — this project intentionally targets the undocumented web frontend protocol.
- Real-time voice / video calls — requires a different transport model; defer unless explicitly requested.
- Mobile platforms (iOS/Android bindings) — out of scope for a Rust crate; bindings could be a separate project.
- Paid API abstraction or quota management — Google owns billing; SDK only wraps web frontend access.
- Telemetry / heartbeat RPCs — library SDK should not emit analytics traffic to Google.

## Context

The project started as a reverse-engineering exercise and now has working implementations for chat, upload, auth, session management, browser attestation, conversation actions, user profile/preferences, locale/model config, settings-page data, and live backend resilience. The codebase is organized as a typical Rust crate with `src/`, `tests/`, `examples/`, `benches/`, and `docs/`. Spikes in `.planning/spikes/` document the protocol discovery process.

Key external dependencies: `reqwest`, `tokio`, `serde`, `thiserror`, `tracing`, `tokio-tungstenite` (optional attestation), `humantime`, `tempfile`, `const_format`.

Google can change the web frontend protocol without notice, so the SDK must fail loudly and recover gracefully when response shapes drift. v0.2 introduced `serde_json::Value` wrappers for undocumented RPC surfaces, conservative transient-400 detection, and redacted HAR capture to aid debugging without leaking secrets. v0.3 consolidated protocol literals into `src/constants.rs` so future drift updates touch a single source of truth and a regression gate keeps high-risk literals from reappearing in production code.

## Constraints

- **Tech stack**: Rust 1.80+, Tokio, reqwest — fixed by project foundation.
- **Protocol**: Undocumented WIZ web frontend — breakage risk is external and unavoidable.
- **Compatibility**: semver must be respected after v0.1; breaking changes acceptable only in 0.x pre-releases.
- **Security**: Cookies are secrets; SDK must redact them in logs and avoid leaking them in errors.
- **Testing**: Live-cookie integration tests are marked `#[ignore]`; CI relies on fixtures and mocked fixtures.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Target web frontend protocol instead of official API | Provides access to features not yet exposed in official SDKs | ✓ Validated — v0.2 ships 9 additional undocumented RPCs |
| Use feature-gated browser attestation | Keeps core SDK lightweight; Chrome CDP is a heavy optional dependency | ✓ Good — optional path remains stable |
| semver progression 0.1 → 0.2 → 1.0 | Stabilize core API first, add advanced features without breaking changes | ✓ Good — v0.1 and v0.2 both backward-compatible additions |
| Cookie-based auth as default | Matches current reverse-engineered flow | ✓ Good — `CredentialsProvider` trait added for extensibility |
| Expose undocumented RPCs as thin typed facades over `batchexecute_rpc` | Avoids new transport code; tolerates protocol drift | ✓ Good — v0.2 RPCs shipped with minimal surface |
| Return `serde_json::Value` wrappers for undocumented config RPCs | Prevents brittle structs when Google changes shapes | ✓ Good — no parser breakages across v0.2 |
| Conservative transient WIZ 400 detection (`er` + `di` + `af.httprm`) | Avoids retrying genuine client errors | ✓ Good — live probe passes 14/14 |
| Opt-in HAR capture with cookie/auth redaction | Aids debugging without leaking credentials | ✓ Good — redaction verified in unit tests |
| Centralize magic strings in `src/constants.rs` with minimal public API exposure | Makes protocol drift safer to update and review | ✓ Good — 19 MAINT requirements validated, regression gate green |
| Keep test fixtures raw while centralizing production constants | Fixtures retain captured protocol shapes; production code uses named constants | ✓ Good — no production inline literals for targeted deny-list |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

## Current State

**Shipped:** v0.3 Magic String Elimination (2026-08-11)
**Phases completed:** 16 of 16 (100%)
**Current focus:** Planning v1.0 Stable Release

v0.3 delivered a single cross-cutting `src/constants.rs` module that centralizes protocol literals across the SDK. All production modules now consume named constants for URL paths, batchexecute query/transport markers, WIZ/session keys, RPC identifiers, model/category strings, chat roles, MIME types, upload headers, static headers, HAR/redaction values, transient WIZ markers, tracing/metric names, CDP attestation strings, and tool schema keys. Tests and examples were refactored to reuse these constants, and a regression gate prevents reintroduction of high-risk magic strings in `src/`. All quality gates (`cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps`) pass.

## Next Milestone: v1.0 Stable Release

**Goal:** Polish documentation, verify semver, and publish v1.0 to crates.io.

**Target features:**
- Final API audit and deprecation cleanup
- MSRV policy documented and verified
- crates.io publication with changelog and release notes
- Migration guide from v0.x to v1.0

---
*Last updated: 2026-08-11 after v0.3 milestone completion*
