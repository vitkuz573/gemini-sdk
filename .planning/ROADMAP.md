# Roadmap: Gemini SDK

**Project:** Gemini SDK
**Created:** 2026-08-08
**Goal:** Stable, documented, semver-respecting Rust SDK for the Gemini web frontend, published on crates.io.

## Phase 1: Stabilize v0.1 Core

**Goal:** Lock the public API, fix auth ergonomics, and make the SDK publishable as v0.1.

**Requirements:** API-01, API-02, API-03, API-04, AUTH-01, AUTH-02, AUTH-03, CHAT-01, CHAT-03, CHAT-05, MEDIA-01, REL-01, TOOL-01, TOOL-02, TOOL-03, TOOL-04

**Key outcomes:**

- Public API marked and documented for forward compatibility.
- `CredentialsProvider` trait replaces raw cookie strings where appropriate.
- Error types consolidated and semver-friendly.
- Text chat, streaming, multi-turn, inline images work through stable types.
- CI-quality tests, clippy, docs, and examples.

**Mode:** standard
**Depends on:** —
**Estimated waves:** 3

**Plans:**

4/4 plans complete

4/4 plans complete

3/4 plans executed

2/4 plans executed

1/4 plans executed

- [x] 01-01-PLAN.md — API surface stabilization and forward compatibility.
- [x] 01-02-PLAN.md — Auth ergonomics: CredentialsProvider trait and redaction.
- [x] 01-03-PLAN.md — Chat + media tests and multi-turn example.
- [x] 01-04-PLAN.md — Reliability verification and tooling/publish gates.

---

## Phase 2: Reliability & Protocol Hardening

**Goal:** Eliminate known fragility from CONCERNS.md and make the SDK resilient to Google's protocol drift.

**Requirements:** AUTH-04, CHAT-02, CHAT-04, PROTO-01, PROTO-02, PROTO-04, REL-02, REL-03

**Key outcomes:**

- Cookie merge persists back into client state.
- Blocking locks removed from synchronous builder methods.
- WAA / ogads failures surface typed errors.
- WIZ slot indices centralized and parser tests expanded.
- Generation config and system instructions exposed.

**Mode:** standard
**Depends on:** Phase 1
**Estimated waves:** 3

---

## Phase 3: Observability & Configurability

**Goal:** Let production users observe, meter, and tune the SDK without forking it.

**Requirements:** PROTO-03, REL-04, OBS-01, OBS-02, MEDIA-02

**Key outcomes:**

- Request/response hooks API.
- `tracing` spans across auth, request, parse, upload.
- Injectable `reqwest::Client` for connection pool control.
- Upload progress callbacks.
- Robust HTML extraction with multiple fallbacks.

**Mode:** standard
**Depends on:** Phase 2
**Estimated waves:** 2

---

## Phase 4: Advanced Media & Sessions

**Goal:** Support richer media types and persistent sessions.

**Requirements:** MEDIA-03, ADV-02

**Key outcomes:**

- Audio and video upload paths.
- Session save/restore helpers for conversation and auth state.

**Mode:** standard
**Depends on:** Phase 3
**Estimated waves:** 2

---

## Phase 5: Tools & Auto-Refresh

**Goal:** Add function calling and reduce manual auth maintenance.

**Requirements:** ADV-01, ADV-03, OBS-03

**Key outcomes:**

- Tools / function calling round-trip.
- Auto cookie refresh / consent re-acquisition.
- Metrics facade for requests, retries, parse failures, attestation.

**Mode:** standard
**Depends on:** Phase 4
**Estimated waves:** 3

---

## Phase 6: v1.0 Release

**Goal:** Polish documentation, verify semver, and publish v1.0.

**Requirements:** TOOL-05

**Key outcomes:**

- Final API audit and deprecation cleanup.
- MSRV policy documented and verified.
- crates.io publication with changelog and release notes.
- Migration guide from v0.x to v1.0.

**Mode:** standard
**Depends on:** Phase 5
**Estimated waves:** 2

