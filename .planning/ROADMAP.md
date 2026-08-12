# Roadmap: v0.5 Browserless WAA Reverse

**Milestone:** v0.5 Browserless WAA Reverse
**Goal:** Reverse-engineer and implement a browserless WAA (Web Application Authentication / BotGuard) token generator for `StreamGenerate` slot 3, so the SDK can obtain valid attestation context without requiring the user to launch headless Chrome. Correctness takes priority over keeping the dependency surface small.

## Phases

- [ ] **Phase 21: Spike Closure & Transform Validation** — Confirm the BotGuard challenge→slot-3 transform is reproducible, capture the fixture corpus, and choose the implementation strategy.
- [ ] **Phase 22: Generator Implementation** — Build the browserless slot-3 token generator behind an optional `browserless-waa` feature.
- [ ] **Phase 23: SDK Integration** — Wire the browserless provider into `GeminiClient` session warm-up while preserving the existing CDP fallback.
- [ ] **Phase 24: Testing & Hardening** — Add fixture, unit, and live-cookie tests; redact sensitive WAA artifacts; keep quality gates green with and without the feature.
- [ ] **Phase 25: Documentation & Release Readiness** — Document the experimental feature, ToS/legal disclaimer, and fallback behavior.

## Phase Overview

| # | Phase | Goal | Requirements | Success Criteria |
|---|-------|------|--------------|------------------|
| 21 | Spike Closure & Transform Validation | Prove the challenge→slot-3 transform and decide the implementation path. | SPIKE-01, SPIKE-02, SPIKE-03, SPIKE-04 | 4 |
| 22 | Generator Implementation | Produce a valid slot-3 token from a `Waa/Create` challenge without a browser. | GEN-01, GEN-02, GEN-03, GEN-04 | 4 |
| 23 | SDK Integration | Integrate the browserless path into session warm-up and preserve the CDP fallback. | INT-01, INT-02, INT-03, INT-04 | 4 |
| 24 | Testing & Hardening | Verify correctness, redaction, and quality gates across feature combinations. | TEST-01, TEST-02, TEST-03, TEST-04 | 4 |
| 25 | Documentation & Release Readiness | Surface legal risk and usage guidance to users. | DOC-01, DOC-02, DOC-03 | 3 |

**Total phases:** 5
**Total requirements mapped:** 19
**Coverage:** 100%

## Phase Details

### Phase 21: Spike Closure & Transform Validation

**Goal:** Close the reverse-engineering spike by reproducing the BotGuard `Waa/Create` challenge → `StreamGenerate` slot-3 token transform in a scriptable harness, capturing enough ground-truth pairs to constrain the algorithm, and choosing the implementation strategy.

**Depends on:** Phase 20 (previous milestone)

**Requirements:** SPIKE-01, SPIKE-02, SPIKE-03, SPIKE-04

**Success Criteria** (what must be TRUE):
1. Every available `(Waa/Create challenge, slot-3 token)` pair from `/home/vitaly/mitm.har` and spike 004 artifacts is extracted and documented with structure.
2. At least 3 additional independent `(challenge, slot-3)` pairs are captured or synthesized and recorded as fixture data.
3. The spike documents whether the slot-3 token is deterministic per challenge and what binds it (session, request type, timestamp, or none).
4. A concrete implementation strategy is chosen (pure Rust algorithm, JS engine harness, headless Chromium harness, replay cache, or infeasible) and the spike is closed with a verdict plus a list of any remaining blockers. The choice is based on reproducibility, not on minimizing dependencies.

**Plans:** TBD

### Phase 22: Generator Implementation

**Goal:** Implement a browserless WAA token generator that produces a valid `!`-prefixed base64url slot-3 token from a `Waa/Create` challenge, isolated behind an optional Cargo feature.

**Depends on:** Phase 21

**Requirements:** GEN-01, GEN-02, GEN-03, GEN-04

**Success Criteria** (what must be TRUE):
1. With the `browserless-waa` feature enabled, the generator produces a slot-3 token matching every deterministic captured fixture pair within the allowed tolerance.
2. The generator is gated behind a new optional Cargo feature (`browserless-waa`).
3. Unit tests verify the generator against all captured fixture pairs.
4. If the implementation uses a JS runtime, it uses an engine that actually reproduces captured tokens (`rquickjs`, `deno_core`, Node.js subprocess, or headless Chromium via CDP/Playwright/Puppeteer). The choice is driven by fidelity, not by size.

