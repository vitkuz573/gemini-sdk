# Phase 2: Reliability & Protocol Hardening - Research

**Researched:** 2026-08-10
**Domain:** Rust async SDK reliability, Tokio sync primitives, WIZ protocol constants, streaming parsers
**Confidence:** HIGH

## Summary

Phase 2 hardens the Gemini SDK against known fragility called out in `.planning/codebase/CONCERNS.md` and the `02-CONTEXT.md` decisions. The work is mostly internal: replace a `StdMutex<ClientConfig>` with `tokio::sync::RwLock<ClientConfig>` and make the synchronous builder methods async (REL-02); surface WAA/ogads failures as a typed `Error::AttestationFailed` instead of falling back to synthetic context (REL-03); persist consent cookies back into the shared client cookie state (AUTH-04); centralize the magic WIZ slot indices used in `src/proto/slots.rs` and `src/proto/parser.rs` into a dedicated `src/proto/indices.rs` module (PROTO-01); expand parser tests with fixtures for every documented response shape (PROTO-02); remove remaining `.unwrap()`/`.expect()` paths in protocol code and convert them to structured `Error::Parse` (PROTO-04); add a typed `futures::Stream` adapter over the raw `reqwest::Response` byte stream (CHAT-02); and expose system instructions plus generation-config defaults through `ChatBuilder` and the client (CHAT-04).

All changes stay within the existing Rust stack (Tokio, reqwest, serde_json, futures, async-stream) and do not introduce new external dependencies. No package legitimacy checks are required. The phase is a refinement pass over code that already compiles and passes tests, so the primary risk is behavioral regression in the WIZ request/response path and breaking the public builder API by making `with_language`/`with_max_retries`/`with_timeout` async.

**Primary recommendation:** Implement the changes in three waves — (1) config lock + cookie merge + attestation error surfacing, (2) protocol constants + parser hardening + tests, (3) streaming adapter + generation config/system instructions — so each wave is independently verifiable with `cargo test` and `cargo clippy`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Replace `StdMutex<ClientConfig>` with `tokio::sync::RwLock<ClientConfig>` and convert synchronous `with_language` / `with_max_retries` / `with_timeout` builder methods on `GeminiClient` into `async` methods that await `write().await`. Acceptable breaking change for a pre-1.0 async SDK.
- Surface WAA/ogads failures as a typed public error: add `Error::AttestationFailed { reason: String }`, make `run_waa_init_chain` propagate the error; callers can proceed without attestation if they handle it, but the SDK will not hide the failure.
- Create `src/proto/indices.rs` with named constants for every magic index used when building and parsing the 97-slot request list and response arrays; update `src/proto/slots.rs` and `src/proto/parser.rs` to import and use them.
- Add fixture files and unit tests for every documented response shape (simple text, thinking, concatenated text chunks, first-turn meta entry, continuation-token entry with keys "26" and "21", error-code wrapper).
- Add `GeminiClient::generate_stream` returning `Pin<Box<dyn Stream<Item = Result<ChatResponse>> + Send>>`, consuming the raw `reqwest::Response` stream line by line and yielding incremental `ChatResponse` deltas. Keep raw `stream_generate`/`stream_generate_raw` available.
- Extend `GenerationConfig` with `system_instruction: Option<String>`, wire it into slot 0 / request payload, expose `ChatBuilder::with_system_instruction` and a client-level default via async config builder.
- Audit `accept_consent_and_refresh`: merge response cookies into `self.inner.cookies` before refetching `/app` and verify the merged clone is persisted.
- Convert remaining `.unwrap()`/`.expect()` paths in parser and slot builders into `Error::Parse` with descriptive messages.

### the agent's Discretion
- Exact method names, visibility, and helper placement may be adjusted to match existing conventions.
- Whether to expose attestation status as a public getter or keep it internal.

