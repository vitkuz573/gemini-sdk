# Project Research Summary

**Project:** gemini-sdk
**Domain:** Rust SDK for the Google Gemini web frontend, adding a non-browser path for BotGuard / WAA attestation tokens
**Milestone:** v0.5 Browserless WAA Reverse
**Researched:** 2026-08-12
**Confidence:** MEDIUM — feature shape and integration points are clear; the actual feasibility of a deterministic slot-3 transform is still provisional.

## Executive Summary

This milestone aims to make image uploads work in `gemini-sdk` without launching headless Chrome, by reverse-engineering or emulating enough of Google's BotGuard VM to generate the `StreamGenerate` slot-3 WAA token directly from the `Waa/Create` challenge. The recommended strategy is pragmatic: reuse the existing lightweight HTTP stack, keep any new heavy dependency behind an optional Cargo feature, and treat the CDP-based `browser-attestation` path as a permanent fallback rather than something to replace. The core risk is that BotGuard is a self-modifying, signal-collecting VM with rotating bytecode and strong environment checks, so a static "magic formula" port is unlikely to survive a Google update. The safest path is to prove a reproducible transform in a scriptable harness first, then productionize only the proven algorithm in Rust.

Based on the four research streams, the work should be structured as a spike-closure phase followed by algorithm porting, SDK integration, and hardening. The SDK architecture already has the right seams: `SessionState` owns `waa_token`, `run_waa_init_chain` already fetches the challenge, and `build_inner_req_list` already accepts the token for slot 3. A new `src/waa/` module can be added without disturbing the rest of the crate. The main research gap is whether the transform is deterministic enough to implement in pure Rust, or whether a lightweight JS harness is needed. Either way, the feature must be gated, the CDP path preserved, and legal/ToS risk disclosed to users.

## Key Findings

### Recommended Stack

The existing crate already covers most of what the browserless path needs: `base64`, `serde_json`, `bytes`, `sha1`, and `reqwest`. The only new runtime crates under consideration are `hex` and `nom` for an algorithmic Rust port, and `rquickjs` as a fallback VM-emulation harness. Snapshot testing should use `insta` as a dev-dependency. Heavy engines such as `deno_core`, `boa_engine`, and headless Chrome are explicitly out of scope for the default or even optional primary path.

**Core technologies:**
- `base64` / `serde_json` / `bytes` (existing): decode base64url challenges, parse the JSON+protobuf envelope, and slice binary payloads without copying.
- `sha1` (existing): reuse for any HMAC/SHA-1 transforms discovered in the VM; only add `hmac` after confirming the exact transform.
- `hex` (optional): hex-encode intermediate hashes and raw dumps during reverse engineering.
- `nom` (optional): parse structured binary WAA challenge blobs if length-prefixed fields or bytecode headers are found.
- `rquickjs` (optional): execute captured `botguard.js` with mocked DOM/browser globals if the algorithmic port fails. Lightweight compared to V8, with safe Rust bindings.
- `insta` (dev): snapshot `(challenge, slot-3 token)` pairs and decoded payloads.

See [STACK.md](STACK.md) for alternatives, Cargo feature proposal, and integration points.

### Expected Features

**Must have (table stakes):**
- WAA `Create` challenge acquisition — already implemented; must remain stable.
- Challenge → slot-3 token generator — the whole point of the milestone; highest risk and highest priority.
- Session warm-up integration — `run_waa_init_chain` populates `SessionState::waa_token` from the generator.
- Synthetic / fallback WAA context — tolerate `ogads` failures the same way the existing path does.
- Optional Cargo feature gating — keep the default SDK light.
- Fixture tests — validate the generator against captured pairs without live cookies.
- Live-cookie integration test — final acceptance gate, marked `#[ignore]` like existing live tests.

**Should have (differentiators):**
- No headless Chrome dependency for normal image uploads.
- Deterministic / reproducible token generation — valuable only if proven.
- Faster session warm-up and CI-friendly image-upload coverage.
- Pluggable generator backend (internal trait) if multiple strategies emerge.

**Defer (v2+):**
- Public attestation diagnostics.
- Automatic VM change detection.

See [FEATURES.md](FEATURES.md) for the full feature dependency graph and prioritization matrix.

### Architecture Approach

The browserless generator should be a new attestation provider that sits beside the existing CDP path. Introduce a `src/waa/` module with a small facade (`mod.rs`), a generator (`generator.rs`), challenge parsing (`challenge.rs`), and an optional VM harness (`vm.rs`). The client selects an `AttestationStrategy` (Disabled / Browserless / CDP) and writes the resulting token into `SessionState::waa_token`; `build_inner_req_list` consumes it as usual. This preserves the existing protocol-builder signatures and keeps `client.rs` from becoming a dumping ground for crypto/VM code.

