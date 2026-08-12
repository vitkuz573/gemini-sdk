# Pitfalls Research

**Domain:** Browserless WAA / BotGuard token generation for an existing Rust SDK
**Project:** Gemini SDK
**Researched:** 2026-08-12
**Confidence:** LOW

> **Confidence note:** Findings below are synthesized from public reverse-engineering write-ups, BotGuard analysis repositories, and the project's own spike 004 artifacts. Google does not document BotGuard internals, so all technical claims about VM behavior, anti-debug details, and signal sets are inferred from third-party research and should be treated as provisional. The legal/ToS notes are based on standard platform terms and case law patterns, not legal advice.

## Critical Pitfalls

### Pitfall 1: Treating BotGuard as a static algorithm instead of a self-modifying VM

**What goes wrong:**
Developers assume the WAA challenge token is transformed by a fixed, portable cryptographic routine (e.g., "decode, XOR with a key, base64url-encode"). They try to extract one magic constant or one bytecode snippet and write a Rust generator from it. The resulting token is rejected by Google as soon as the JS hash rotates or the challenge program changes.

**Why it happens:**
Spike 004 shows the slot-3 payload is binary and not a substring of the WAA challenge, which makes the transform look algorithmic. Public analysis confirms BotGuard is a register-based VM that executes a bytecode program, creates new opcodes at runtime via `LOADSTRING`/`EVAL`, and encrypts bytecode reads with rolling keys. The algorithm is the VM interpreter plus the specific program, not a single equation.

**How to avoid:**
- Do not port a "formula" until you can reproduce multiple challenge→token pairs in a harness.
- Treat the problem as "emulate or replace the VM interpreter for a given JS hash/program" rather than "find the crypto key."
- If porting, design the Rust module around the VM interpreter structure (registers, opcode dispatch, memory reader) and load the program dynamically from the WAA challenge response.

**Warning signs:**
- Token works once against live cookies but fails a few hours later.
- Decoded slot-3 payload changes materially when only the BotGuard JS URL hash changes.
- Static deobfuscation produces a different control-flow graph on each JS fetch.

**Phase to address:**
v0.5 Browserless WAA Reverse — specifically the spike work before any Rust porting begins.

---

### Pitfall 2: Underestimating the browser signal fidelity required

**What goes wrong:**
A Node.js/QuickJS/V8 harness is built with partial DOM mocks (a fake `document`, `navigator`, `window`). The VM runs without crashing but produces a token that the server rejects because critical signals (`trustedTypes`, iframe `load`/`error` events, `performance.now` jitter, `localStorage`, WebGL/Canvas fingerprints, `requestIdleCallback`) are missing or inconsistent.

**Why it happens:**
BotGuard is described as a browser *attestation* system, not merely a fingerprint: the token proves the VM executed inside a genuine browser environment. Public analysis notes 120+ DOM/navigator characteristics and behavioral event tracking (mouse, keyboard, scroll, focus, visibility, resize). A minimal JS runtime will trip environment checks even if the bytecode interpreter is correct.

**How to avoid:**
- Instrument a real browser first to enumerate exactly which signals the specific BotGuard version reads.
- If a harness is unavoidable, use a real browser engine (headless Chromium via CDP, Playwright, Puppeteer with stealth patches) rather than a JS-only VM.
- Capture the full DOM state, storage, cookies, and loaded gstatic bundles at the moment of `botguard.bg()` invocation.

**Warning signs:**
- VM executes but callback returns an empty or unexpectedly short token.
- Server responds with generic 400s or WIZ errors despite a syntactically valid slot-3 token.
- Replaying the same challenge through a real browser yields a different accepted token than the harness.

**Phase to address:**
v0.5 Browserless WAA Reverse — data-capture sub-phase and harness prototyping.

---

### Pitfall 3: Ignoring anti-debug and anti-logger traps during instrumentation

**What goes wrong:**
While trying to trace `botguard.bg()` arguments or set breakpoints, the VM detects the instrumentation via timing deltas (`performance.now()` vs `Date.now()`) or console/log hook traps. It silently diverges to a garbage execution path, producing tokens that look valid locally but are rejected server-side. Worse, the corrupted execution path is mistaken for "normal" behavior and copied into the Rust port.

