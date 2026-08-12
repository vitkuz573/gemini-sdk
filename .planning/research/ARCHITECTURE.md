# Architecture Research: Browserless WAA Integration

**Domain:** Rust SDK for Google Gemini web frontend attestation (WAA/BotGuard)
**Researched:** 2026-08-12
**Confidence:** MEDIUM — based on current SDK code, spike 004 artifacts, and spike findings; pending reverse-engineering of the slot-3 transform.

## Standard Architecture

### System Overview

The SDK is organized as a layered async Rust client. The browserless WAA feature is a new **attestation provider** that sits beside the existing CDP-based attestation path.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Public API (client.rs)                           │
│  GeminiClient  ──►  ChatBuilder / generate / upload / config RPCs       │
├─────────────────────────────────────────────────────────────────────────┤
│                       Session & Attestation Layer                        │
│  ┌─────────────────────┐  ┌─────────────────────┐  ┌───────────────┐   │
│  │   session.rs        │  │  attestation.rs     │  │  waa/ (new)   │   │
│  │  state, extractors  │  │  CDP capture (opt)  │  │  browserless  │   │
│  └─────────────────────┘  └─────────────────────┘  └───────────────┘   │
├─────────────────────────────────────────────────────────────────────────┤
│                       Protocol Builders (proto/)                         │
│  slots.rs  /  mod.rs  /  indices.rs  ──►  97-slot StreamGenerate body   │
├─────────────────────────────────────────────────────────────────────────┤
│                       Constants & Transport Layer                        │
│  constants.rs  /  auth.rs  /  retry.rs  /  har.rs                        │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Current Implementation |
|-----------|----------------|------------------------|
| `GeminiClient` | Owns HTTP client, credentials, session state; routes attestation to chosen provider. | `Arc<Inner>` with `Mutex<SessionState>`, config in `RwLock`. |
| `SessionState` | Stores WAA token (`waa_token`), WAA context header (`waa_context`), fingerprint, nonce, conversation state. | `src/session.rs`. |
| CDP attestation | Captures a real browser StreamGenerate payload via Chrome DevTools Protocol when `browser-attestation` feature is enabled. | `src/attestation.rs` (`BrowserAttestationClient`). |
| **Browserless WAA (new)** | Generates the slot-3 WAA token from the `Waa/Create` challenge without a browser. | New module `src/waa/`; provider trait / enum. |
| Protocol builders | Build the 97-slot `StreamGenerate` request; slot 3 receives the WAA token. | `src/proto/slots.rs`, `src/proto/mod.rs`. |
| Constants | Centralizes WAA/OGADS API keys, endpoints, CDP constants, header templates. | `src/constants.rs`. |

## Recommended Project Structure

After integration, `src/` should contain a dedicated `waa/` module rather than scattering browserless logic across `client.rs`:

```
src/
├── lib.rs
├── client.rs              # Routes to WaaGenerator; keeps existing API surface
├── session.rs             # Already owns waa_token / waa_context / waa_fingerprint
├── attestation.rs         # CDP-based feature-gated path (unchanged)
├── waa/
│   ├── mod.rs             # Public facade + AttestationProvider enum/trait
│   ├── generator.rs       # Browserless token generator implementation
│   ├── challenge.rs       # Waa/Create response parsing / challenge extraction
│   └── vm.rs              # Minimal BotGuard VM harness / algorithm port
├── proto/
│   ├── mod.rs             # WAA/ogads body builders
│   ├── slots.rs           # Slot 3 injection point
│   └── indices.rs         # SLOT_WAA_TOKEN constant
└── constants.rs           # Add browserless WAA constants
```

### Structure Rationale

