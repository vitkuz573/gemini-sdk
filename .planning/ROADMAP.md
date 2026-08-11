# Roadmap: Gemini SDK

**Project:** Gemini SDK
**Created:** 2026-08-08
**Goal:** Stable, documented, semver-respecting Rust SDK for the Gemini web frontend, published on crates.io.

## Milestones

- ✅ **v0.1 Core** — Phases 1-6 (shipped 2026-08-10) — see [archive](milestones/v0.1-ROADMAP.md)
- ✅ **v0.2 API Expansion** — Phases 7-12 (shipped 2026-08-11) — see [archive](milestones/v0.2-ROADMAP.md)
- 🚧 **v0.3 Magic String Elimination** — Phases 13-16 (planned)
- ⏳ **v1.0 Stable Release** — Phases 17+ (planned)

## Phases

<details>
<summary>✅ v0.1 Core (Phases 1-6) — SHIPPED 2026-08-10</summary>

- [x] Phase 1: Stabilize v0.1 Core (4/4 plans) — completed 2026-08-09
- [x] Phase 2: Reliability & Protocol Hardening (3/3 plans) — completed 2026-08-10
- [x] Phase 3: Observability & Configurability (5/5 plans) — completed 2026-08-10
- [x] Phase 4: Advanced Media & Sessions (2/2 plans) — completed 2026-08-10
- [x] Phase 5: Tools & Auto-Refresh (3/3 plans) — completed 2026-08-10
- [x] Phase 6: v1.0 Release (2/2 plans) — completed 2026-08-10

</details>

<details>
<summary>✅ v0.2 API Expansion (Phases 7-12) — SHIPPED 2026-08-11</summary>

- [x] Phase 7: Conversation Actions (1/1 plan) — completed 2026-08-10
- [x] Phase 8: User Profile & Preferences (1/1 plan) — completed 2026-08-10
- [x] Phase 9: Locale & Model Config (1/1 plan) — completed 2026-08-10
- [x] Phase 10: Settings Pages (1/1 plan) — completed 2026-08-10
- [x] Phase 11: Protocol Drift & Integration (1/1 plan) — completed 2026-08-10
- [x] Phase 12: Live Testing & Backend Resilience (1/1 plan) — completed 2026-08-11

</details>

### 🚧 v0.3 Magic String Elimination (Planned)

- [x] Phase 13: Core Protocol Constants (completed 2026-08-11)
  - [x] 13-01-PLAN.md — Centralize URL paths, batchexecute query keys, transport markers, WIZ/session keys, and RPC IDs.
- [x] Phase 14: Model, Chat & Upload Constants (completed 2026-08-11)
  - [x] 14-01-PLAN.md — Centralize model/category strings, chat roles, MIME types, and upload headers/endpoints.
- [x] Phase 15: Infrastructure Constants
  - [x] 15-01-PLAN.md — Centralize headers/base URLs, HAR/redaction strings, transient markers, tracing/metrics, CDP/attestation, and tool schema keys.
- [ ] Phase 16: Test & Example Cleanup + Regression Guard
  - [ ] 16-01-PLAN.md — Clean up tests/examples and add a regression gate for eliminated magic strings.

### ⏳ v1.0 Stable Release (Planned)

- [ ] Phase 17: API Audit & Deprecation Cleanup
- [ ] Phase 18: MSRV Policy & Documentation Polish
- [ ] Phase 19: crates.io Publication

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
| ----- | --------- | -------------- | ------ | --------- |
| 1. Stabilize v0.1 Core | v0.1 | 4/4 | Complete | 2026-08-09 |
| 2. Reliability & Protocol Hardening | v0.1 | 3/3 | Complete | 2026-08-10 |
| 3. Observability & Configurability | v0.1 | 5/5 | Complete | 2026-08-10 |
| 4. Advanced Media & Sessions | v0.1 | 2/2 | Complete | 2026-08-10 |
| 5. Tools & Auto-Refresh | v0.1 | 3/3 | Complete | 2026-08-10 |
| 6. v1.0 Release | v0.1 | 2/2 | Complete | 2026-08-10 |
| 7. Conversation Actions | v0.2 | 1/1 | Complete | 2026-08-10 |
| 8. User Profile & Preferences | v0.2 | 1/1 | Complete | 2026-08-10 |
| 9. Locale & Model Config | v0.2 | 1/1 | Complete | 2026-08-10 |
| 10. Settings Pages | v0.2 | 1/1 | Complete | 2026-08-10 |
| 11. Protocol Drift & Integration | v0.2 | 1/1 | Complete | 2026-08-10 |
| 12. Live Testing & Backend Resilience | v0.2 | 1/1 | Complete | 2026-08-11 |
| 13. Core Protocol Constants | v0.3 | 1/1 | Complete    | 2026-08-11 |
| 14. Model, Chat & Upload Constants | v0.3 | 1/1 | Complete    | 2026-08-11 |
| 15. Infrastructure Constants | v0.3 | 1/1 | Complete    | 2026-08-11 |
| 16. Test & Example Cleanup + Regression Guard | v0.3 | 0/TBD | Not started | - |
| 17. API Audit & Deprecation Cleanup | v1.0 | 0/TBD | Not started | - |
| 18. MSRV Policy & Documentation Polish | v1.0 | 0/TBD | Not started | - |
| 19. crates.io Publication | v1.0 | 0/TBD | Not started | - |

---

*Last updated: 2026-08-11 — inserted v0.3 milestone between v0.2 and v1.0*
