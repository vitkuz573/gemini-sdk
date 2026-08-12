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
- ✓ Eliminate raw numeric slot indices in `src/proto/slots.rs` and centralize HAR-backed constants — v0.4
- ✓ Add regression gate preventing raw `inner[\d+]` assignments in StreamGenerate builder — v0.4
- ✓ SAPISIDHASH + `x-goog-authuser: 0` auth plumbing for settings-page RPCs — v0.5
- ✓ Typed `UsageStats` parser with array-shaped response support — v0.5
- ✓ Root-page `SNlM0e` async token fallback — v0.5
- ✓ Fixture and live-cookie tests for usage stats auth and parsing — v0.5

### Active

- [ ] Reverse-engineer the BotGuard WAA challenge → slot-3 token transform for browserless attestation (v0.5)
- [ ] Implement a Rust WAA token generator behind an optional feature/module (v0.5)
- [ ] Integrate browserless WAA into session warm-up so image uploads work without headless Chrome (v0.5)
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

- **Tech stack**: Rust 1.80+, Tokio, reqwest — fixed by project foundation. Browserless WAA may add heavy dependencies (V8, Chromium, or QuickJS) behind optional features when required for correctness.
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

**Shipped:** v0.5 Usage Stats Reliability (2026-08-12)
**Phases completed:** 20 of 20 (100%)
**Current focus:** Reverse-engineer and implement a browserless WAA token generator so image uploads and multi-turn state work without launching headless Chrome.

Spike 004 established that the WAA token in `StreamGenerate` slot 3 is produced by Google's BotGuard VM. The VM takes the WAA `Create` challenge token as input and produces a short `!`-prefixed base64url attestation token. The current SDK optionally obtains this via headless Chrome CDP, which is operationally brittle. The captured HAR at `/home/vitaly/mitm.har` and spike artifacts (`pairs.json`, `botguard.js`, slot dumps) contain at least one `(challenge, slot-3)` pair and the BotGuard VM source. v0.5 will determine how to generate valid slot-3 tokens without requiring the SDK user to manage a Chrome instance. The chosen approach prioritizes correctness and maintainability over keeping the dependency surface minimal: BotGuard is a real browser VM, and pretending otherwise produces fragile partial emulations.

## Current Milestone: v0.5 Browserless WAA Reverse

**Goal:** Reverse-engineer and implement a browserless WAA (Web Application Authentication / BotGuard) token generator for `StreamGenerate` slot 3, so the SDK can obtain valid attestation context without requiring the user to launch headless Chrome.

**Target features:**
- Re-analyze `/home/vitaly/mitm.har` and existing spike 004 artifacts (`pairs.json`, `botguard.js`, slot dumps) for deterministic WAA challenge → slot-3 relationships.
- Capture or synthesize additional `(Waa/Create challenge, slot-3 token)` pairs to constrain the BotGuard VM behavior.
- Build a robust VM harness that reproduces the observed slot-3 token from a WAA challenge. Prefer a real browser engine (headless Chromium via CDP/Playwright/Puppeteer, or V8/QuickJS with faithful DOM mocks) over brittle pure-Rust approximations.
- Integrate the browserless WAA path into `GeminiClient` session warm-up so it can replace or bypass the manual CDP-based attestation capture when enabled.
- Add fixture tests, unit tests, and a live-cookie integration test that verify image upload works without the `browser-attestation` feature.
- Keep all quality gates green (`cargo test`, `cargo clippy`, `cargo doc`).

## Previous Milestone: v0.5 Usage Stats Reliability

**Goal:** Fix `GeminiClient::get_usage_stats` so it returns real usage statistics instead of an empty object, matching the live Gemini frontend request shape and auth requirements.

**Target features:**
- Correct the auth header / cookie mismatch that causes `jSf9Qc` to return an empty payload (SAPISIDHASH + `x-goog-authuser` parity with browser).
- Reconcile the `jSf9Qc` inner payload against the live HAR at `/home/vitaly/mitm.har` and the documented response shape `[2,[[999999,0,5,...]],false]`.
- Return a typed `UsageStats` value with documented accessors instead of an opaque empty object, preserving a fallback `Value` escape hatch.
- Add fixture and live-cookie tests that verify non-empty stats parsing and auth header correctness.
- Update the companion CLI (`gemini-cli`) contract so its `usage` subcommand surfaces real counts.
- Keep all quality gates green (`cargo test`, `cargo clippy`, `cargo doc`).

## Next Milestone: v1.0 Stable Release

**Goal:** Polish documentation, verify semver, and publish v1.0 to crates.io.

**Target features:**
- Final API audit and deprecation cleanup
- MSRV policy documented and verified
- crates.io publication with changelog and release notes
- Migration guide from v0.x to v1.0
- OAuth / refresh-token flow as an alternative to cookie strings (post-v1.0)
- Resumable upload with explicit chunk size control (post-v1.0)

---
*Last updated: 2026-08-12 — started v0.5 Browserless WAA Reverse after v0.5 Usage Stats Reliability*
