# Feature Landscape: Browserless WAA Token Generator

**Domain:** Rust SDK for the Google Gemini web frontend (`gemini.google.com`), adding a non-browser path for BotGuard / WAA attestation.
**Researched:** 2026-08-12
**Confidence:** MEDIUM — based on spike 004 artifacts, captured HAR, and the existing `GeminiClient` WAA init chain. The core *shape* of the work is clear; the actual feasibility of a deterministic transform depends on additional instrumented captures that are not yet in the repo.

## Feature Landscape

### Table Stakes (Required for the Browserless WAA Path)

Features that must exist for the browserless path to be a credible replacement for the headless-CDP attestation capture.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| WAA `Create` challenge acquisition | Same prerequisite as the CDP path; the browserless generator needs a fresh challenge token to transform. | LOW | Already implemented in `GeminiClient::waa_create`. Must remain stable. |
| Challenge → slot-3 token transform | The whole point of the milestone: produce the `!`-prefixed base64url token that currently comes from `botguard.bg(M, cb)`. | HIGH | Obfuscated VM; needs either a discovered deterministic algorithm or a validated replay/derived token. |
| Session warm-up integration | The generated token must land in `SessionState::waa_token` and be used by `build_inner_req_list` for `StreamGenerate`. | MEDIUM | `run_waa_init_chain` already wires `waa_token` and `waa_context`; add a new branch before or after `ogads_get_async_data`. |
| Synthetic / fallback WAA context | If `ogads GetAsyncData` fails, the SDK already falls back to `build_default_waa_context`. The browserless path must tolerate the same fallback. | LOW | `build_waa_context_header` and `is_valid_waa_context_array` already handle shape validation. |
| Optional feature gating | Keep the core crate lightweight and avoid new heavy dependencies if the generator needs a JS harness or large lookup tables. | LOW | Follow the `browser-attestation` pattern: expose behind a Cargo feature such as `browserless-waa`. |
| Fixture tests | Regression tests need captured `(challenge, slot3)` pairs so the generator can be validated without live cookies. | MEDIUM | Spike 004 has `pairs.json` and decoded binaries; these become fixtures. |
| Live-cookie integration test | The ultimate acceptance gate: image upload works without the `browser-attestation` feature enabled. | HIGH | Requires valid cookies and a working generator; mark `#[ignore]` like other live tests. |

### Differentiators (Where the Browserless Path Wins)

Features that make the browserless path preferable to the existing CDP capture path.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| No headless Chrome dependency | Removes `tokio-tungstenite`, Chrome install, and process management for users who only need text/image chat. | MEDIUM | The `browser-attestation` feature can remain for advanced debugging, but it becomes optional for normal uploads. |
| Deterministic / reproducible generation | Same challenge produces same slot-3 across runs, making tests and replays stable. | HIGH | Only valuable if proven true; if the token includes a time/random component, document it and scope accordingly. |
| Faster session warm-up | Avoids browser launch/navigate/capture cycle; reduces latency by seconds. | LOW | Warm-up becomes pure HTTP once the generator is wired in. |
| CI-friendly image-upload tests | Fixtures + deterministic generator let CI verify slot-3 shape without launching Chrome. | MEDIUM | Enables `#[test]`-level coverage for a path currently gated behind `#[ignore]` live-cookie tests. |
| Pluggable generator backend | If multiple strategies emerge (algorithmic, replay cache, VM harness), an internal trait lets the SDK swap implementations. | MEDIUM | Avoid leaking this as public API until it stabilizes. |

### Anti-Features (Do Not Build)

Features that are tempting but would expand scope, increase maintenance, or create legal/reliability problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Full BotGuard VM emulator in Rust | The "correct" way to reproduce the token. | Google's VM is self-modifying, signal-collecting, and intentionally hostile to emulation; a full port is a research project of its own and breaks silently when Google updates the JS. | Target a constrained transform/replay strategy and fall back to CDP when it fails. |
| Embedded JavaScript engine (V8/QuickJS/Node) in the crate | Lets us run `botguard.js` directly. | Adds huge binary/dependency cost, licensing complexity, and still requires DOM mocks; defeats the purpose of a lightweight Rust SDK. | If a JS harness is needed for discovery, keep it in spike scripts outside the crate. |
| Removing the existing `browser-attestation` feature | Once browserless works, CDP feels redundant. | The CDP path is the ground-truth capture mechanism and the only fallback if Google rotates the VM; removing it would make recovery impossible. | Keep both features; prefer browserless, fall back to CDP when enabled. |
| Hard-coded slot-3 tokens | Simplest way to make image uploads "work" without reverse engineering. | Tokens are bound to a challenge/session and will be rejected server-side; also fragile to rotation. | Use captured `(challenge, slot3)` pairs only as fixtures for validating a real generator. |
| Public API exposing WAA internals | Developers might want raw challenge/token access. | The protocol is undocumented and changes without notice; exposing internals locks in unstable surface. | Keep the generator internal; expose only opt-in feature flags and warm-up behavior. |
| Telemetry / heartbeat RPCs | "Make the SDK look more like a real browser." | Out of scope per `PROJECT.md`; adds traffic, privacy risk, and no attestation value. | Do not implement. |

## Feature Dependencies