### Deferred Ideas (OUT OF SCOPE)
- Configurable proxy / custom HTTP client (REL-04) → Phase 3.
- Upload progress callbacks (MEDIA-02) → Phase 3.
- Audio/video uploads (MEDIA-03) → Phase 4.
- Tools / function calling (ADV-01) → Phase 5.
- Session persistence (ADV-02) → Phase 4.
- Auto cookie refresh (ADV-03) → Phase 5.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| AUTH-04 | Consent flow persists merged cookies back into the client state. | Persist merged `Cookies` clone into `self.inner.cookies` in `accept_consent_and_refresh`; add integration test. |
| CHAT-02 | Streaming chat yields `ChatResponse` chunks via `futures::Stream`. | Implement `generate_stream` using `futures::Stream`/`async-stream`; parse each WIZ frame incrementally. |
| CHAT-04 | System instructions and generation config can be set per chat or per client. | Add `system_instruction` to `GenerationConfig`, wire into slot 0, expose builder and async client default. |
| PROTO-01 | WIZ slot indices are centralized as named constants in one module. | Create `src/proto/indices.rs` and replace all magic numbers in slots/parser. |
| PROTO-02 | Parser tests cover every documented response shape with fixture files. | Add fixtures and unit tests for simple text, thinking, concatenated chunks, first-turn meta, continuation token key 26/21, error-code wrapper. |
| PROTO-04 | Missing or unexpected protocol fields produce structured errors instead of panics. | Replace remaining `.unwrap()`/`.expect()` in slots/parser with `Error::Parse`. |
| REL-02 | Non-async builder methods do not call `blocking_lock` inside a Tokio runtime. | Switch `config` to `tokio::sync::RwLock`; make `with_*` methods async. |
| REL-03 | WAA / ogads attestation failures surface a typed error instead of silently falling back. | Add `Error::AttestationFailed`; propagate from `run_waa_init_chain`; remove synthetic fallback. |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Config concurrency | Public API Layer (`src/client.rs`) | Session & Auth Layer | `GeminiClient` owns `ClientConfig`; async builders are public API. |
| Attestation error handling | Session & Auth Layer (`src/client.rs`, `src/errors.rs`) | Public API Layer | WAA chain is internal; error type is public. |
| Cookie state persistence | Session & Auth Layer (`src/client.rs`, `src/auth.rs`) | Public API Layer | `Cookies` merge is internal; outcome affects auth. |
| Protocol constants | Protocol Layer (`src/proto/indices.rs`, `src/proto/slots.rs`) | — | Constants encode undocumented WIZ layout. |
| Response parsing | Protocol Layer (`src/proto/parser.rs`) | — | Parses upstream WIZ frames into SDK types. |
| Streaming adapter | Public API Layer (`src/client.rs`) | Protocol Layer (`src/proto/parser.rs`) | `generate_stream` is public; parsing logic reused. |
| Generation config / system instructions | Public API Layer (`src/chat.rs`, `src/client.rs`) | Protocol Layer (`src/proto/slots.rs`) | User-facing builder options map to slot 0. |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tokio` | 1.40 (`full`) | Async runtime and `RwLock` | De facto standard; already locked by project. [VERIFIED: cargo metadata] |
| `reqwest` | 0.12 (`cookies`, `json`, `multipart`, `stream`) | HTTP client + raw byte stream | Already used; streaming support required. [VERIFIED: cargo metadata] |
| `serde` / `serde_json` | 1.0 | WIZ protocol (de)serialization | Standard Rust serialization stack. [VERIFIED: cargo metadata] |
| `futures` | 0.3 | `Stream` trait and adapters | Already a dependency; needed for `generate_stream` return type. [VERIFIED: cargo metadata] |
| `async-stream` | 0.3 | Ergonomic async stream generation | Already a dependency; simplifies `generate_stream` implementation. [VERIFIED: cargo metadata] |
| `thiserror` | 1.0 | Derive-based `Error` enum | Already used for `Error`. [VERIFIED: cargo metadata] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `base64` | 0.22 | Inline image encoding / WAA token encoding | Already used. [VERIFIED: cargo metadata] |
| `uuid` | 1.11 (`v4`) | Request UUID generation | Already used. [VERIFIED: cargo metadata] |

## Package Legitimacy Audit

No new external packages are required for this phase. All libraries are already declared in `Cargo.toml` and verified via `cargo metadata`.

## Architecture Patterns

### Pattern 1: Async Builder Methods with Shared Async Lock
**What:** Public `GeminiClient` builder methods become `async fn` and mutate shared state through `tokio::sync::RwLock::write().await`.
**When to use:** When configuration is shared across clones and callers are already in an async context.
**Example:**
```rust
pub async fn with_language(self, language: impl Into<String>) -> Self {
    let language = language.into();
    {
        let mut config = self.inner.config.write().await;
        config.language.clone_from(&language);
    }
    self
}
```

### Pattern 2: Typed Errors Instead of Silent Fallbacks
**What:** Internal failures that were previously logged and replaced with a default are propagated as structured `Error` variants.
**When to use:** When hiding a failure would make debugging impossible or violate the user's expectation that the SDK reports problems.
**Example:**
```rust
let waa_context = self
    .ogads_get_async_data(&cookie_header, &credentials, &waa_token)
    .await
    .map_err(|e| Error::AttestationFailed { reason: e.to_string() })?;