**Why it happens:**
Public reverse-engineering reports describe BotGuard's chronometric defense: breakpoints pause execution but not the clock, so the seed used to decrypt the next bytecode block mutates. An anti-logger mechanism overrides object methods so that `console.log` shifts the memory reader pointer and corrupts the instruction stream.

**How to avoid:**
- Prefer passive network capture (HAR with response bodies, mitmproxy) and external script instrumentation over breakpoints inside the VM.
- If JS hooks are required, hook at the boundary (`botguard.bg`, `eval`, `atob`, `document.createElement`) rather than inside VM internals, and avoid `console.log` inside the hooked functions.
- Cross-check tokens produced under instrumentation against tokens produced by a completely untouched browser session; if they differ, the instrumentation is altering behavior.

**Warning signs:**
- Token changes every time DevTools is open vs. closed.
- Logpoints cause the page to reload or the VM to return early.
- Captured arguments to `botguard.bg()` do not reproduce the observed slot-3 token when replayed.

**Phase to address:**
v0.5 Browserless WAA Reverse — instrumentation planning and data validation.

---

### Pitfall 4: Assuming a captured slot-3 token is replayable across sessions or request types

**What goes wrong:**
The team captures one slot-3 token from a text chat and hard-codes it, or caches a token per challenge and reuses it for image uploads. The SDK works briefly in local tests, then image uploads and multi-turn state begin failing with WIZ errors.

**Why it happens:**
Google may bind tokens to the specific request context (session, `x-goog-authuser`, request type, content binding, timestamp, or BotGuard program). Spike 004 already observed that the two slot-3 payloads share a long common suffix, suggesting a large fixed component plus a small context-dependent prefix. Server-side acceptance of reused tokens is an empirical question, not a given.

**How to avoid:**
- Explicitly test the server acceptance boundary before designing a caching strategy:
  - Does a reused slot-3 from an earlier session work?
  - Does a text-chat slot-3 work for an image request?
  - How long does a token remain valid?
- Treat tokens as single-use-per-request until proven otherwise.
- If caching is added later, gate it behind a feature flag with live-cookie tests that detect rejection.

**Warning signs:**
- `upload_image_works` passes once and then fails on the next run with the same credentials.
- Slot-3 token length or prefix is identical across different request types.
- Server returns `af.httprm` or `er`/`di` WIZ errors after token reuse.

**Phase to address:**
v0.5 Browserless WAA Reverse — server acceptance experiments before integration.

---

### Pitfall 5: Hard-coding BotGuard JS hashes, API keys, or magic constants

**What goes wrong:**
The implementation hard-codes the current `//www.google.com/js/bg/<hash>.js` URL, the WAA API key, or an ARX cipher constant extracted from one version of the VM. A few days later the SDK fails for all users because Google rotated the script or changed the constant.

**Why it happens:**
Analysis reports note that BotGuard JS hashes and cryptographic constants rotate regularly. The current SDK already centralizes protocol literals in `src/constants.rs` with a regression gate, but WAA is especially volatile because the JS is delivered dynamically.

**How to avoid:**
- Fetch the BotGuard interpreter URL and program from the live `Waa/Create` response for each session, rather than embedding them.
- Treat any extracted constant as per-version metadata, not a compile-time constant.
- Add a runtime "script hash changed" fallback that degrades to the existing CDP attestation path while logging the new hash for analysis.

**Warning signs:**
- Tests pass locally using an old `botguard.js` artifact but fail against production.
- CI breaks after a weekend even though no code changed.
- New `botguard.js` fetches produce different decoded bytecode structure.

**Phase to address:**
v0.5 Browserless WAA Reverse — dynamic challenge fetching and integration.

---

### Pitfall 6: Removing the CDP attestation path before proving browserless parity

**What goes wrong:**
Once a browserless generator compiles, the `browser-attestation` feature or `BrowserAttestationClient` is deleted or deprecated. Shortly after, Google changes the BotGuard program and the SDK has no working attestation path at all, breaking image uploads for every user.

**Why it happens:**
Browserless WAA is inherently fragile because it depends on undocumented, rotating client-side code. The existing CDP path, while heavy, is the only proven fallback. The project already feature-gates attestation to keep the core SDK lightweight; that gating should be preserved, not collapsed.