**Major components:**
1. `GeminiClient` / `ClientConfig` — chooses attestation strategy and routes warm-up to the selected provider.
2. `src/waa/` — isolated browserless domain: provider trait/enum, challenge parsing, token generator, optional VM harness.
3. `src/attestation.rs` — unchanged CDP path; permanent fallback when `browser-attestation` is enabled.
4. `SessionState` / `proto/slots.rs` — existing state and slot builder already support slot-3 injection with no signature changes.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full data flow, anti-patterns, and suggested build order.

### Critical Pitfalls

1. **Treating BotGuard as a static algorithm instead of a self-modifying VM.** The transform is the VM interpreter plus a rotating program, not a single equation. Prove reproducibility across multiple fresh challenge→token pairs before porting to Rust.
2. **Underestimating browser signal fidelity.** BotGuard reads 100+ DOM/navigator signals. If a JS harness is needed, instrument a real browser first and mock the exact signals observed; otherwise use CDP as ground truth.
3. **Falling into anti-debug / anti-logger traps.** Breakpoints and `console.log` can shift the VM's chronometric seed. Prefer passive network capture (HAR, mitmproxy) and hook only at boundary functions.
4. **Assuming tokens are replayable across sessions or request types.** Server-side binding is an empirical question. Treat tokens as single-use per request until live experiments prove otherwise.
5. **Hard-coding BotGuard JS hashes, API keys, or constants.** Fetch interpreter hash and program from the live `Waa/Create` response, and degrade to CDP when the hash rotates.
6. **Removing the CDP path before proving parity.** The CDP path is the only known-good fallback and the ground-truth capture mechanism. Keep it feature-gated.
7. **Porting to Rust before proving the transform in a harness.** Define a hard spike exit criterion: the harness must generate an accepted token in a live-cookie request before any `src/waa/` production code is written.

See [PITFALLS.md](PITFALLS.md) for the full checklist, recovery strategies, and phase mapping.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Spike Closure & Transform Validation
**Rationale:** The bottleneck is understanding BotGuard, not language choice. Any Rust work before the transform is proven will likely be wasted. This phase must resolve the core feasibility question.
**Delivers:** A reproducible challenge→slot-3 transform in a scriptable harness; a decision on whether the path is algorithmic, VM-emulation, or infeasible; a corpus of validated `(challenge, slot-3)` pairs; instrumentation hygiene rules.
**Addresses:** Fixture corpus, determinism proof, server acceptance boundaries.
**Avoids:** Pitfalls 1 (static algorithm), 3 (anti-debug), 4 (token reuse), 8 (premature Rust port).

### Phase 2: Generator Implementation
**Rationale:** Once the transform is proven, productionize it. If pure Rust is viable, this is a focused crypto/bytecode module. If a JS harness is needed, it is the only place `rquickjs` and DOM mocks are introduced.
**Delivers:** `src/waa/generator.rs` (and `vm.rs` if needed) with a working token generator; unit tests against spike fixtures.
**Uses:** `hex`, `nom`, `sha1` for algorithmic path; `rquickjs` for VM-emulation path; `insta` for snapshot regression tests.
**Implements:** `WaaProvider` trait / `AttestationStrategy` enum from the architecture design.
**Avoids:** Pitfalls 2 (signal fidelity), 5 (hard-coded constants).

### Phase 3: SDK Integration
**Rationale:** The SDK already has the right seams, so wiring should be straightforward once the generator exists.
**Delivers:** `src/waa/mod.rs` and `src/waa/challenge.rs`; `ClientConfig::waa_strategy`; updated `run_waa_init_chain` that prefers browserless and falls back to CDP or synthetic context; optional `browserless-waa` Cargo feature.
**Implements:** Provider abstraction pattern, state-driven token injection pattern, feature-gated heavy dependencies pattern.
**Avoids:** Pitfalls 2 (signal fidelity — handled by generator), 6 (CDP removal), integration gotchas around feature flags and error propagation.

### Phase 4: Testing & Hardening
**Rationale:** The feature is fragile to Google-side rotation, so regression coverage and clear failure modes are essential.
**Delivers:** Fixture tests with `insta`, `#[ignore]` live-cookie image-upload test, structured `Error::AttestationFailed` variants, token/challenge redaction in logs, `spawn_blocking` integration for any CPU-heavy VM work.
**Avoids:** Pitfalls 4 (reuse), 5 (rotation), integration gotchas around errors, performance traps (blocking async runtime).

### Phase 5: Documentation & Release Readiness
**Rationale:** Reverse-engineering an anti-abuse system carries legal and UX risk that must be surfaced before release.
**Delivers:** README / `LEGAL.md` ToS disclaimer, clear experimental feature naming (`browserless-waa`), MSRV and dependency documentation, fallback behavior explained to users.
**Avoids:** Pitfall 7 (legal/ToS risk), UX pitfalls around silent failures and unclear opt-out.

### Phase Ordering Rationale