```

### Pattern 3: Centralized Protocol Constants
**What:** All magic indices for the 97-slot WIZ array live in `src/proto/indices.rs`, grouped by builder/parser usage and documented.
**When to use:** Any time the same undocumented protocol index appears in more than one file or is used both to build and parse.
**Example:**
```rust
// src/proto/indices.rs
pub const SLOT_PROMPT: usize = 0;
pub const SLOT_LANGUAGE: usize = 1;
pub const SLOT_WAA_TOKEN: usize = 3;
pub const SLOT_PART_TEXT: usize = 1;
pub const SLOT_PART_THINKING: usize = 37;
```

### Pattern 4: Incremental Stream Parsing
**What:** A `futures::Stream` adapter reads raw bytes, buffers lines, parses each WIZ frame, and yields `ChatResponse` deltas.
**When to use:** When the caller wants incremental output and the underlying transport is a line-delimited byte stream.
**Example:**
```rust
pub async fn generate_stream(
    &self,
    message: &ChatMessage,
    category: ModelCategory,
    config: Option<GenerationConfig>,
) -> Result<Pin<Box<dyn Stream<Item = Result<ChatResponse>> + Send>>> {
    let response = self.stream_generate_raw(message, None, category, config).await?;
    Ok(Box::pin(try_stream! {
        let mut stream = response.bytes_stream();
        let mut buf = BytesMut::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            // ...parse frames and yield ChatResponse deltas...
        }
    }))
}
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async stream of parsed chunks | Manual `Stream` state machine | `async-stream::try_stream!` | Already in dependency tree; reduces boilerplate and state-machine bugs. |
| Async read/write lock | `std::sync::Mutex` + `blocking_lock` | `tokio::sync::RwLock` | Avoids panics inside Tokio runtime; consistent with existing `cookies`/`session` locks. |
| Response body buffering | Manual chunked string assembly | `bytes::BytesMut` via `reqwest::bytes_stream` | reqwest already exposes byte stream; `BytesMut` is the standard buffer. |

## Runtime State Inventory

Not applicable — this phase modifies source code and tests only; no runtime state, databases, or external registrations are changed.

## Common Pitfalls

### Pitfall 1: Panic on `blocking_lock` Inside Tokio Runtime
**What goes wrong:** Calling `StdMutex::blocking_lock()` from a Tokio task panics when no blocking thread pool is configured.
**Why it happens:** `with_language`/`with_max_retries`/`with_timeout` are synchronous but mutate shared state under a std mutex.
**How to avoid:** Convert to `async fn` and use `tokio::sync::RwLock`.

### Pitfall 2: Dropping Merged Cookies on the Floor
**What goes wrong:** `accept_consent_and_refresh` merges response cookies into a local clone but never writes it back to `self.inner.cookies`.
**Why it happens:** The `Cookies` clone is obtained via `self.cookies().await` and merged, but then a new `self.inner.cookies.lock().await` guard is used to overwrite it.
**How to avoid:** Build the merged `Cookies`, then assign `*guard = cookies` inside a single lock scope before refetching `/app`.

