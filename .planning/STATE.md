---
gsd_state_version: 1.0
milestone: v0.3
milestone_name: Magic String Elimination
current_phase: 13 — Core Protocol Constants
current_plan: —
status: Planning
stopped_at: Milestone v0.3 planning in progress
last_updated: "2026-08-11T12:00:00.000Z"
last_activity: 2026-08-11
last_activity_desc: Started v0.3 milestone planning — magic string elimination
progress:
  total_phases: 16
  completed_phases: 12
  total_plans: 25
  completed_plans: 25
  percent: 75
---

# Project State

**Project:** Gemini SDK
**Initialized:** 2026-08-08
**Current milestone:** v0.3 — Magic String Elimination
**Current phase:** 13 — Core Protocol Constants
**Current Plan:** —
**Total Plans in Phase:** 0/TBD

milestone: v0.3

## Project Reference

See: `.planning/PROJECT.md` (updated 2026-08-11)

**Core value:** Developers can reliably integrate Gemini into Rust applications using a stable, documented, semver-respecting SDK that handles auth, protocol quirks, retries, and common content types out of the box.
**Current focus:** Milestone v0.2 archived. Planning v0.3 Magic String Elimination: centralize URL paths, RPC identifiers, WIZ/session keys, model/category strings, MIME types, HAR/redaction strings, tracing operation names, DevTools/attestation strings, and tool schema keys as named constants.

## Phase Status

| Phase | Status | Plans | Progress |
|-------|--------|-------|----------|
| 1 — Stabilize v0.1 Core | ✓ Complete | 4/4 | 100% |
| 2 — Reliability & Protocol Hardening | ✓ Complete | 3/3 | 100% |
| 3 — Observability & Configurability | ✓ Complete | 5/5 | 100% |
| 4 — Advanced Media & Sessions | ✓ Complete | 2/2 | 100% |
| 5 — Tools & Auto-Refresh | ✓ Complete | 3/3 | 100% |
| 6 — v1.0 Release | ✓ Complete | 2/2 | 100% |
| 7 — Conversation Actions | ✓ Complete | 1/1 | 100% |
| 8 — User Profile & Preferences | ✓ Complete | 1/1 | 100% |
| 9 — Locale & Model Config | ✓ Complete | 1/1 | 100% |
| 10 — Settings Pages | ✓ Complete | 1/1 | 100% |
| 11 — Protocol Drift & Integration | ✓ Complete | 1/1 | 100% |
| 12 — Live Testing & Backend Resilience | ✓ Complete | 1/1 | 100% |
| 13 — Core Protocol Constants | 🚧 Planning | 0/TBD | 0% |
| 14 — Model/Chat/Upload Constants | 🚧 Planning | 0/TBD | 0% |
| 15 — Infrastructure Constants | 🚧 Planning | 0/TBD | 0% |
| 16 — Test & Example Cleanup + Regression Guard | 🚧 Planning | 0/TBD | 0% |

## Active Decisions

- Phase 12: Conservative WIZ transient 400 detection requires all three markers (er, di, af.httprm) on HTTP 400; HAR capture is opt-in and redacts cookies, Authorization, x-goog-ext-* headers, and cookie-like POST substrings.
- Phase 11: Updated `X_CLIENT_DATA` constant to `CNeOywE=` to match the latest HAR capture and added a read-only v0.2 API tour example with configurable `GEMINI_BASE_URL`.
- Phase 10: Reused the `locale_model_config.rs` Value-wrapper pattern for settings-page RPCs to keep v0.2 surfaces consistent.
- v0.3: Magic strings must be centralized as named constants without changing public API behavior or names. Existing `pub(crate) const` RPC IDs are a pattern to extend, not replace.
- semver progression: 0.1 → 0.2 → 0.3 → 1.0
- Cookie-based auth remains default; provider trait added for extensibility (async `CredentialsProvider` with boxed futures, no async-trait dependency).
- Web frontend protocol remains the target; official REST/Vertex AI out of scope.
- Telemetry / reporting RPCs and `signaler-pa` / `myactivity.google.com` endpoints remain out of scope (no library SDK should emit analytics traffic).

## Open Risks

- Google may change the undocumented WIZ protocol without notice.
- Browser attestation depends on Chrome CDP and live frontend selectors.
- Live-cookie integration tests cannot run in CI.
- Mass string centralization can introduce regressions if constants are renamed inconsistently; plans include regression gates.

## Context

Codebase map available in `.planning/codebase/`.
Spike findings skill available at `.opencode/skills/spike-findings-gemini-sdk/SKILL.md`.
v0.2 RPC coverage derived from spike 001 (HAR API coverage).

---
*Last updated: 2026-08-11 — started v0.3 milestone planning*

## Performance Metrics

| Phase | Plan | Duration | Notes |
|-------|------|----------|-------|
| Phase 12-live-testing-resilience P01 | 45 min | 5 tasks | 11 files |
| Phase 11-protocol-drift-integration P01 | 20 min | 3 tasks | 5 files |
| Phase 09-locale-model-config P01 | 11 min | 4 tasks | 8 files |
| Phase 08-user-profile-preferences P01 | 6 min | 3 tasks | 8 files |

## Decisions

- [v0.3 planning]: Introduce a dedicated `src/constants.rs` (or module family) for cross-cutting strings and keep RPC-specific constants co-located in their feature modules.
- [v0.3 planning]: Avoid public API changes; constants remain `pub(crate)` unless they were already public.
- [Phase ?]: Used runtime API stability tests instead of trybuild to keep dev-dependency footprint minimal.
- [Phase ?]: Privatized ChatResponse and ModelInfo fields and added accessors to strengthen forward compatibility beyond #[non_exhaustive].
- [Phase ?]: CredentialsProvider uses Pin<Box<dyn Future>> to avoid async-trait dependency — Keeps trait object-safe and dependency surface minimal for v0.1 per RESEARCH.md Pattern 4
- [Phase ?]: Credentials Debug shows '<redacted>' / '(empty)' with no prefix leakage — Eliminates secret prefix entropy and length disclosure in logs
- [Phase ?]: Added Conversation::model_category() and ChatBuilder::category() accessors — Enables external integration tests to verify category preservation without exposing mutable internal fields, preserving #[non_exhaustive] forward-compatibility.
- [Roadmap v0.2]: Mapped USER-01/02 to Phase 8 (alongside PREFS-*) per the milestone grouping of `o30O0e` + `L5adhe`; previous draft had them in Phase 7 with `PCck7e`.
- [Roadmap v0.2]: Mapped TOOL-06 (mocked fixture tests per RPC) to Phase 11 as the final verification gate; each phase's plan is expected to ship fixture tests alongside its RPC methods.

## Session

**Last session:** 2026-08-11T12:00:00.000Z
**Stopped at:** Completed v0.3 planning artifacts
**Resume file:** None

## Current Position

Phase: 13 — Core Protocol Constants
Plan: —
Status: Planning
Last activity: 2026-08-11 — created v0.3 milestone plan

## Operator Next Steps

- Review the four v0.3 phase plans under `.planning/phases/13-*` through `16-*`.
- Run `/gsd-execute-phase 13` to begin implementation.