**How to avoid:**
- Keep `browser-attestation` as an opt-in fallback and add a new optional `browserless-waa` feature.
- Implement an automatic fallback chain: try browserless → on failure, log and optionally use CDP if enabled → otherwise fail with `Error::AttestationFailed`.
- Do not remove the CDP module until browserless parity has been stable against live cookies for a full release cycle.

**Warning signs:**
- `Cargo.toml` loses the `browser-attestation` feature during the refactor.
- Integration tests only exercise the new browserless path.
- No test exists that verifies CDP still compiles and runs.

**Phase to address:**
v0.5 Browserless WAA Reverse — integration and feature-gating sub-phase.

---

### Pitfall 7: Failing to surface legal and ToS risk to users

**What goes wrong:**
The SDK ships a browserless WAA generator without prominent documentation that reverse-engineering BotGuard and sending forged/attestation tokens may violate Google's Terms of Service. Users get accounts banned or face DMCA/GitHub takedown actions, and the project maintainers are exposed to legal risk.

**Why it happens:**
Reverse-engineering client-side anti-abuse systems sits in a contested legal zone. Google's ToS prohibit interfering with their services, and while security research has protections in some jurisdictions, distributing a library that generates tokens to access an undocumented web API is not clearly covered. Existing projects in this space (e.g., BgUtils, tomkabel's research) include explicit disclaimers.

**How to avoid:**
- Add a clear `LEGAL.md` or section in `README.md` stating:
  - The project is not affiliated with Google.
  - Browserless WAA is a best-effort reverse-engineering feature.
  - Use may violate Google's ToS and could result in account suspension.
  - Users assume responsibility for compliance with local law.
- Gate the `browserless-waa` feature behind an explicit opt-in in documentation.
- Do not ship pre-generated tokens, hard-coded keys, or tools whose sole purpose is evasion.

**Warning signs:**
- GitHub issues appear reporting Google account bans after using the SDK.
- The crate receives a takedown notice or security advisory.
- `README.md` describes the feature as "official" or "supported by Google."

**Phase to address:**
v0.5 Browserless WAA Reverse — documentation and release-readiness phase.

---

### Pitfall 8: Porting to Rust before proving the transform in a scriptable harness

**What goes wrong:**
Developers start writing a Rust WAA module while still unsure whether the transform is deterministic. They burn weeks on Rust borrow-checking, async glue, and base64url decoding, only to discover that the algorithm requires a full browser environment and cannot be browserless.

**Why it happens:**
Rust is the project's production language, so it is tempting to build there immediately. But the bottleneck is understanding BotGuard, not language choice. A dynamic, scriptable harness (Python/Node.js/Browser DevTools) allows orders-of-magnitude faster iteration than a compiled Rust crate.

**How to avoid:**
- Define a hard "spike exit criterion": the browserless path must generate a token that passes a live-cookie request in a Node.js/Browser harness before any Rust port begins.
- Use the Rust phase only for productionizing a *proven* algorithm.
- If the spike cannot meet the criterion, close it with a documented fallback to CDP.

**Warning signs:**
- New `src/waa.rs` exists before a single reproducible challenge→token pair is documented.
- Live-cookie tests are written in Rust before the underlying generator is known to work.
- Meetings discuss "Rust performance" before "does the VM even run outside a browser?"

