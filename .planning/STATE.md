---
gsd_state_version: 1.0
milestone: v0.5
milestone_name: Usage Stats Reliability
current_phase: 19
current_phase_name: Payload & Parser Alignment
current_plan: P01 — not started
status: planning
stopped_at: Planning phase 19
last_updated: "2026-08-11T15:40:00.000Z"
last_activity: 2026-08-11
last_activity_desc: Planned phase 19 payload parser alignment
progress:
  total_phases: 20
  completed_phases: 18
  total_plans: 1
  completed_plans: 0
  percent: 90
---

# Project State

**Project:** Gemini SDK
**Initialized:** 2026-08-08
**Current milestone:** v0.5 — Usage Stats Reliability
**Current phase:** 18
**Current Plan:** P01 — not started
**Total Plans in Phase:** 3

milestone: v0.5

## Project Reference

See: `.planning/PROJECT.md` (updated 2026-08-11)

**Core value:** Developers can reliably integrate Gemini into Rust applications using a stable, documented, semver-respecting SDK that handles auth, protocol quirks, retries, and common content types out of the box.
**Current focus:** Fix `GeminiClient::get_usage_stats` auth/payload mismatch so it returns real usage statistics.

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
| 13 — Core Protocol Constants | ✓ Complete | 1/1 | 100% |
| 14 — Model/Chat/Upload Constants | ✓ Complete | 1/1 | 100% |
| 15 — Infrastructure Constants | ✓ Complete | 1/1 | 100% |
| 16 — Test & Example Cleanup + Regression Guard | ✓ Complete | 1/1 | 100% |
| 17 — StreamGenerate Slot Hardening | ✓ Complete | 1/1 | 100% |
| 18 — Auth Header Parity for Usage Stats | ✓ Complete | 3/3 | 100% |
| 19 — Payload & Parser Alignment | 🔄 In progress | 0/1 | 0% |
| 20 — Live Verification & CLI Contract | ⏳ Not started | 0/TBD | 0% |

## Active Decisions

- v0.4: All StreamGenerate slot indices used by the SDK must be named constants in `src/proto/indices.rs`, with HAR-cited doc comments.
- v0.4: Legacy misleading names (`SLOT_REQUEST_UUID` for slot 10, `SLOT_CATEGORY` for slot 7, etc.) are renamed to match observed semantics.
- v0.3: Magic strings must be centralized as named constants without changing public API behavior or names. Existing `pub(crate) const` RPC IDs are a pattern to extend, not replace.
- semver progression: 0.1 → 0.2 → 0.3 → 0.4 → 1.0
- Cookie-based auth remains default; provider trait added for extensibility (async `CredentialsProvider` with boxed futures, no async-trait dependency).
- Web frontend protocol remains the target; official REST/Vertex AI out of scope.
- Telemetry / reporting RPCs and `signaler-pa` / `myactivity.google.com` endpoints remain out of scope (no library SDK should emit analytics traffic).

## Open Risks

- Google may change the undocumented WIZ protocol without notice.
- Browser attestation depends on Chrome CDP and live frontend selectors.
- Live-cookie integration tests cannot run in CI.
- Renaming constants is safe internally, but any future backports must use new names.

## Context

Codebase map available in `.planning/codebase/`.
Spike findings skill available at `.opencode/skills/spike-findings-gemini-sdk/SKILL.md`.
v0.2 RPC coverage derived from spike 001 (HAR API coverage).
v0.4 slot naming derived from spike references/protocol.md and live HAR at `/home/vitaly/mitm.har`.

---
*Last updated: 2026-08-11 — started v0.5 Usage Stats Reliability*

## Performance Metrics

| Phase | Plan | Duration | Notes |
|-------|------|----------|-------|
| Phase 12-live-testing-resilience P01 | 45 min | 5 tasks | 11 files |
| Phase 11-protocol-drift-integration P01 | 20 min | 3 tasks | 5 files |
| Phase 09-locale-model-config P01 | 11 min | 4 tasks | 8 files |
| Phase 08-user-profile-preferences P01 | 6 min | 3 tasks | 8 files |
| Phase 17 PP01 | 22m | 3 tasks | 2 files |

## Decisions

- [v0.5 start]: Insert v0.5 Usage Stats Reliability before v1.0 Stable Release because `get_usage_stats` returns `{}` against a live signed-in account.
- [v0.4 start]: Insert v0.4 StreamGenerate Slot Hardening before v1.0 Stable Release because raw slot indices survived v0.3.
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

**Last session:** 2026-08-11T13:44:32.929Z
**Stopped at:** Completed milestone v0.5 planning
**Resume file:** None

## Current Position

Phase: 19 — Payload & Parser Alignment
Plan: P01 — not started
Status: Planning complete; ready to execute
Last activity: 2026-08-11 — Planned phase 19 payload parser alignment

## Operator Next Steps

- Execute plan 19-01 (TDD: align parser and expose typed accessors).