- **`waa/`:** Keeps the new attestation domain isolated from chat/RPC code. The CDP path stays in `attestation.rs` to avoid breaking the existing feature gate.
- **`waa/generator.rs`:** Owns the algorithm once the BotGuard transform is reverse-engineered. It is the only file that should depend on any crypto/bytecode primitives.
- **`waa/challenge.rs`:** Parsing the `Waa/Create` response shape is small but separate; it mirrors how `proto/mod.rs` centralizes body builders.
- **`session.rs` remains unchanged structurally:** `waa_token`, `waa_context`, and `waa_fingerprint` already exist; the new path writes to the same fields.

## Architectural Patterns

### Pattern 1: Provider Abstraction for Attestation

**What:** Hide CDP and browserless implementations behind a common attestation provider interface. The client holds `Option<Arc<dyn WaaProvider>>` or an enum that resolves to CDP, Browserless, or None.

**When to use:** When two very different mechanisms (browser automation vs. algorithmic generation) produce the same artifact (slot-3 token).

**Trade-offs:**

- Pros: CDP feature gate is preserved; caller can opt in to browserless; testable with a mock provider.
- Cons: One more trait/async boundary in the warm-up chain; avoid over-generalizing.

**Example:**

```rust
#[async_trait::async_trait]
pub trait WaaProvider: Send + Sync {
    async fn generate_token(
        &self,
        client: &GeminiClient, // or http + cookies + challenge
        challenge: &WaaChallenge,
    ) -> Result<String>;
}

pub enum AttestationStrategy {
    Disabled,
    Browserless(Arc<dyn WaaProvider>),
    #[cfg(feature = "browser-attestation")]
    Cdp(Arc<BrowserAttestationClient>),
}
```

### Pattern 2: State-Driven Token Injection

**What:** `SessionState` owns the WAA token/context; request builders read from it. The browserless path does not change the `StreamGenerate` builder signature.

**When to use:** Slot 3 is already part of session state, and `build_inner_req_list` already accepts `waa_token: Option<&str>`.

**Trade-offs:**

- Pros: Minimal churn in `proto/slots.rs` and `client.rs`; existing fallback logic (send `Value::Null`) works automatically.
- Cons: The token must be generated during `init_session()` warm-up, not lazily per request, because multi-turn state depends on it.

**Example:**

```rust
// client.rs warm-up path
let token = match strategy {
    AttestationStrategy::Browserless(p) => p.generate_token(&challenge).await.ok(),
    #[cfg(feature = "browser-attestation")]
    AttestationStrategy::Cdp(c) => capture_via_cdp(c).await.ok(),
    AttestationStrategy::Disabled => None,
};
session.waa_token = token;
```

### Pattern 3: Feature-Gated Heavy Dependencies

**What:** Keep the CDP/tokio-tungstenite dependency behind the existing `browser-attestation` feature. The browserless path should use only lightweight crates (e.g., `base64`, maybe a small JS interpreter if required).

**When to use:** The core value of the SDK is "no heavy browser dependency."

**Trade-offs:**

- Pros: Default build is fast; users who want browser automation still can.
- Cons: If browserless requires a JS runtime, feature-gating becomes more complex.

## Data Flow

### Request Flow (Browserless Path)

```
GeminiClient::init_session()
    ↓
fetch /app HTML, extract SNlM0e, build_label, f.sid, fingerprint
    ↓
run_waa_init_chain()
    ├── sJBwce prerequisite (batchexecute)
    ├── WAA/Create  ──►  waa_create() returns challenge token
    │                      ↓
    │              WaaProvider::generate_token(challenge)
    │                      ↓
    │              slot-3 token (browserless algorithm)
    ├── ogads GetAsyncData ──► WAA context header (or fallback template)
    └── ESY5D feature flags
    ↓
store token + context in SessionState
    ↓
chat / generate / upload
    ↓
build_inner_req_list(waa_token = session.waa_token)  // slot 3
    ↓
StreamGenerate
```

### Key Data Flows

