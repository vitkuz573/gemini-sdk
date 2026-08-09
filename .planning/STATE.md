# Project State

**Project:** Gemini SDK
**Initialized:** 2026-08-08
**Current milestone:** v0.1
**Current phase:** Phase 1: Stabilize v0.1 Core

milestone: v0.1

## Project Reference

See: `.planning/PROJECT.md` (updated 2026-08-08)

**Core value:** Developers can reliably integrate Gemini into Rust applications using a stable, documented, semver-respecting SDK that handles auth, protocol quirks, retries, and common content types out of the box.
**Current focus:** Stabilize public API, auth ergonomics, and chat/media types for v0.1.

## Phase Status

| Phase | Status | Plans | Progress |
|-------|--------|-------|----------|
| 1 — Stabilize v0.1 Core | ○ Pending | 0/3 | 0% |
| 2 — Reliability & Protocol Hardening | ○ Pending | 0/3 | 0% |
| 3 — Observability & Configurability | ○ Pending | 0/2 | 0% |
| 4 — Advanced Media & Sessions | ○ Pending | 0/2 | 0% |
| 5 — Tools & Auto-Refresh | ○ Pending | 0/3 | 0% |
| 6 — v1.0 Release | ○ Pending | 0/2 | 0% |

## Active Decisions

- semver progression: 0.1 → 0.2 → 1.0
- Cookie-based auth remains default; provider trait added for extensibility.
- Web frontend protocol remains the target; official REST/Vertex AI out of scope.

## Open Risks

- Google may change the undocumented WIZ protocol without notice.
- Browser attestation depends on Chrome CDP and live frontend selectors.
- Live-cookie integration tests cannot run in CI.

## Context

Codebase map available in `.planning/codebase/`.
Spike findings skill available at `.opencode/skills/spike-findings-gemini-sdk/SKILL.md`.

---
*Last updated: 2026-08-08 after initialization*