**Plans:** TBD

### Phase 23: SDK Integration

**Goal:** Wire the browserless WAA provider into the SDK session warm-up path so `GeminiClient` can populate `SessionState::waa_token` without the CDP feature, while leaving the existing CDP path untouched.

**Depends on:** Phase 22

**Requirements:** INT-01, INT-02, INT-03, INT-04

**Success Criteria** (what must be TRUE):
1. A new `src/waa/` module provides challenge parsing, provider abstraction, and generator wiring without changing existing public API signatures.
2. `GeminiClient::run_waa_init_chain` uses the browserless provider to populate `SessionState::waa_token` when the feature is enabled.
3. The existing CDP `browser-attestation` path still compiles and functions as a fallback.
4. Browserless WAA failures during warm-up are non-fatal and fall back to the current synthetic/no-attestation behavior.

**Plans:** TBD

### Phase 24: Testing & Hardening

**Goal:** Make the browserless WAA path testable, observable, and safe by adding fixtures, a live-cookie integration test, redaction, and gatekeeping quality checks.

**Depends on:** Phase 23

**Requirements:** TEST-01, TEST-02, TEST-03, TEST-04

**Success Criteria** (what must be TRUE):
1. Fixture-based tests exercise challenge parsing and token generation using spike 004 data.
2. An `#[ignore]` live-cookie integration test verifies image upload works with the `browserless-waa` feature enabled and `browser-attestation` disabled.
3. WAA tokens, challenges, and BotGuard bytecode are redacted from logs, HAR captures, and error messages, with a regression test covering redaction.
4. `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo doc --no-deps` pass both with and without the new feature.

**Plans:** TBD

### Phase 25: Documentation & Release Readiness

**Goal:** Surface the experimental nature, legal/ToS risk, and fallback behavior of browserless WAA to users before release.

**Depends on:** Phase 24

**Requirements:** DOC-01, DOC-02, DOC-03

**Success Criteria** (what must be TRUE):
1. `README.md` or a dedicated `LEGAL.md` documents the `browserless-waa` feature, its experimental status, and the fallback behavior.
2. A clear reverse-engineering / Terms of Service disclaimer states that the feature is unofficial and use may result in account restrictions.
3. Crate-level documentation and `src/waa/mod.rs` module docs explain when to enable the feature and when to fall back to CDP.

**Plans:** TBD

## Traceability

| Requirement | Phase |
|-------------|-------|
| SPIKE-01 | 21 |
| SPIKE-02 | 21 |
| SPIKE-03 | 21 |
| SPIKE-04 | 21 |
| GEN-01 | 22 |
| GEN-02 | 22 |
| GEN-03 | 22 |
| GEN-04 | 22 |
| INT-01 | 23 |
| INT-02 | 23 |
| INT-03 | 23 |
| INT-04 | 23 |
| TEST-01 | 24 |
| TEST-02 | 24 |
| TEST-03 | 24 |
| TEST-04 | 24 |
| DOC-01 | 25 |
| DOC-02 | 25 |
| DOC-03 | 25 |

**Coverage:**
- v0.5 requirements: 19 total
- Mapped to phases: 19
- Unmapped: 0 ✓

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 21. Spike Closure & Transform Validation | 0/0 | Not started | - |
| 22. Generator Implementation | 0/0 | Not started | - |
| 23. SDK Integration | 0/0 | Not started | - |
| 24. Testing & Hardening | 0/0 | Not started | - |
| 25. Documentation & Release Readiness | 0/0 | Not started | - |

## Open Questions / Research Flags

- Phase 21 requires fresh instrumented captures and server-acceptance experiments to resolve whether the transform is deterministic and browserless-feasible.
- Phase 22 may need deeper research if Phase 21 selects a VM-emulation path (`rquickjs` DOM mocking and signal fidelity).
- Phase 24 depends on access to a live signed-in cookie set for the `#[ignore]` image-upload test.
- Phase 25 requires maintainer review of the ToS/legal disclaimer wording; the research summary is not legal advice.