1. **Challenge → Token:** `Waa/Create` returns a base64url challenge (~31 KB). The browserless provider decodes/parses it and applies the discovered transform to produce the `!`-prefixed base64url slot-3 token (~2 KB).
2. **Token → Slot 3:** `build_inner_req_list` writes `session.waa_token` into `inner[SLOT_WAA_TOKEN]`.
3. **Token → Context Header:** `build_waa_context_header` in `client.rs` merges the token/fingerprint with the ogads response (or synthetic template) to produce `x-goog-ext-525001261-jspb`.

## Scaling Considerations

This is a client SDK; "scale" here means concurrency and token reuse, not users.

| Scale | Architecture Adjustments |
|-------|--------------------------|
| Single-threaded CLI | Default provider per `GeminiClient` is fine. |
| Concurrent clients | Each `GeminiClient` has its own session; WAA generation is per-session. Avoid a global provider with mutable state. |
| High-volume automation | Cache the WAA token only within a session lifetime; Google likely rejects reused tokens across sessions. |

### Scaling Priorities

1. **First bottleneck:** Repeated WAA/Create + ogads calls per request. The chain should run once per session init, not per `StreamGenerate`.
2. **Second bottleneck:** If the browserless generator has non-trivial compute, run it on `tokio::task::spawn_blocking` to avoid blocking the async runtime.

## Anti-Patterns

### Anti-Pattern 1: Reusing the WAA Challenge as Slot 3

**What people do:** Place the raw `Waa/Create` response string directly into `StreamGenerate` slot 3.

**Why it's wrong:** The challenge token is ~31 KB and is not the `!`-prefixed base64url token the server expects. Image uploads fail with attestation errors.

**Do this instead:** Route the challenge through the browserless generator (or CDP capture) and store only the generated slot-3 token in `SessionState.waa_token`.

### Anti-Pattern 2: Making WAA Warm-Up Hard-Failing

**What people do:** Return `Error::AttestationFailed` from `init_session()` when WAA/Create or ogads fails.

**Why it's wrong:** The SDK currently works for text-only generation without valid WAA context. Failing warm-up breaks text-only use cases.

**Do this instead:** Keep the current behavior: WAA/Create failures are `AttestationFailed`, but ogads/batchexecute warm-up failures are tolerated with a synthetic context. The browserless generator failure should also be non-fatal and fall back to `None`.

### Anti-Pattern 3: Coupling Browserless Logic to `client.rs`

**What people do:** Inline VM parsing and byte-munging directly into `run_waa_init_chain()`.

**Why it's wrong:** `client.rs` is already large (~3500 lines); adding crypto/VM code there makes the warm-up path harder to test and review.

**Do this instead:** Implement a `WaaProvider` in `src/waa/` and call it from `run_waa_init_chain` via a single async method.

### Anti-Pattern 4: Dropping the CDP Path

**What people do:** Replace the CDP module entirely with the browserless path.

**Why it's wrong:** If Google changes the BotGuard challenge or enforces stricter checks, browserless generation may stop working. The CDP path is the only known good fallback.

**Do this instead:** Keep `attestation.rs` feature-gated and make CDP a fallback strategy when browserless is unavailable or fails.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| `waa-pa.clients6.google.com` | HTTP POST `Waa/Create` | Already implemented in `client.rs:waa_create()`. Returns the challenge. |
| `ogads-pa.clients6.google.com` | HTTP POST `GetAsyncData` with `SAPISIDHASH` auth | Already implemented; tolerates failure and falls back to synthetic context. |
| `gemini.google.com/_/BardChatUi/data/...` | `StreamGenerate` with slot-3 token and `x-goog-ext-525001261-jspb` header | Existing builder injects token from session state. |
| Chrome CDP (optional) | WebSocket capture of real browser payload | Feature-gated in `attestation.rs`; fallback path. |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `client.rs` ↔ `waa/mod.rs` | Async trait call during warm-up | Provider is configured on `ClientConfig` or via builder method. |
| `waa/mod.rs` ↔ `proto/mod.rs` | Challenge parsing helpers | `build_waa_create_body()` is reused; new challenge parser may live in `waa/challenge.rs`. |
| `session.rs` ↔ `waa/generator.rs` | Read/write `waa_token` | Generator is stateless; session stores the output. |
| `proto/slots.rs` ↔ `session.rs` | `build_inner_req_list` reads `session.waa_token` | No signature change needed. |