**Phase to address:**
v0.5 Browserless WAA Reverse — spike phase, before implementation phase.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Embed a single captured slot-3 token as a fallback | Unblocks tests without running a browser | Breaks on any Google-side rotation; hides real failures | Never in production code; acceptable only for a one-off fixture test |
| Skip the VM and replay the last observed token | Simplifies implementation | Server-side correlation will reject stale tokens; users see transient 400s | Only during early data collection, never shipped |
| Use a Node.js subprocess instead of Rust for generation | Faster to iterate; leverages `bgutils-js`-like code | Adds runtime dependency on Node; breaks the "pure Rust" value proposition | As a spike harness or behind a clearly labeled experimental feature |
| Cache WAA challenge responses indefinitely | Reduces challenge RPCs | Stale bytecode will generate rejected tokens once the JS hash rotates | Never; cache TTL must be minutes at most |
| Silence WAA failures and fall back to no attestation | Image uploads may still work in permissive sessions | Multi-turn state and upload reliability regress silently; support burden rises | Never; failures must be observable and retryable |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `GeminiClient` warm-up | Calling the WAA generator only when `browser-attestation` is enabled, so the new `browserless-waa` feature is never exercised | Add a unified `AttestationProvider` trait; both CDP and browserless implement it; warm-up selects based on features |
| `reqwest` cookie jar | Reusing the same HTTP client for WAA challenge RPCs and normal chat without isolating headers | Use a dedicated WAA request builder with the exact `Content-Type`, `x-goog-api-key`, and protobuf body shapes observed in the browser |
| `StreamGenerate` slot builder | Injecting the browserless token into the wrong slot index or wrapping it in the wrong JSON type | Keep slot construction in `src/proto/slots.rs`; add a regression test that asserts slot-3 is a `!`-prefixed base64url string |
| Error enum (`src/errors.rs`) | Reusing the generic `Attestation(String)` variant for browserless failures, losing structured diagnostics | Introduce `Error::AttestationFailed { reason }` for both paths and include a machine-readable reason (e.g., `vm_timeout`, `token_rejected`, `challenge_fetch_failed`) |
| Feature flags | Making `browserless-waa` imply or replace `browser-attestation` | Keep features orthogonal; document the fallback order and compile-time errors for mutually exclusive misuse |
| Session persistence | Serializing the WAA token as part of `session::Snapshot` | Do not persist tokens; regenerate on warm-up; persist only cookies and conversation state |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Running the BotGuard VM on every request | High CPU/latency, especially if VM is emulated in JS/WASM | Cache the minter/response per session and mint per request; regenerate only when TTL expires or token is rejected | Breaks at moderate request volume if uncached |
| Synchronous VM execution inside an async Rust task | Tokio thread blocking, latency spikes in unrelated requests | Run the VM in a `spawn_blocking` or dedicated worker; keep the async boundary thin | Breaks as soon as concurrent chat requests exist |
| Fetching BotGuard JS for every chat turn | Repeated ~66 KB downloads, slower warm-up, higher Google-side request volume | Fetch once per session warm-up; reuse the interpreter hash unless the server rejects the token | Breaks at first multi-turn conversation |
| Storing full WAA challenge bytecode in memory indefinitely | Memory bloat in long-lived `GeminiClient` instances | Drop bytecode after VM initialization; keep only the program handle/minter | Breaks in daemon/CLI tools with many sessions |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Logging WAA tokens, challenge payloads, or BotGuard bytecode in `tracing` output | Tokens are sensitive proof-of-origin material; logs may leak capabilities | Treat WAA artifacts as secrets; redact in HAR capture and tracing; add a regression test for redaction |
| Shipping extracted Google API keys in source code | Keys rotate and may be revoked; exposes project to ToS action | Load keys from the WAA challenge response or environment; never hard-code |
| Running the browserless generator against a shared Google account in CI | Account bans, rate-limiting, pollutes live metrics | Use fixture tests by default; live-cookie tests marked `#[ignore]` as the project already does |
| Accepting arbitrary BotGuard bytecode without sandboxing | Malicious challenge could exploit the VM interpreter or JS engine | If executing JS, use a hardened, isolated process; if pure Rust, validate bytecode bounds before execution |
| Exposing WAA internals in public API | Consumers may misuse tokens or build bypass tools | Keep WAA types crate-private or behind an unstable feature; document that the API is reverse-engineered |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Browserless path fails silently and image upload "just doesn't work" | Developers cannot tell whether the problem is auth, attestation, or a Google change | Provide a dedicated error variant and a doc page explaining when to enable CDP fallback |
| Feature name implies official Google support (e.g., `official-waa`) | Users assume stability and file issues for Google-side changes | Use names like `browserless-waa` or `experimental-attestation`; mark module docs as reverse-engineered |
| MSRV or dependency explosion from JS/WASM runtime | Users on older Rust or constrained environments cannot build | Keep the feature optional; document MSRV impact and any new heavy dependencies |
| No clear path to opt out | Users who hit bans or failures cannot easily disable the new path | Provide `GeminiClientBuilder` toggle; default to CDP if both features are enabled and browserless fails |

## "Looks Done But Isn't" Checklist