### Pitfall 3: Hidden Attestation Failures
**What goes wrong:** `ogads_get_async_data` falls back to `build_default_waa_context()` on any error, so image uploads and multi-turn state may behave inconsistently without explanation.
**Why it happens:** The SDK wanted to be resilient to optional WAA endpoints.
**How to avoid:** Propagate the failure as `Error::AttestationFailed`; callers can catch it and construct a client without attestation if they choose.

### Pitfall 4: Magic Numbers Drift Out of Sync
**What goes wrong:** Slot indices are duplicated between `slots.rs` (builder) and `parser.rs` (parser); a change in one file silently breaks the other.
**Why it happens:** The undocumented WIZ protocol was reverse-engineered incrementally.
**How to avoid:** Centralize constants in `indices.rs` with clear names and doc comments.

### Pitfall 5: Streaming Adapter Loses Final State
**What goes wrong:** A stream adapter yields incremental chunks but never calls `ingest_conversation_state`, so multi-turn breaks for streaming callers.
**Why it happens:** The current `generate_raw` consumes the whole body and extracts state; `generate_stream` must do the equivalent after the stream ends.
**How to avoid:** After the byte stream ends, parse any remaining buffered data and call `ingest_conversation_state` (or expose a helper for callers to call explicitly and document it).

## Code Examples

### Replacing `StdMutex` with `tokio::sync::RwLock`
```rust
struct Inner {
    http: Client,
    cookies: Mutex<Cookies>,
    session: Mutex<SessionState>,
    config: RwLock<ClientConfig>, // was StdMutex<ClientConfig>
}
```

### Surfacing Attestation Errors
```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    // ... existing variants ...
    /// Browser attestation / WAA acquisition failed.
    #[error("attestation failed: {reason}")]
    AttestationFailed { reason: String },
}
```

### Centralized Slot Constants
```rust
// src/proto/indices.rs
/// Slot that carries the user's prompt and optional attachments.
pub const SLOT_PROMPT: usize = 0;
/// Slot that carries the language code.
pub const SLOT_LANGUAGE: usize = 1;
/// Slot that carries multi-turn conversation state.
pub const SLOT_CONVERSATION_STATE: usize = 2;
/// Slot that carries the WAA token.
pub const SLOT_WAA_TOKEN: usize = 3;
/// Slot that carries the request nonce.
pub const SLOT_NONCE: usize = 4;
```