```
WAA Create challenge acquisition
    └──requires──> Valid signed-in session (already exists)

Challenge → slot-3 token generator
    └──requires──> WAA Create challenge acquisition
    └──requires──> Fixture corpus of (challenge, slot3) pairs
    └──enhances──> Session warm-up integration

Session warm-up integration
    └──requires──> Challenge → slot-3 token generator (or fallback)
    └──requires──> Synthetic WAA context fallback (already exists)
    └──requires──> ogads GetAsyncData (already exists, best-effort)

Live image-upload integration test
    └──requires──> Session warm-up integration
    └──requires──> Image upload RPC path (already exists)

Optional Cargo feature
    └──conflicts──> None; can coexist with browser-attestation
```

### Dependency Notes

- **Challenge acquisition must remain first.** The generator cannot run without the `Waa/Create` token. Reusing the existing `waa_create` RPC keeps the milestone focused on the transform.
- **The generator is an enhancement, not a replacement, for session warm-up.** `run_waa_init_chain` should try the browserless path and fall back to the existing synthetic `waa_token = None` behavior if the generator is disabled or fails.
- **The live image-upload test depends on warm-up.** Until `session.waa_token` is populated by the generator, `build_inner_req_list` will leave slot 3 empty and uploads will fail.
- **The optional feature is independent of `browser-attestation`.** A user can enable neither, one, or both. If both are enabled, the SDK should prefer the lightweight browserless path and only launch Chrome if browserless fails and CDP is explicitly requested.

## MVP Definition

### Launch With (v0.5)

Minimum scope that makes image uploads work without headless Chrome for the captured fixture cases.

- [ ] **WAA challenge acquisition remains stable** — no regressions in `waa_create`.
- [ ] **A token-generation strategy is chosen and implemented** — algorithmic transform, replay cache, or VM-derived deterministic function, documented with its acceptance boundaries.
- [ ] **Generator is gated behind a Cargo feature** — e.g. `browserless-waa`.
- [ ] **Session warm-up prefers browserless when enabled** — `run_waa_init_chain` populates `session.waa_token` from the generator instead of leaving it `None`.
- [ ] **Fallback to existing behavior** — if the generator is disabled or returns an error, warm-up continues with `waa_token = None` and the existing synthetic context.
- [ ] **Fixture tests validate the generator** — at least one captured `(challenge, slot3)` pair produces the expected slot-3 token.
- [ ] **`cargo test`, `cargo clippy`, `cargo doc` stay green** — including when the new feature is both on and off.

### Add After Validation (v0.5.x or v1.1)

Features to add once the generator is proven against live traffic.

- [ ] **Live-cookie integration test for image upload without `browser-attestation`** — trigger: the fixture generator passes and a maintainer can run live tests.
- [ ] **Multiple challenge/session coverage** — trigger: additional instrumented captures become available.
- [ ] **Generator strategy selection API (internal)** — trigger: more than one valid strategy exists.

### Future Consideration (v2+)

Features to defer until the core browserless path is stable and widely used.

- [ ] **Public attestation diagnostics** — expose why warm-up chose browserless vs CDP vs fallback.
- [ ] **Automatic VM change detection** — detect when the BotGuard JS changes and warn that the generator may need revalidation.

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| WAA challenge acquisition stability | HIGH | LOW | P1 (done) |
| Challenge → slot-3 generator | HIGH | HIGH | P1 |
| Session warm-up integration | HIGH | MEDIUM | P1 |
| Optional Cargo feature | MEDIUM | LOW | P1 |
| Fixture tests | HIGH | MEDIUM | P1 |
| Synthetic context fallback | HIGH | LOW | P1 (done) |
| Live image-upload integration test | HIGH | HIGH | P2 (blocked on live validation) |
| Pluggable generator backend | LOW | MEDIUM | P3 |
| CDP path removal | NEGATIVE | LOW | Anti-feature |
| Full BotGuard VM emulator | LOW | VERY HIGH | Anti-feature |

**Priority key:**
- P1: Must have for v0.5 browserless milestone
- P2: Should have once generator is live-validated
- P3: Nice to have / future consideration

## Competitor / Ecosystem Feature Analysis

| Feature | Existing CDP path (`browser-attestation`) | Browserless goal | Notes |
|---------|-------------------------------------------|------------------|-------|
| Heavy dependency | Chrome + `tokio-tungstenite` | None beyond existing HTTP stack | Main motivation for the milestone. |
| Ground-truth capture | Yes (real browser token) | No (derived/replayed token) | CDP remains the reference if the browserless token is rejected. |
| Determinism | No (fresh browser each time) | Target: yes | Only achievable if the VM transform is not time-bound. |
| CI suitability | Poor (needs Chrome) | Good (fixture-based) | Enables better regression coverage. |
| Maintenance surface | Chrome version drift, DOM selectors | Generator algorithm drift | New failure mode: Google rotates BotGuard and the transform breaks. |

## Sources

- Spike 004: `.planning/spikes/004-waa-token/README.md`
- Capture guide: `.planning/spikes/004-waa-token/CAPTURE_GUIDE.md`
- Existing WAA init chain: `src/client.rs` (`run_waa_init_chain`, `waa_create`, `ogads_get_async_data`, `build_waa_context_header`)
- Existing slot builder and StreamGenerate wiring: `src/proto/mod.rs`, `src/proto/slots.rs`
- Existing CDP attestation path: `src/attestation.rs`
- Project scope and constraints: `.planning/PROJECT.md`

---
*Feature research for: browserless WAA / BotGuard token generation in gemini-sdk*
*Researched: 2026-08-12*
