# Requirements: Gemini SDK

**Defined:** 2026-08-12
**Core Value:** Developers can reliably integrate Gemini into Rust applications using a stable, documented, semver-respecting SDK that handles auth, protocol quirks, retries, and common content types out of the box.

## v0.5 Requirements

Requirements for the Browserless WAA Reverse milestone. Each maps to roadmap phases.

### Spike & Transform Validation

- [ ] **SPIKE-01**: Re-analyze `/home/vitaly/mitm.har` and spike 004 artifacts (`pairs.json`, `botguard.js`, slot dumps) to extract all available `(Waa/Create challenge, StreamGenerate slot-3 token)` pairs and document their structure.
- [ ] **SPIKE-02**: Capture or synthesize at least 3 additional independent `(challenge, slot-3)` pairs using the methodology in `CAPTURE_FULL_JS.md` or by replaying the existing challenge through a real browser, and record them as fixture data.
- [ ] **SPIKE-03**: Determine whether the slot-3 token is deterministic per challenge (same challenge → same token) and whether it is bound to session, request type, or timestamp; document the acceptance boundary.
- [ ] **SPIKE-04**: Decide the implementation strategy: pure Rust algorithmic port, lightweight JS VM harness, replay cache, or infeasible; close the spike with a verdict and a list of any remaining blockers.

### Generator Implementation

- [ ] **GEN-01**: Implement a browserless WAA token generator that produces a valid `!`-prefixed base64url slot-3 token from a `Waa/Create` challenge, consistent with captured fixture pairs.
- [ ] **GEN-02**: Keep the generator behind a new optional Cargo feature (`browserless-waa`) so the default crate remains lightweight.
- [ ] **GEN-03**: Add unit tests that verify the generator reproduces every captured fixture pair within the allowed tolerance (exact byte match for deterministic pairs).
- [ ] **GEN-04**: If the generator requires a JS runtime, use the lightest viable option (`rquickjs`) with mocked browser globals; do not embed a full browser engine.

### SDK Integration

- [ ] **INT-01**: Introduce a `src/waa/` module with challenge parsing, provider abstraction, and generator wiring without changing public API signatures.
- [ ] **INT-02**: Wire the browserless provider into `GeminiClient::run_waa_init_chain` so it populates `SessionState::waa_token` when the feature is enabled.
- [ ] **INT-03**: Preserve the existing CDP `browser-attestation` path as a fallback; do not remove or deprecate it in this milestone.
- [ ] **INT-04**: Ensure browserless WAA failures are non-fatal to session warm-up and fall back to the current synthetic/no-attestation behavior.

### Testing & Hardening

- [ ] **TEST-01**: Add fixture-based tests for challenge parsing and token generation using spike 004 data.
- [ ] **TEST-02**: Add an `#[ignore]` live-cookie integration test that verifies image upload works with the `browserless-waa` feature enabled and `browser-attestation` disabled.
- [ ] **TEST-03**: Redact WAA tokens, challenges, and BotGuard bytecode from logs, HAR captures, and error messages; add a regression test for redaction.
- [ ] **TEST-04**: Keep all quality gates green: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo doc --no-deps`, both with and without the new feature.

### Documentation & Release Readiness

- [ ] **DOC-01**: Document the `browserless-waa` feature, its experimental status, and the fallback behavior in `README.md` or a dedicated `LEGAL.md`.
- [ ] **DOC-02**: Include a clear reverse-engineering / Terms of Service disclaimer stating that the feature is unofficial and use may result in account restrictions.
- [ ] **DOC-03**: Update crate-level documentation and `src/waa/mod.rs` module docs to explain when to enable the feature and when to fall back to CDP.

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Advanced Browserless WAA

- **ADV-01**: Automatically detect BotGuard JS hash rotation and emit a warning or fallback to CDP when the generator may be stale.
- **ADV-02**: Expose internal attestation diagnostics so developers can see why browserless, CDP, or fallback was selected.
- **ADV-03**: Cache WAA tokens within a session lifetime if server acceptance experiments prove reuse is safe.

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Full BotGuard VM emulator in Rust | The VM is self-modifying, signal-collecting, and rotates; a full emulator is a research project, not a milestone deliverable. |
| Embedded V8 / Deno / Node runtime | Heavy dependencies that defeat the lightweight SDK goal; only `rquickjs` is acceptable as an optional fallback. |
| Removing the `browser-attestation` CDP feature | CDP is the ground-truth fallback and must remain available. |
| Hard-coded slot-3 tokens or cached cross-session tokens | Tokens are bound to challenge/session; hard-coding would break on rotation or reuse. |
| Telemetry / heartbeat RPCs to "look more like a browser" | Out of scope per PROJECT.md; adds traffic and privacy risk without attestation value. |
| Public API exposing raw WAA challenges or tokens | Undocumented protocol surface; keep internals crate-private to preserve flexibility. |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SPIKE-01 | Phase 21 | Pending |
| SPIKE-02 | Phase 21 | Pending |
| SPIKE-03 | Phase 21 | Pending |
| SPIKE-04 | Phase 21 | Pending |
| GEN-01 | Phase 22 | Pending |
| GEN-02 | Phase 22 | Pending |
| GEN-03 | Phase 22 | Pending |
| GEN-04 | Phase 22 | Pending |
| INT-01 | Phase 23 | Pending |
| INT-02 | Phase 23 | Pending |
| INT-03 | Phase 23 | Pending |
| INT-04 | Phase 23 | Pending |
| TEST-01 | Phase 24 | Pending |
| TEST-02 | Phase 24 | Pending |
| TEST-03 | Phase 24 | Pending |
| TEST-04 | Phase 24 | Pending |
| DOC-01 | Phase 25 | Pending |
| DOC-02 | Phase 25 | Pending |
| DOC-03 | Phase 25 | Pending |

**Coverage:**
- v0.5 requirements: 19 total
- Mapped to phases: 19
- Unmapped: 0 ✓

---
*Requirements defined: 2026-08-12*
*Last updated: 2026-08-12 after research synthesis*