- [ ] **Challenge parsing:** Can parse the current `Waa/Create` response shape and extract interpreter URL, hash, global name, and program bytes.
- [ ] **Determinism proof:** Has reproduced at least 5 challenge→slot-3 pairs from fresh sessions and confirmed server acceptance.
- [ ] **Anti-debug awareness:** Has validated that instrumentation does not alter token output.
- [ ] **Rotation handling:** Has observed and recovered from at least one BotGuard JS hash rotation.
- [ ] **Fallback preserved:** CDP attestation still compiles and passes tests when enabled.
- [ ] **Error propagation:** Browserless failures surface as structured `Error::AttestationFailed`, not opaque 400s.
- [ ] **Legal disclosure:** README or `LEGAL.md` contains a clear reverse-engineering/ToS disclaimer.
- [ ] **Redaction:** WAA tokens and challenges are absent from logs, HAR files, and error messages.
- [ ] **Live parity:** `upload_image_works` passes without the `browser-attestation` feature on live cookies.
- [ ] **No hard-coded secrets:** No Google API keys, script hashes, or constants are committed to source control.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Rotated BotGuard JS breaks browserless generator | LOW if fallback exists; HIGH if CDP was removed | Re-enable CDP feature immediately; capture new challenge/JS artifacts; update parser only if structure changed |
| Instrumentation corrupted the understood algorithm | MEDIUM | Discard affected artifacts; reproduce from a clean browser capture; re-run differential analysis on new pairs |
| Server rejects reused/cached tokens | LOW | Switch to per-request generation; reduce cache TTL; add live test for reuse acceptance |
| Account ban during live testing | LOW-MEDIUM | Use a dedicated test account; rotate credentials; ensure tests are `#[ignore]` by default |
| Legal/ToS takedown | HIGH | Remove disputed implementation; rely solely on CDP attestation path; review with counsel before re-adding |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Static-algorithm assumption | v0.5 spike — data analysis | Multiple fresh challenge→token pairs reproduced in harness |
| Insufficient browser signal fidelity | v0.5 spike — harness design | Harness token matches real-browser token and is accepted server-side |
| Anti-debug/logger blind spots | v0.5 spike — instrumentation | Token output identical with instrumentation on/off; differential analysis clean |
| Token reuse assumptions | v0.5 spike — server acceptance experiments | Live tests confirm or deny cross-session and cross-request-type reuse |
| Hard-coded hashes/constants | v0.5 implementation — dynamic fetch | New `botguard.js` hash is fetched and handled without code change |
| CDP removal | v0.5 implementation — feature design | `browser-attestation` feature still compiles and passes tests |
| Legal/ToS risk | v0.5 documentation | `LEGAL.md`/README reviewed; feature clearly labeled experimental |
| Premature Rust port | v0.5 spike — exit criteria | Harness generates accepted token before `src/waa.rs` is created |
| Silent failures / poor errors | v0.5 implementation — error design | Unit tests assert `Error::AttestationFailed` variants for each failure mode |
| Performance/blocking issues | v0.5 implementation — async boundaries | Benchmark or load test shows no Tokio thread blocking |

## Sources

- Project spike 004 artifacts and README — primary context on Gemini-specific WAA challenge shape and slot-3 behavior.
- tomkabel/google-botguard-security-research — BotGuard VM architecture, anti-debug, anti-logger, and "puppet" bypass strategy. *Confidence: LOW (third-party reverse engineering).*
- dsekz/botguard-reverse — detailed VM opcode analysis, memory reader, self-modifying bytecode. *Confidence: LOW (third-party reverse engineering).*
- LuanRT/BgUtils and `bgutils-js` on npm — documented WAA `Create`/`GenerateIT` flow and PO-token minting; shows the broader Google WAA API pattern. *Confidence: LOW-MEDIUM (widely used open-source implementation).*
- think.resoneo.com BotGuard v41 analysis — fingerprinting scope, ARX cipher with rotating constants, and detection techniques. *Confidence: LOW (third-party analysis).*
- Google Terms of Service and general reverse-engineering law references — legal risk is inferential, not authoritative. *Confidence: LOW (not legal advice).*

---
*Pitfalls research for: browserless WAA / BotGuard reverse engineering in the Gemini Rust SDK*
*Researched: 2026-08-12*