- **Spike before implementation:** Phase 1 must finish before Phase 2. The research explicitly flags premature Rust porting as a top failure mode.
- **Generator before integration:** Phase 2 must deliver a callable `WaaProvider` before Phase 3 can wire it into `run_waa_init_chain`.
- **Integration before live hardening:** Phase 3 puts the token into `SessionState`; Phase 4 then verifies end-to-end behavior, error handling, and performance.
- **Documentation last:** Phase 5 depends on the final feature shape and fallback semantics decided in Phases 3–4.
- **CDP fallback is preserved throughout:** Architecture and pitfalls both demand the existing `browser-attestation` feature remain untouched as a permanent fallback.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1:** HIGH — BotGuard internals are undocumented; need more instrumented captures and server-acceptance experiments. Consider `/gsd-plan-phase --research-phase 1`.
- **Phase 2:** MEDIUM-HIGH — If Phase 1 shows a VM-emulation path is required, `rquickjs` DOM mocking and signal fidelity need focused research. If the algorithmic path is proven, confidence rises to MEDIUM.

Phases with standard patterns (skip research-phase):
- **Phase 3:** LOW — SDK integration uses existing patterns: optional Cargo features, async warm-up chain, provider trait. The architecture is well mapped.
- **Phase 4:** LOW-MEDIUM — Rust testing and error-design patterns are standard; only the live-cookie test needs special handling.
- **Phase 5:** LOW — Documentation and legal-disclosure patterns are conventional, though the wording should be reviewed by a maintainer.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM | Existing crate dependencies are authoritative; optional additions (`rquickjs`, `nom`, `hex`) are reasonable but depend on which generator path is chosen. Deno/Boa/CDP avoidance is clear. |
| Features | MEDIUM | Feature landscape is well understood from spike 004 and existing SDK code. The only uncertainty is whether deterministic generation is achievable. |
| Architecture | MEDIUM-HIGH | Integration points (`SessionState`, `run_waa_init_chain`, slot builder) are concrete and unchanged; the proposed `src/waa/` module follows existing crate conventions. |
| Pitfalls | LOW-MEDIUM | Findings are based on third-party reverse-engineering write-ups and inferred VM behavior, not official documentation. The risk list itself is credible, but specific technical claims are provisional. |

**Overall confidence:** MEDIUM — the project shape is clear and the integration is low-risk, but the central feasibility question (can we generate valid slot-3 tokens without a browser?) remains unproven and must be resolved in Phase 1.

### Gaps to Address

- **Deterministic transform feasibility:** Cannot be resolved by desk research alone. Requires fresh instrumented captures and live-cookie experiments in Phase 1. If infeasible, the milestone should be closed with a documented fallback to CDP.
- **Signal set for VM emulation:** If `rquickjs` is needed, the exact DOM/navigator signals the current BotGuard version reads are unknown. Needs real-browser instrumentation.
- **Token binding/validity window:** How long a token is valid and whether it is bound to request type or session is unknown. Needs server-acceptance experiments.
- **Legal wording:** The ToS disclaimer language should be reviewed by the project maintainer; this summary is not legal advice.

## Sources

### Primary (HIGH confidence)
- `src/client.rs` (`run_waa_init_chain`, `waa_create`, `ogads_get_async_data`) — existing WAA warm-up flow.
- `src/session.rs` — `SessionState` fields and serialization.
- `src/proto/slots.rs`, `src/proto/mod.rs` — slot-3 injection and WAA body builders.
- `src/attestation.rs` — existing CDP-based attestation path.
- `Cargo.toml` — existing dependencies and feature gating.

### Secondary (MEDIUM confidence)
- `.planning/spikes/004-waa-token/README.md` — spike findings, captured pairs, and current gaps.
- `.planning/spikes/004-waa-token/CAPTURE_GUIDE.md` and `CAPTURE_FULL_JS.md` — capture methodology.
- `.opencode/skills/spike-findings-gemini-sdk/references/waa-attestation.md` — WAA chain and context header template.
- `docs.rs` / `crates.io` metadata for `rquickjs` 0.12.2, `nom` 8.0.0, `hex` 0.4.3, `insta` 1.48.0 — version and MSRV verification.

### Tertiary (LOW confidence)
- tomkabel/google-botguard-security-research — BotGuard VM architecture and anti-debug behavior. Third-party reverse engineering; needs validation.
- dsekz/botguard-reverse — VM opcode and self-modifying bytecode analysis. Third-party reverse engineering; needs validation.
- LuanRT/BgUtils / `bgutils-js` — documented WAA `Create`/`GenerateIT` pattern. Widely used but not official.
- think.resoneo.com BotGuard v41 analysis — fingerprinting scope and ARX cipher. Third-party analysis; needs validation.
- Google Terms of Service and general reverse-engineering law references — inferential, not legal advice.

---
*Research completed: 2026-08-12*
*Ready for roadmap: yes*
