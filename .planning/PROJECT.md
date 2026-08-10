# Gemini SDK

## What This Is

A production-ready Rust SDK for the Google Gemini / Bard web frontend (`gemini.google.com`). It exposes an ergonomic async API for text and image chat, streaming responses, multi-turn conversations, model listing, and file uploads, with optional browser attestation for advanced use cases.

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

- ✓ Cookie-based authentication using `__Secure-1PSID` and `__Secure-1PSIDCC` — existing
- ✓ Text-only chat completions — existing
- ✓ Inline image data uploads — existing
- ✓ Streaming and non-streaming response handling — existing
- ✓ Multi-turn `Conversation` state — existing
- ✓ Model listing via `batchexecute` (`GetUserStatus` / `Fd0Qje`) — existing
- ✓ Retry logic with exponential backoff and rate-limit handling — existing
- ✓ Strongly-typed error enum with transient detection — existing
- ✓ Optional browser attestation via headless Chrome CDP — existing

### Active

- [ ] Stabilize public API surface for v0.1 (client, builder, chat types, errors)
- [ ] Refactor internal client responsibilities into testable components
- [ ] Fix known fragile areas: cookie merge, blocking locks, WAA fallback surfacing
- [ ] Improve auth ergonomics with a `CredentialsProvider` trait
- [ ] Add system instructions and generation config support
- [ ] Add upload progress callbacks
- [ ] Add session persistence helpers
- [ ] Improve protocol resilience against WIZ slot / HTML shape changes
- [ ] Add request/response hooks for observability
- [ ] Add tools / function calling support
- [ ] Support audio and video uploads
- [ ] Add metrics and structured tracing integration
- [ ] Auto cookie refresh / consent acquisition
- [ ] Publish v0.1, v0.2, and v1.0 milestones to crates.io

### Out of Scope

- Official Google REST / Vertex AI SDK replacement — this project intentionally targets the undocumented web frontend protocol.
- Real-time voice / video calls — requires a different transport model; defer unless explicitly requested.
- Mobile platforms (iOS/Android bindings) — out of scope for a Rust crate; bindings could be a separate project.
- Paid API abstraction or quota management — Google owns billing; SDK only wraps web frontend access.

## Context

The project started as a reverse-engineering exercise and now has working implementations for chat, upload, auth, session management, and browser attestation. The codebase is organized as a typical Rust crate with `src/`, `tests/`, `examples/`, `benches/`, and `docs/`. Spikes in `.planning/spikes/` document the protocol discovery process.

Key external dependencies: `reqwest`, `tokio`, `serde`, `thiserror`, `tracing`, `tokio-tungstenite` (optional attestation).

Google can change the web frontend protocol without notice, so the SDK must fail loudly and recover gracefully when response shapes drift.

## Constraints

- **Tech stack**: Rust 1.80+, Tokio, reqwest — fixed by project foundation.
- **Protocol**: Undocumented WIZ web frontend — breakage risk is external and unavoidable.
- **Compatibility**: semver must be respected after v0.1; breaking changes acceptable only in 0.x pre-releases.
- **Security**: Cookies are secrets; SDK must redact them in logs and avoid leaking them in errors.
- **Testing**: Live-cookie integration tests are marked `#[ignore]`; CI relies on fixtures and mocked fixtures.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Target web frontend protocol instead of official API | Provides access to features not yet exposed in official SDKs | — Pending validation |
| Use feature-gated browser attestation | Keeps core SDK lightweight; Chrome CDP is a heavy optional dependency | — Pending |
| semver progression 0.1 → 0.2 → 1.0 | Stabilize core API first, add advanced features without breaking changes | — Pending |
| Cookie-based auth as default | Matches current reverse-engineered flow | — Pending |

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

## Current Milestone: v0.2 API Expansion

**Goal:** Expose the remaining undocumented Gemini web-frontend `batchexecute` RPCs documented in spike 009 as typed, tested public APIs, so the SDK covers the complete user-facing surface beyond core chat.

**Target features:**
- Conversation actions (regenerate, rate, delete turn) via `PCck7e`
- User profile retrieval via `o30O0e`
- Last-selected mode / user preferences via `L5adhe`
- Locale and model configuration RPCs: `cYRIkd`, `whPPme`, `Te6DCf`, `ku4Jyf`
- Settings-page data: usage stats (`jSf9Qc`) and scheduled prompts (`XPSWpd`)
- Fix known protocol drift: update `x-client-data` constant to match latest HAR

**Key constraints:**
- Telemetry/reporting RPCs and `signaler-pa`/`myactivity.google.com` remain out of scope.
- Each RPC is a thin typed facade over the existing `batchexecute_rpc` generic helper.
- Backward-compatible additions only; public API changes must not break v0.1 consumers.

---
*Last updated: 2026-08-10 — milestone v0.2 initialized*