## Components That Must Change

### New Components

| File | Purpose |
|------|---------|
| `src/waa/mod.rs` | Public module facade, `WaaProvider` trait/enum, strategy selection. |
| `src/waa/generator.rs` | Browserless slot-3 token generator; depends on the reverse-engineered transform. |
| `src/waa/challenge.rs` | Parses `Waa/Create` response into challenge token and metadata. |
| `src/waa/vm.rs` | Optional: minimal JS/DOM harness if the algorithm cannot be fully ported to Rust. |

### Modified Components

| File | Change |
|------|--------|
| `src/client.rs` | Add `waa_strategy` to `ClientConfig`; wire `WaaProvider` into `run_waa_init_chain`; preserve CDP fallback. |
| `src/session.rs` | No struct changes required; existing `waa_token`/`waa_context`/`waa_fingerprint` fields are reused. |
| `src/proto/mod.rs` | Possibly add challenge-response parser; `build_waa_create_body` already exists. |
| `src/proto/slots.rs` | No changes required; slot 3 injection path already exists. |
| `src/constants.rs` | Add constants for WAA template defaults, BotGuard URL patterns, provider config. |
| `Cargo.toml` | Add optional dependency for the browserless algorithm (e.g., `base64`, `sha1`, or a JS engine if needed). Keep CDP deps behind existing feature. |

## Suggested Build Order

The order below respects dependencies: the algorithm must exist before the client can call it, and the integration must be tested before the CDP fallback is considered safe.

1. **Spike closure / algorithm harness** — confirm the BotGuard transform from spike 004 and capture the missing `(challenge, slot3)` pairs. If infeasible, stop here.
2. **`src/waa/challenge.rs`** — parse the `Waa/Create` response into a typed challenge. Add unit tests with fixture data.
3. **`src/waa/generator.rs`** — implement the browserless token generator with the discovered algorithm. Test against captured pairs.
4. **`src/waa/mod.rs`** — define `WaaProvider` trait/enum and a default browserless provider.
5. **`src/constants.rs` + `Cargo.toml`** — add any new constants or optional dependencies; keep the feature gate clean.
6. **`src/client.rs`** — add `ClientConfig::waa_strategy` and `with_waa_strategy`; update `run_waa_init_chain` to call the provider and store the result in `session.waa_token`.
7. **`src/session.rs` (verify)** — ensure serialization/deserialization handles `waa_token`/`waa_context` correctly (already present).
8. **Fallback wiring** — ensure browserless failure does not fail `init_session`; ensure CDP path is still reachable when the `browser-attestation` feature is enabled.
9. **Fixture tests** — add tests for challenge parsing and token generation using spike 004 data.
10. **Live-cookie integration test** — run `upload_image_works` without the `browser-attestation` feature.
11. **Quality gates** — `cargo test`, `cargo clippy`, `cargo doc`, regression gate for magic strings.

## Sources

- `src/client.rs` — current `run_waa_init_chain`, `waa_create`, `ogads_get_async_data`, `build_waa_context_header`.
- `src/session.rs` — `SessionState` fields and `Snapshot` serialization.
- `src/proto/mod.rs` — WAA/ogads body builders.
- `src/proto/slots.rs` — 97-slot builder and slot 3 injection.
- `src/constants.rs` — centralized WAA/attestation constants.
- `.planning/spikes/004-waa-token/README.md` — spike findings and missing data.
- `.opencode/skills/spike-findings-gemini-sdk/references/waa-attestation.md` — WAA chain and context header template.

---
*Architecture research for: gemini-sdk browserless WAA reverse engineering*
*Researched: 2026-08-12*
