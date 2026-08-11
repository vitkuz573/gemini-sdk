# Roadmap: Gemini SDK

**Project:** Gemini SDK
**Created:** 2026-08-08
**Goal:** Stable, documented, semver-respecting Rust SDK for the Gemini web frontend, published on crates.io.

## Milestones

- ✅ **v0.1 Core** — Phases 1-6 (shipped 2026-08-10) — see [archive](milestones/v0.1-ROADMAP.md)
- ✅ **v0.2 API Expansion** — Phases 7-12 (shipped 2026-08-11) — see [archive](milestones/v0.2-ROADMAP.md)
- ✅ **v0.3 Magic String Elimination** — Phases 13-16 (shipped 2026-08-11) — see [archive](milestones/v0.3-ROADMAP.md)
- 🚧 **v0.4 StreamGenerate Slot Hardening** — Phase 17 (in progress)
- 📋 **v1.0 Stable Release** — Phases 18+ (planned)

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

<details>
<summary>✅ v0.3 Magic String Elimination (Phases 13-16) — SHIPPED 2026-08-11</summary>

- [x] Phase 13: Core Protocol Constants (1/1 plan) — completed 2026-08-11
- [x] Phase 14: Model, Chat & Upload Constants (1/1 plan) — completed 2026-08-11
- [x] Phase 15: Infrastructure Constants (1/1 plan) — completed 2026-08-11
- [x] Phase 16: Test & Example Cleanup + Regression Guard (1/1 plan) — completed 2026-08-11

</details>

### 🚧 v0.4 StreamGenerate Slot Hardening (In Progress)

- [ ] Phase 17: StreamGenerate Slot Hardening
  - Goal: Replace all raw numeric slot indices in `src/proto/slots.rs` with HAR-backed named constants and add a regression gate.
  - Requirements: SLOT-01 — SLOT-04, QUAL-01 — QUAL-06
  - **Plans:** 1 plan
  - Plans:
    - [ ] `17-01-PLAN.md` — Rename legacy slot constants, add missing named constants, refactor builder to use constants only, and add regression gate.
  - Success criteria:
    1. No raw `inner[\d+]` assignments remain in production builder code.
    2. All new constants have HAR-cited doc comments.
    3. `cargo test --all-targets` passes.
    4. `cargo clippy --all-targets -- -D warnings` passes.
    5. `cargo doc --no-deps` passes.
    6. Regression gate fails if raw numeric slot assignments are reintroduced.

### 📋 v1.0 Stable Release (Planned)

- [ ] Phase 18: API Audit & Deprecation Cleanup
- [ ] Phase 19: MSRV Policy & Documentation Polish
- [ ] Phase 20: crates.io Publication

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
| 13. Core Protocol Constants | v0.3 | 1/1 | Complete | 2026-08-11 |
| 14. Model, Chat & Upload Constants | v0.3 | 1/1 | Complete | 2026-08-11 |
| 15. Infrastructure Constants | v0.3 | 1/1 | Complete | 2026-08-11 |
| 16. Test & Example Cleanup + Regression Guard | v0.3 | 1/1 | Complete | 2026-08-11 |
| 17. StreamGenerate Slot Hardening | v0.4 | 0/TBD | In Progress | - |
| 18. API Audit & Deprecation Cleanup | v1.0 | 0/TBD | Not started | - |
| 19. MSRV Policy & Documentation Polish | v1.0 | 0/TBD | Not started | - |
| 20. crates.io Publication | v1.0 | 0/TBD | Not started | - |

---

*Last updated: 2026-08-11 — inserted v0.4 StreamGenerate Slot Hardening before v1.0 Stable Release*