---

## Phase 7: Conversation Actions

**Goal:** Expose conversation turn actions (regenerate, rate, delete) over RPC `PCck7e` as typed, tested public APIs.

**Requirements:** CONVACT-01, CONVACT-02, CONVACT-03, CONVACT-04

**Key outcomes:**

- New public methods on `GeminiClient`: `regenerate_turn`, `rate_turn`, `delete_turn`.
- All three actions return a typed `ConversationActionResult` with success/failure status.
- Each action invokes RPC `PCck7e` via the existing `batchexecute_rpc` helper.
- Mocked fixture tests cover request payload shape and response parsing.

**Mode:** standard
**Depends on:** Phase 6
**Estimated waves:** 2

**Plans:** TBD

---

## Phase 8: User Profile & Preferences

**Goal:** Expose user info (`o30O0e`) and last-selected mode (`L5adhe`) as typed, tested public APIs.

**Requirements:** USER-01, USER-02, PREFS-01, PREFS-02, PREFS-03

**Key outcomes:**

- New public methods on `GeminiClient`: `get_user_info`, `get_last_selected_mode`, `set_last_selected_mode`.
- User profile fields tolerate missing or null payload entries.
- Preference payloads follow the exact shape captured in spike 009.

**Mode:** standard
**Depends on:** Phase 7
**Estimated waves:** 2

**Plans:** 1/1 plans complete

- [ ] 08-PLAN.md

---

## Phase 9: Locale & Model Config

**Goal:** Expose locale and model configuration RPCs (`cYRIkd`, `whPPme`, `Te6DCf`, `ku4Jyf`) as thin typed facades.

**Requirements:** LOCALE-01, LOCALE-02, LOCALE-03, LOCALE-04, LOCALE-05

**Key outcomes:**

- New public methods on `GeminiClient`: `get_locale_tools`, `get_model_config`, `get_locale_config`, `get_tools_config`.
- All four responses returned as `serde_json::Value` wrappers to tolerate undocumented shape drift.

**Mode:** standard
**Depends on:** Phase 8
**Estimated waves:** 1

**Plans:**

- [x] 09-01-PLAN.md

1/1 plans complete

---

## Phase 10: Settings Pages

**Goal:** Expose usage-stats (`jSf9Qc`) and scheduled-prompts (`XPSWpd`) RPCs as typed public APIs.

**Requirements:** SETTINGS-01, SETTINGS-02, SETTINGS-03

**Key outcomes:**

- New public methods on `GeminiClient`: `get_usage_stats`, `get_scheduled_prompts`.
- Typed wrappers over `serde_json::Value` with structured accessors.

**Mode:** standard
**Depends on:** Phase 9
**Estimated waves:** 2

**Plans:** TBD

---

## Phase 11: Protocol Drift & Integration

**Goal:** Update the drifted `x-client-data` constant, add usage examples for the new RPCs, and run the final quality gate.

**Requirements:** DRIFT-01, TOOL-06, TOOL-07

**Key outcomes:**

- Default `x-client-data` constant updated from `CI7yygE=` to `CNeOywE=` to match the latest HAR capture.
- At least one runnable example binary demonstrating the new APIs.
- Final clippy / test / doc gates pass; all new RPCs have mocked fixture tests.

**Mode:** standard
**Depends on:** Phase 10
**Estimated waves:** 1

**Plans:** TBD

---

## Milestones

| Milestone | Phases | Target | Definition of Done |
|-----------|--------|--------|--------------------|
| v0.1 Core | 1-6 | Shipped 2026-08-10 | `cargo test`, `cargo clippy`, `cargo doc` pass; API stable enough for external users; crate published. |
| v0.2 API Expansion | 7-11 | In progress | All 9 undocumented `batchexecute` RPCs exposed as typed public APIs; `x-client-data` drift fixed; quality gates green. |
| v1.0 | TBD | TBD | Semver-stable API published. |

---

*Last updated: 2026-08-10 after v0.2 roadmap created*