### Stream Adapter Skeleton
```rust
use async_stream::try_stream;
use futures::Stream;
use bytes::BytesMut;

pub async fn generate_stream(
    &self,
    message: &ChatMessage,
    category: ModelCategory,
    config: Option<GenerationConfig>,
) -> Result<Pin<Box<dyn Stream<Item = Result<ChatResponse>> + Send>>> {
    let response = self.stream_generate_raw(message, None, category, config).await?;
    Ok(Box::pin(try_stream! {
        let mut stream = response.bytes_stream();
        let mut buf = BytesMut::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(Error::Request)?;
            buf.extend_from_slice(&chunk);
            // Parse complete lines/frames and yield ChatResponse deltas.
        }
    }))
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `StdMutex` + `blocking_lock` in sync builders | `tokio::sync::RwLock` + async builders | Phase 2 | Removes Tokio panic risk; aligns with async conventions. |
| Silent synthetic WAA context fallback | `Error::AttestationFailed` propagation | Phase 2 | Callers can detect and handle attestation failures. |
| Inline magic WIZ indices | Named constants in `src/proto/indices.rs` | Phase 2 | Easier to keep builder and parser synchronized. |
| Raw `reqwest::Response` only | Optional typed `futures::Stream` adapter | Phase 2 | Easier incremental consumption for callers. |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `futures` and `async-stream` already in `Cargo.toml` are sufficient for the stream adapter. | Standard Stack | Low — both crates already provide `Stream` and `try_stream!`. |
| A2 | Making `with_language`/`with_max_retries`/`with_timeout` async is acceptable for a pre-1.0 crate. | User Constraints | Low — explicitly approved in CONTEXT.md. |
| A3 | The 97-slot array shape remains stable enough that centralizing indices does not require changing the shape itself. | Protocol Constants | Medium — Google can change the protocol, but the refactor is purely naming; behavior is unchanged. |

## Open Questions (RESOLVED)

1. **Should attestation failure abort session init or only be reported?**
   - RESOLVED: Propagate as `Error::AttestationFailed`; callers can proceed without attestation if they choose.
2. **Where should system instruction be encoded in the 97-slot list?**
   - RESOLVED: Append as a preamble to the prompt text in slot 0 for v0.1 compatibility (per CONTEXT.md Specific Ideas).
3. **Should `generate_stream` automatically ingest conversation state?**
   - RESOLVED: After the stream ends, parse buffered data and call `ingest_conversation_state`; document that callers must await the stream to completion for state persistence.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All tasks | ✓ | 1.95.0 | — |
| Cargo | Build/test | ✓ | 1.95.0 | — |
| Existing crates (`tokio`, `reqwest`, `futures`, `async-stream`) | All tasks | ✓ | See Cargo.lock | — |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Cargo built-in tests + `tokio-test` for async helpers |
| Config file | `Cargo.toml` |
| Quick run command | `cargo test --lib --quiet` |
| Full suite command | `cargo test --all-targets --quiet && cargo clippy --all-targets -- -D warnings && cargo doc --no-deps` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| AUTH-04 | Consent cookie merge persisted | unit/integration | `cargo test --test integration_tests consent_cookie_merge` | Wave 0 |
| CHAT-02 | `generate_stream` yields `ChatResponse` deltas | integration | `cargo test --test integration_tests stream` | Wave 0 |
| CHAT-04 | System instruction reaches slot 0 | unit | `cargo test --test proto_tests system_instruction_in_slot0` | Wave 0 |
| PROTO-01 | No raw magic numbers in slots/parser | static | `grep` / `cargo clippy` (no new test) | — |
| PROTO-02 | Parser handles all documented shapes | unit/integration | `cargo test --test proto_tests` | fixtures exist |
| PROTO-04 | No `.unwrap()`/`.expect()` in parser/slots | static | `grep` / `cargo clippy` | — |
| REL-02 | Async builders don't panic in Tokio runtime | unit | `cargo test --test integration_tests config_builder_async` | Wave 0 |
| REL-03 | WAA/ogads failure returns typed error | unit | `cargo test --lib client_tests attestation_failed` | Wave 0 |

### Wave 0 Gaps
- `tests/integration_tests.rs` needs new test functions for consent cookie merge, streaming adapter, async config builder, and system instruction wiring.
- `tests/proto_tests.rs` needs new fixtures and tests for first-turn meta, continuation token key 21, and error-code wrapper.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Cookie merge persistence must not overwrite valid credentials with empty values; validate trusted consent origins. |
| V3 Session Management | yes | Session tokens and WAA context must not leak in error messages. |
| V5 Input Validation | yes | Parser must reject malformed WIZ frames without panicking (`Error::Parse`). |
| V6 Cryptography | no | No new crypto introduced. |

### Known Threat Patterns for Rust/Tokio/Reqwest

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Panic on malformed protocol data | Denial of Service | Convert `unwrap`/`expect` to structured errors. |
| Cookie leakage in error/debug output | Information Disclosure | Redact cookie values in `Error` strings and `Debug` impls. |
| Blocking lock inside async runtime | Denial of Service | Use `tokio::sync::RwLock` and async methods. |

## Sources

### Primary (HIGH confidence)
- `cargo metadata` — dependency versions.
- `src/client.rs`, `src/proto/slots.rs`, `src/proto/parser.rs`, `src/errors.rs`, `src/chat.rs` — current implementation.
- `.planning/codebase/CONCERNS.md` — identified fragility.
- `.planning/phases/02-reliability-protocol-hardening/02-CONTEXT.md` — locked decisions.

### Secondary (MEDIUM confidence)
- `tests/fixtures/*.json`, `tests/proto_tests.rs` — existing parser coverage.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — verified against `Cargo.toml` and `cargo metadata`.
- Architecture: HIGH — based on direct source inspection.
- Pitfalls: HIGH — directly derived from current code and CONCERNS.md.

**Research date:** 2026-08-10
**Valid until:** 2026-09-10 (30 days for stable Rust stack)
