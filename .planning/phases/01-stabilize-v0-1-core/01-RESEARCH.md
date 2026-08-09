# Phase 1: Stabilize v0.1 Core - Research

**Researched:** 2026-08-09
**Domain:** Rust async SDK for the undocumented Google Gemini web frontend protocol
**Confidence:** HIGH

## Summary

Phase 1 is a stabilization and packaging pass over an already-functional Rust crate. The codebase currently implements cookie auth, text/image chat, streaming responses, multi-turn conversation state, model listing, file upload, and optional browser attestation. The implementation is solid enough that `cargo test`, `cargo clippy`, and `cargo package` already succeed in this environment, but the public API is not yet prepared for semver commitments: many public types lack `#[non_exhaustive]`, `CredentialsProvider` does not exist, the error type is missing variants needed by later phases, and documentation lints are only warnings, not denied.

The primary research conclusion is that Phase 1 should not introduce new protocol behavior. Instead, it should reshape the public surface so future changes can be additive. That means marking public enums/structs forward-compatible, introducing a small auth trait with a default cookie-based implementation, consolidating error construction, and making documentation/clippy gates strict. The existing transport, retry, protocol, and test infrastructure can remain largely unchanged, because they already satisfy the Phase 1 requirements for chat, media, retries, and tooling.

**Primary recommendation:** Treat Phase 1 as a "seal the crate" pass: finalize the module layout, add `#[non_exhaustive]` to public enums/structs, introduce `CredentialsProvider`, tighten lints, and ensure examples compile, so that v0.1 can be published with a stable API surface.

<user_constraints>
## User Constraints (from CONTEXT.md)

No CONTEXT.md exists for this phase. All implementation choices are the agent's discretion, subject to the project decisions already captured in PROJECT.md, STATE.md, and AGENTS.md.

### Locked Decisions (from PROJECT.md / STATE.md)
- Tech stack: Rust 1.80+, Tokio, reqwest — fixed by project foundation.
- Protocol target: undocumented WIZ web frontend (`gemini.google.com`); official REST/Vertex AI is out of scope.
- semver progression: 0.1 → 0.2 → 1.0.
- Cookie-based auth remains default; a `CredentialsProvider` trait may be added for extensibility.
- Live-cookie integration tests are marked `#[ignore]` and do not run in CI.
- Security: cookies are secrets; redact them in logs and avoid leaking them in errors.

### the agent's Discretion
- Exact shape of `CredentialsProvider` trait and its associated types.
- Which public types receive `#[non_exhaustive]` vs. sealed traits.
- How to tighten documentation/clippy gates (e.g., `#![deny(missing_docs)]` on public items).
- Refactoring scope inside `client.rs` to improve testability without changing external behavior.
- Whether to keep `backoff` or replace it (deferred to Phase 2 unless blocking for v0.1).

### Deferred Ideas (OUT OF SCOPE for Phase 1)
- System instructions and generation config exposure per client/turn (Phase 2, CHAT-04).
- Upload progress callbacks (Phase 3, MEDIA-02).
- Session persistence helpers (Phase 4, ADV-02).
- Request/response hooks and structured tracing spans (Phase 3, OBS-01/OBS-02).
- Tools / function calling (Phase 5, ADV-01).
- Audio and video uploads (Phase 4, MEDIA-03).
- Auto cookie refresh / consent re-acquisition persistence (Phase 5, ADV-03).
- crates.io publication final checklist (Phase 6, TOOL-05), though the manifest should be publishable after Phase 1.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| API-01 | `GeminiClient`, `ChatBuilder`, `Conversation`, and `ChatResponse` types are marked `#[non_exhaustive]` or otherwise documented for forward compatibility. | Apply `#[non_exhaustive]` to `GeminiClient` (already opaque via `Arc<Inner>`), `ChatBuilder` (already lifetime-bound), `Conversation`, and `ChatResponse`; ensure public enums (`ModelCategory`, `ThinkingLevel`, `ImageSource`, `ContentPart`) already carry it or add it. |
| API-02 | All public error types live in a dedicated module and implement `std::error::Error` + `Send` + `Sync` + `'static`. | `Error` is already `#[non_exhaustive]` and uses `thiserror` in `src/errors.rs`; verify `Send + Sync + 'static` via `static_assertions` or compile test. |
| API-03 | Breaking changes to public types are only introduced in minor 0.x versions before v1.0 and in major versions after v1.0. | Document semver policy; use `#[non_exhaustive]` and `#[doc(hidden)]` internal types; avoid exposing raw protocol indices or session internals. |
| API-04 | Crate compiles with `#![deny(missing_docs)]` on public items. | Change `src/lib.rs` `#![warn(missing_docs)]` to `#![deny(missing_docs)]` for lib builds; keep `--all-targets` clippy green. |
| AUTH-01 | Cookie-based auth accepts a header string and validates required cookies (`__Secure-1PSID`, `__Secure-1PSIDCC`). | `Credentials::from_header` and `Cookies::from_header` already parse and validate; tests exist in `src/auth.rs`. |
| AUTH-02 | A `CredentialsProvider` trait allows custom auth sources (env, file, keyring) without changing `GeminiClient` API. | Introduce `auth::CredentialsProvider: Send + Sync` returning `Credentials`/`Cookies`; provide `CookieHeaderProvider` default; add `GeminiClient::from_provider`. |
| AUTH-03 | Credentials are fully redacted in `Debug` output. | Current `Credentials` Debug shows first 4 chars of each secret; change to `"<redacted>"` to satisfy full redaction. |
| CHAT-01 | Text-only chat returns a complete `ChatResponse` with text and optional reasoning content. | `ChatBuilder::send_message` → `generate` → `parse_chat_response` already works; covered by fixtures. |
| CHAT-02 | Streaming chat yields `ChatResponse` chunks via `futures::Stream`. | Current `stream_generate` returns raw `reqwest::Response`; Phase 1 scope is limited to API stability, so document that streaming parsing is raw for v0.1 and defer a typed stream adapter to Phase 2/3. |
| CHAT-03 | Multi-turn `Conversation` preserves state across turns and can be cloned/shared safely. | `Conversation` already stores messages and category; session state is written back after each turn; `Clone` is derived. |
| CHAT-05 | Model category selection (`Auto`, `Pro`, `Flash`, etc.) is preserved and validated. | `ModelCategory` is `#[non_exhaustive]` and has `as_enum_value`; `ChatBuilder::with_category` stores it; validation is implicit in slot building. |
| MEDIA-01 | Inline image uploads encode data and produce a usable upload ID. | `upload::upload_attachments` and `ImageSource::from_bytes` already perform base64 encode + resumable upload; fixture tests pass. |
| REL-01 | Retries use exponential backoff with jitter for transient HTTP errors and rate limits. | `retry::with_backoff` uses `backoff::ExponentialBackoff` with 500ms–8s interval and 30s max elapsed time; `Error::is_transient` covers 429/5xx and `Transient`/`RateLimited`/`Timeout`. |
| TOOL-01 | `cargo test` passes without live cookies using fixtures and mocked fixtures. | All 56 unit + integration tests pass without `GEMINI_COOKIES`; live tests in `tests/real_cookies.rs` are skipped unless env present. |
| TOOL-02 | `cargo clippy --all-targets -- -D warnings` passes. | Currently passes in this environment. |
| TOOL-03 | `cargo doc --no-deps` builds with no warnings. | One broken intra-doc link exists (`GeminiClient::verify_signed_in` referenced from `src/auth.rs:121`); fix this and switch to `#![deny(rustdoc::broken_intra_doc_links)]`. |
| TOOL-04 | Examples compile and demonstrate text chat, streaming, image upload, and multi-turn. | `examples/text_chat.rs`, `examples/image_chat.rs`, `examples/stream_chat.rs` exist; verify multi-turn example exists or add one. |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Public API ergonomics | Public API Layer (`src/lib.rs`, `src/client.rs`, `src/chat.rs`) | — | Consumers interact only with `GeminiClient`, `ChatBuilder`, and chat types. |
| Cookie parsing / validation | Auth Layer (`src/auth.rs`) | Public API Layer | Credentials logic is internal; only provider interface is public. |
| Session init / WIZ extraction | Session & Auth Layer (`src/session.rs`, `src/client.rs`) | Protocol Layer | HTML extraction and token lifecycle are SDK-internal. |
| Request body construction | Protocol Layer (`src/proto/mod.rs`, `src/proto/slots.rs`) | Public API Layer (via `PreparedRequest`) | 97-slot WIZ array is hidden; `PreparedRequest` is `#[doc(hidden)]`. |
| Response parsing | Protocol Layer (`src/proto/parser.rs`) | — | Converts raw WIZ frames into `ChatResponse`. |
| Upload flow | Transport + Protocol Layer (`src/upload.rs`, `src/proto/slots.rs`) | Session Layer | Uses `reqwest` and session push ID. |
| Retry / transient detection | Session & Auth Layer (`src/retry.rs`, `src/errors.rs`) | Transport Layer | Wraps `reqwest` errors and decides retry eligibility. |
| Error surface | Public API Layer (`src/errors.rs`) | All layers | Single `Error` enum exposed to callers. |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tokio` | 1.40 (`full`) | Async runtime and sync primitives | De facto standard for async Rust; already locked by project. [VERIFIED: cargo metadata] |
| `reqwest` | 0.12 (`cookies`, `json`, `multipart`, `stream`) | HTTP client for Gemini frontend, WAA, ogads, upload | Mature, cookie-aware, streaming support. [VERIFIED: cargo metadata] |
| `serde` / `serde_json` | 1.0 | WIZ protocol (de)serialization | Standard Rust serialization stack. [VERIFIED: cargo metadata] |
| `thiserror` | 1.0 | Derive-based `Error` enum | Reduces boilerplate, keeps error type idiomatic. [VERIFIED: cargo metadata] |
| `tracing` | 0.1 | Structured logging | Rust ecosystem standard; library already uses it. [VERIFIED: cargo metadata] |
| `backoff` | 0.4 (`tokio`) | Exponential backoff retry wrapper | Already in use; CONCERNS.md notes it is lightly maintained but acceptable for v0.1. [VERIFIED: cargo metadata] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `uuid` | 1.11 (`v4`) | Request UUID generation | Already used; stable. [VERIFIED: cargo metadata] |
| `base64` | 0.22 | Inline image encoding / WAA token encoding | Already used. [VERIFIED: cargo metadata] |
| `urlencoding` | 2.1 | URL-encoded `f.req` bodies | Already used. [VERIFIED: cargo metadata] |
| `rand` | 0.8 | Nonce generation | Already used. [VERIFIED: cargo metadata] |
| `sha1` | 0.10 | SAPISIDHASH authorization | Already used. [VERIFIED: cargo metadata] |
| `futures` / `async-stream` | 0.3 | Stream helpers for attestation and examples | Already used. [VERIFIED: cargo metadata] |
| `static_assertions` | 1.1 | Compile-time `Send + Sync + 'static` checks on `Error` | Recommended addition for API-02 verification. [ASSUMED] |
| `tokio-test` | 0.4 | `#[tokio::test]` for async tests | Dev dependency; already present. [VERIFIED: cargo metadata] |
| `wiremock` | 0.6 | HTTP mocking for integration tests | Dev dependency; not currently used, but recommended for trait-based client tests. [VERIFIED: cargo metadata] |
| `criterion` | 0.5 (`async_tokio`) | Benchmarks | Already used for slot-building benchmark. [VERIFIED: cargo metadata] |
| `dotenvy` | 0.15 | Load `.env` files for live tests | Dev dependency; already present. [VERIFIED: cargo metadata] |
| `regex` | 1.11 | Fixture redaction (feature-gated) | Optional, used only by `capture-fixtures` feature. [VERIFIED: cargo metadata] |
| `tokio-tungstenite` | 0.24 | CDP WebSocket for browser attestation | Optional `browser-attestation` feature. [VERIFIED: cargo metadata] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `backoff` | `tokio-retry2` or hand-rolled | `backoff` is already integrated and passes tests; migration can be deferred to Phase 2 per CONCERNS.md. |
| `reqwest` | `hyper` directly | `reqwest` provides cookie handling, connection pooling, and multipart that the protocol needs; not worth hand-rolling. |
| `static_assertions` | Compile-fail doctests | `static_assertions` is concise and does not require nightly; add as dev-dependency. |

**Installation (no new runtime deps required for Phase 1):**
```bash
# Optional dev-dependency for compile-time trait checks
cargo add --dev static_assertions
```

**Version verification:** Current crate versions verified via `cargo metadata` and match `Cargo.toml` exactly. No new external package is strictly required for Phase 1; `static_assertions` is optional.

## Package Legitimacy Audit

> Phase 1 does not introduce new runtime crates. All dependencies listed in `Cargo.toml` already exist in the lockfile and were resolved from crates.io.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `tokio` | crates.io | ~7 yrs | very high | github.com/tokio-rs/tokio | OK | Approved |
| `reqwest` | crates.io | ~8 yrs | very high | github.com/seanmonstar/reqwest | OK | Approved |
| `serde` | crates.io | ~9 yrs | very high | github.com/serde-rs/serde | OK | Approved |
| `thiserror` | crates.io | ~6 yrs | very high | github.com/dtolnay/thiserror | OK | Approved |
| `tracing` | crates.io | ~6 yrs | very high | github.com/tokio-rs/tracing | OK | Approved |
| `backoff` | crates.io | ~6 yrs | moderate | github.com/ihrwein/backoff | OK | Approved (lightly maintained; deferred) |
| `static_assertions` | crates.io | ~7 yrs | high | github.com/nvzqz/static-assertions | OK | Approved (optional) |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```text
                    Caller
                      │
                      ▼
            ┌──────────────────┐
            │  GeminiClient    │  ← Arc<Inner>: cloneable, Send+Sync
            │  src/client.rs   │
            └────────┬─────────┘
                     │ chat() / continue_conversation()
                     ▼
            ┌──────────────────┐
            │   ChatBuilder    │  ← fluent per-turn config
            │  src/client.rs   │
            └────────┬─────────┘
                     │ send_message_with_content()
                     ▼
            ┌──────────────────┐
            │     chat.rs      │  ← ChatMessage, ContentPart,
            │  prepare_request │    Conversation, GenerationConfig
            └────────┬─────────┘
                     │ PreparedRequest
                     ▼
            ┌──────────────────┐     ┌─────────────────┐
            │   proto/slots.rs │────▶│   proto/mod.rs  │
            │ build_inner_req_list   │  body builders  │
            └────────┬─────────┘     └─────────────────┘
                     │
                     ▼
            ┌──────────────────┐
            │  reqwest Client  │  ← transport, retries, cookies
            │  src/retry.rs    │
            └────────┬─────────┘
                     │ HTTP
                     ▼
            ┌──────────────────┐
            │ gemini.google.com│
            │  batchexecute /  │
            │ StreamGenerate   │
            └──────────────────┘
                     │
                     ▼
            ┌──────────────────┐
            │  proto/parser.rs │  ← parse_chat_response, parse_model_list
            └────────┬─────────┘
                     │ ChatResponse
                     ▼
                   Caller
```

### Recommended Project Structure

```
src/
├── lib.rs              # crate root, lint config, re-exports
├── client.rs           # GeminiClient, ChatBuilder, session orchestration
├── chat.rs             # chat types, Conversation, GenerationConfig
├── auth.rs             # Credentials, Cookies, CredentialsProvider trait
├── errors.rs           # Error enum, Result alias
├── models.rs           # ModelCategory, ModelInfo
├── session.rs          # SessionState, HTML extraction helpers
├── upload.rs           # resumable image upload
├── retry.rs            # exponential backoff wrapper
├── proto/
│   ├── mod.rs          # body builders, shared constants
│   ├── slots.rs        # 97-slot StreamGenerate list
│   └── parser.rs       # response parsing
└── attestation.rs      # optional browser CDP attestation (feature-gated)

tests/
├── fixtures/           # captured HTML/JSON fixtures
├── integration_tests.rs
├── proto_tests.rs
└── real_cookies.rs     # live tests (ignored without env)

examples/
├── text_chat.rs
├── image_chat.rs
├── stream_chat.rs
└── multi_turn_chat.rs  # recommended addition for TOOL-04

benches/
└── slot_building.rs
```

### Pattern 1: Inner-Struct Wrapped in `Arc`
**What:** `GeminiClient` holds `Arc<Inner>` so clones share HTTP client, cookies, and session state while remaining `Send + Sync`.
**When to use:** For any async SDK entry point that needs to be cheaply cloneable and share mutable state across tasks.
**Example:**
```rust
// Source: src/client.rs:45-54
#[derive(Clone)]
pub struct GeminiClient {
    inner: Arc<Inner>,
}

struct Inner {
    http: Client,
    cookies: Cookies,
    session: Mutex<SessionState>,
    config: Mutex<ClientConfig>,
}
```

### Pattern 2: Fluent Builder for Per-Turn Config
**What:** `ChatBuilder` captures model category and generation config before sending a message.
**When to use:** When callers want a chainable API for optional settings without mutating the client.
**Example:**
```rust
// Source: src/client.rs:957-976
pub struct ChatBuilder<'a> {
    client: &'a GeminiClient,
    conversation: Option<Conversation>,
    category: ModelCategory,
    config: Option<GenerationConfig>,
}

impl<'a> ChatBuilder<'a> {
    pub fn with_category(mut self, category: ModelCategory) -> Self { ... }
    pub fn with_config(mut self, config: GenerationConfig) -> Self { ... }
}
```

### Pattern 3: Single `#[non_exhaustive]` Error Enum with `thiserror`
**What:** One public `Error` enum carries all SDK failures, supports `#[from]` conversions, and exposes `is_transient()`.
**When to use:** When consumers should match on broad categories and the library may add variants in minor releases.
**Example:**
```rust
// Source: src/errors.rs:12-68
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    // ...
}
```

### Pattern 4: Auth Provider Trait
**What:** Introduce a small async-capable trait so auth can be sourced from environment, files, or keyrings without changing the client constructor surface.
**When to use:** When the default cookie-string constructor should remain simple but advanced users need pluggable auth.
**Example:**
```rust
// Recommended pattern, aligned with AUTH-02
use std::future::Future;
use std::pin::Pin;

pub trait CredentialsProvider: Send + Sync {
    fn credentials(&self) -> Pin<Box<dyn Future<Output = crate::Result<Credentials>> + Send + '_>>;
}

impl CredentialsProvider for Credentials {
    fn credentials(&self) -> Pin<Box<dyn Future<Output = crate::Result<Credentials>> + Send + '_>> {
        Box::pin(async move { Ok(self.clone()) })
    }
}
```

### Anti-Patterns to Avoid
- **Blocking locks in synchronous builder methods:** `update_config_blocking` calls `Mutex::blocking_lock()` inside non-async `with_language`/`with_max_retries`/`with_timeout` and can panic inside a Tokio runtime. Replace with `tokio::sync::RwLock` or async builder stages. [Source: CONCERNS.md, src/client.rs:136]
- **Silent fallback for WAA context:** `ogads_get_async_data` errors fall back to `build_default_waa_context()` silently, hiding degraded attestation. Surface via `tracing::warn!` or an internal flag. [Source: CONCERNS.md, src/client.rs:545]
- **Cookie merge on a clone that is dropped:** `accept_consent_and_refresh` merges response cookies into a local `cookies` clone that is never written back to `self.inner.cookies`, losing refreshed `SOCS` cookies. [Source: CONCERNS.md, src/client.rs:755]
- **Large, multi-responsibility client methods:** `stream_generate_raw` mixes session locking, upload, header construction, and retry invocation; decompose into smaller helpers with explicit inputs/outputs. [Source: CONCERNS.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP client / connection pooling | Custom hyper wrapper | `reqwest::Client` | Already handles cookies, TLS, timeouts, streaming, multipart. |
| JSON serialization | Manual string building for WIZ bodies | `serde_json::Value` + `serde` | Correct escaping, predictable output, easy fixture tests. |
| Exponential backoff | Inline sleep loops | `backoff::ExponentialBackoff` (or migrate later) | Jitter, max elapsed time, transient/permanent classification. |
| Error enum boilerplate | Manual `Display`/`Error` impls | `thiserror` | Standard, maintainable, supports `#[from]` and `#[non_exhaustive]`. |
| Cookie redaction regex | Manual substring masking | Custom `Debug` impl returning `"<redacted>"` | Simpler and eliminates prefix leakage risk. |
| Base64 encoding | Manual alphabet | `base64::engine::general_purpose::STANDARD` | Correct padding, well-tested. |
| SAPISIDHASH | Manual SHA1 plumbing | `sha1` crate inside `Credentials::sapisid_hash` | Already implemented and tested. |

**Key insight:** The only hand-rolled code that should remain is the protocol-specific WIZ slot layout and HTML extraction, because these are reverse-engineered from live traffic and have no off-the-shelf library.

## Common Pitfalls

### Pitfall 1: `#[non_exhaustive]` on Structs Without Private Fields
**What goes wrong:** Adding `#[non_exhaustive]` to a struct that also has a public `Default` impl and all public fields still allows consumers to construct it with functional record update syntax (`Foo { x, ..Default::default() }`), but not with a literal. This can be surprising.
**Why it happens:** `#[non_exhaustive]` only affects code outside the crate; constructors become the primary control point.
**How to avoid:** For `ChatResponse` and `Conversation`, prefer a private `_priv: ()` field plus a public constructor, or ensure all construction goes through `Default` and explicit builder methods. Keep `#[non_exhaustive]` on enums like `ModelCategory` and `ImageSource`.
**Warning signs:** Tests outside the crate that use struct literals start failing after adding `#[non_exhaustive]`.

### Pitfall 2: `Mutex::blocking_lock` Inside an Async Runtime
**What goes wrong:** `update_config_blocking` panics when called from a thread already running a Tokio runtime without a blocking thread pool.
**Why it happens:** Tokio's `Mutex::blocking_lock` is intended for blocking contexts, not async worker threads.
**How to avoid:** Replace `config: Mutex<ClientConfig>` with `tokio::sync::RwLock<ClientConfig>` and use `blocking_write()` only in truly blocking contexts, or change builder methods to async where the caller already awaits. For Phase 1, the simplest fix is `RwLock` + `try_write`/`blocking_write` with documented restrictions.
**Warning signs:** Examples that call `with_language` inside `#[tokio::main]` panic at runtime.

### Pitfall 3: Broken Intra-Doc Links After Tightening Lints
**What goes wrong:** Switching to `#![deny(missing_docs)]` and `#![deny(rustdoc::broken_intra_doc_links)]` fails CI because of the existing `src/auth.rs:121` link to a non-existent `GeminiClient::verify_signed_in`.
**Why it happens:** `verify_signed_in` is a method on `GeminiClient`, but the doc comment is inside `auth.rs`, where `GeminiClient` is not in scope for rustdoc link resolution.
**How to avoid:** Use a fully-qualified path `` [`crate::client::GeminiClient::verify_signed_in`] `` or a plain URL. Run `cargo doc --no-deps` after every doc change.
**Warning signs:** `cargo doc` emits a warning about unresolved links.

### Pitfall 4: Partial Credential Redaction in Debug
**What goes wrong:** Showing the first four characters of each secret leaks prefix entropy and length, which can aid offline attacks or correlation.
**Why it happens:** Current `Credentials` Debug formats `"{}...<redacted>"` with the first 4 chars.
**How to avoid:** Format every secret field as `"<redacted>"` (or `(empty)` when empty) in `Debug`.
**Warning signs:** `assert!(!format!("{:?}", creds).contains("abc"))` fails in auth tests.

### Pitfall 5: Accidentally Breaking the WIZ Slot Layout During Refactor
**What goes wrong:** Moving slot constants or changing `build_inner_req_list` indices breaks live traffic compatibility even though unit tests pass.
**Why it happens:** Tests use static fixtures from an older capture; they cannot detect protocol drift.
**How to avoid:** Keep `SLOT_COUNT = 97`, centralize named constants (Phase 2), and add round-trip tests that compare against committed fixture files.
**Warning signs:** Live-cookie tests (`tests/real_cookies.rs`) fail after a "safe" refactor.

## Code Examples

### Constructing and Using a `GeminiClient`
```rust
// Source: src/lib.rs doc example
use gemini_sdk::GeminiClient;

# async fn run() -> gemini_sdk::Result<()> {
let cookies = "__Secure-1PSID=YOUR_PSID; __Secure-1PSIDCC=YOUR_PSIDCC";
let client = GeminiClient::from_cookie_header(cookies)?;
let response = client
    .chat()
    .send_message("What is Rust?")
    .await?;
println!("{}", response.text());
# Ok(())
# }
```

### Sending a Message with an Inline Image
```rust
// Source: src/chat.rs:64-70 and src/client.rs:985-995
use gemini_sdk::{ChatMessage, ImageSource};

let mut message = ChatMessage::user("Describe this image");
message.parts.push(gemini_sdk::chat::ContentPart::Image(
    ImageSource::from_bytes("image/png", b"\x89PNG..."),
));
```

### Cookie Parsing and Validation
```rust
// Source: src/auth.rs:90-114
use gemini_sdk::auth::{Credentials, PSID, PSIDCC};

let creds = Credentials::from_header(
    "__Secure-1PSID=abc; __Secure-1PSIDCC=def"
).unwrap();
assert_eq!(creds.psid, "abc");
assert_eq!(creds.psidcc, "def");
```

### Multi-Turn Conversation
```rust
// Source: src/client.rs:181-190 and src/client.rs:1001-1010
use gemini_sdk::{Conversation, GeminiClient};

# async fn run(client: &GeminiClient) -> gemini_sdk::Result<()> {
let mut conversation = Conversation::new();
let response = client
    .chat()
    .send_message("Hello")
    .await?;
conversation.add_user_text("Hello");
conversation.add_model_text(response.text.clone());

let response2 = client
    .continue_conversation(conversation)
    .send_message("What can you do?")
    .await?;
# Ok(())
# }
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hard-coded `bl` build label | Extract `cfb2h` from `window.WIZ_global_data` | Spike 007 / current codebase | Reduces breakage when Google deploys a new frontend build. |
| Opaque cookie string everywhere | Typed `Credentials` + `Cookies` wrappers | Current codebase | Easier validation, redaction, and merge logic. |
| Raw streaming `reqwest::Response` returned directly | `stream_generate` returns raw response; `generate` returns parsed `ChatResponse` | Current codebase | Streaming parsing is still caller-responsibility; planned typed stream adapter deferred. |
| Manual retry loops | `backoff::ExponentialBackoff` wrapper | Current codebase | Consistent transient handling. |

**Deprecated/outdated:**
- `Cookies` wrapper: `Credentials` is now the preferred typed representation; `Cookies` is kept for backward compatibility but may be deprecated in v0.2.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `static_assertions` is the simplest way to verify `Error: Send + Sync + 'static` in stable Rust. | Standard Stack | Low — alternative is a compile-fail doctest, which is more verbose but equally correct. |
| A2 | `reqwest::Client` does not need to be injectable in Phase 1; REL-04 is Phase 3. | Architecture Patterns | Medium — if planners want REL-04 earlier, the client inner struct must be redesigned. |
| A3 | No new runtime dependencies are required for Phase 1; only dev-dependency additions are optional. | Standard Stack | Low — all Phase 1 requirements can be met with existing crates. |

**If this table is empty:** Not applicable — assumptions are listed above.

## Open Questions

1. **Should `ChatResponse` expose a streaming API in v0.1?**
   - What we know: CHAT-02 requires streaming to yield `ChatResponse` chunks via `futures::Stream`. The current `stream_generate` returns a raw `reqwest::Response`.
   - What's unclear: Whether a typed stream adapter should be implemented in Phase 1 or deferred.
   - Recommendation: Defer the typed stream adapter to Phase 2/3; in Phase 1, stabilize the raw streaming method signature and document that parsing is caller-managed.

2. **How should `CredentialsProvider` expose async behavior?**
   - What we know: Providers may read from files, env, or keyrings; some may need async.
   - What's unclear: Whether the trait should be async (requires `async-trait` or RPITIT) or sync with the client initializing auth once.
   - Recommendation: Use `async-trait` for simplicity given MSRV 1.80, or define a `Pin<Box<dyn Future>>` signature to avoid the extra dependency. Prefer the latter for v0.1 to keep dependency surface small.

3. **What is the exact `Cargo.toml` metadata needed for crates.io?**
   - What we know: `name`, `version`, `authors`, `edition`, `license`, `description`, `repository`, `keywords`, `categories`, and `rust-version` are already present.
   - What's unclear: Whether additional fields (`readme`, `homepage`, `exclude`) are required for v0.1.
   - Recommendation: Add `readme = "README.md"` and review `exclude` to keep the `.crate` file small; `cargo package` already succeeds.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | Build, test, package | ✓ | 1.95.0 | — |
| `rustc` | Compilation | ✓ | 1.95.0 | — |
| `clippy` | Linting | ✓ | 0.1.95 | — |
| `rustfmt` | Formatting | ✓ | bundled with 1.95.0 | — |
| `rustdoc` | Documentation | ✓ | bundled with 1.95.0 | — |
| `tokio` runtime | All async tests/examples | ✓ | 1.40 (Cargo.toml) | — |
| Chrome / Chromium | `browser-attestation` feature | not checked | — | Skip attestation tests/examples |
| crates.io index | `cargo package` | ✓ | online | Use `--offline` if cached |

**Missing dependencies with no fallback:** none
**Missing dependencies with fallback:** Chrome is only needed for the optional `browser-attestation` feature and the `test_attestation` example; Phase 1 core work does not require it.

## Validation Architecture

> `workflow.nyquist_validation` is enabled in `.planning/config.json`.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Built-in Rust test harness + `tokio-test` 0.4 |
| Config file | none (standard Cargo layout) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test --workspace --all-features` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| API-01 | Public enums/structs are forward-compatible | unit | `cargo test` | ✅ src/models.rs, src/chat.rs |
| API-02 | Error implements `std::error::Error` + `Send + Sync + 'static` | unit/compile | `cargo test` (add static assertions) | ✅ src/errors.rs |
| API-03 | semver policy documented | docs | `cargo doc --no-deps` | ✅ README.md / docs |
| API-04 | No missing docs on public items | docs | `cargo doc --no-deps` | ✅ src/lib.rs |
| AUTH-01 | Cookie header parses and validates required cookies | unit | `cargo test` | ✅ src/auth.rs |
| AUTH-02 | CredentialsProvider can be implemented and used | integration | `cargo test --test integration_tests` | ❌ add test |
| AUTH-03 | Debug output contains no secret material | unit | `cargo test` | ✅ src/auth.rs (update) |
| CHAT-01 | Text chat returns `ChatResponse` with text | integration | `cargo test --test proto_tests` | ✅ tests/proto_tests.rs |
| CHAT-02 | Streaming method returns raw response | integration | `cargo test --test integration_tests` | ✅ tests/integration_tests.rs |
| CHAT-03 | Conversation preserves state across turns | integration | `cargo test --test integration_tests` | ❌ add multi-turn test |
| CHAT-05 | Model category is preserved in slot building | unit | `cargo test` | ✅ src/proto/slots.rs |
| MEDIA-01 | Inline image upload produces usable reference | integration | `cargo test --test proto_tests` | ✅ src/proto/slots.rs |
| REL-01 | Retry uses exponential backoff for transient errors | unit | `cargo test` | ✅ src/retry.rs (implicit) |
| TOOL-01 | Tests pass without live cookies | full suite | `cargo test` | ✅ all fixture-based tests |
| TOOL-02 | Clippy passes | lint | `cargo clippy --all-targets -- -D warnings` | ✅ passes |
| TOOL-03 | Docs build without warnings | docs | `cargo doc --no-deps` | ❌ one broken link to fix |
| TOOL-04 | Examples compile | build | `cargo build --examples` | ✅ text/image/stream exist |

### Sampling Rate
- **Per task commit:** `cargo test` and `cargo clippy --all-targets -- -D warnings`
- **Per wave merge:** `cargo test --workspace --all-features` and `cargo doc --no-deps`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] Add `static_assertions` compile-time check for `Error: Send + Sync + 'static` (API-02).
- [ ] Add integration test for `CredentialsProvider` default provider (AUTH-02).
- [ ] Add multi-turn integration test using fixtures (CHAT-03).
- [ ] Add or verify a multi-turn example binary for TOOL-04.
- [ ] Fix broken intra-doc link in `src/auth.rs:121` (TOOL-03).

## Security Domain

> `security_enforcement` is enabled at ASVS Level 1.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Cookie-based auth via typed `Credentials`; required-cookie validation in `Credentials::from_header`. |
| V3 Session Management | yes | Session state stored in `tokio::sync::Mutex` inside `Arc<Inner>`; cookies rebuilt per request. |
| V4 Access Control | no | No role-based access; client acts as the authenticated user only. |
| V5 Input Validation | yes | Cookie parsing uses `splitn(2, '=')`; prompt empty-check in `extract_prompt`; HTML extraction uses defensive fallbacks. |
| V6 Cryptography | partial | SAPISIDHASH uses SHA1 per Google's protocol; secrets redacted in Debug; TLS handled by `reqwest`. |
| V7 Error Handling | yes | Strongly typed `Error` enum; no secrets included in error messages. |
| V10 Logging | yes | `tracing::debug!` only; credentials redacted in Debug; avoid logging cookie header values. |

### Known Threat Patterns for Rust / HTTP SDKs

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Secret leakage in logs/errors | Information Disclosure | Full redaction in `Debug`; never include cookie header in error strings. |
| SSRF via user-supplied URLs | Spoofing | Reject direct image URLs in `prepare_request` (already done). |
| Replay / stale session | Tampering | `__Secure-1PSIDTS` timestamp cookie used where required; nonce generated per request. |
| Cookie theft via insecure headers | Information Disclosure | HTTPS-only endpoints; `__Secure-` cookie prefix enforced by browser, not SDK. |
| Prototype / protocol drift causing panics | Denial of Service | Defensive parsing with structured `Error::Parse`; no `unwrap` on live responses. |

## Sources

### Primary (HIGH confidence)
- `Cargo.toml` — dependency versions, features, MSRV.
- `src/lib.rs` — public re-exports, lint configuration.
- `src/client.rs` — `GeminiClient`, `ChatBuilder`, session init, WAA chain.
- `src/auth.rs` — `Credentials`, `Cookies`, cookie validation, redaction.
- `src/errors.rs` — `Error` enum and `is_transient`.
- `src/chat.rs` — chat types, `Conversation`, `PreparedRequest`.
- `src/session.rs` — HTML extraction and `SessionState`.
- `src/proto/slots.rs` — 97-slot WIZ request construction.
- `src/proto/parser.rs` — response parsing.
- `src/upload.rs` — resumable upload flow.
- `src/retry.rs` — exponential backoff wrapper.
- `.planning/codebase/ARCHITECTURE.md`, `CONCERNS.md`, `CONVENTIONS.md`, `STACK.md`, `STRUCTURE.md`, `TESTING.md`.
- `.planning/PROJECT.md`, `ROADMAP.md`, `REQUIREMENTS.md`, `STATE.md`.

### Secondary (MEDIUM confidence)
- `cargo metadata` output confirming resolved crate versions.
- `cargo test`, `cargo clippy`, `cargo doc`, `cargo package` execution in this environment.

### Tertiary (LOW confidence)
- `static_assertions` recommendation for compile-time trait checks (not yet added to the project).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates are already resolved and locked in `Cargo.lock`.
- Architecture: HIGH — derived directly from current source and ARCHITECTURE.md.
- Pitfalls: HIGH — all listed items are observable in the current code or CONCERNS.md.

**Research date:** 2026-08-09
**Valid until:** 2026-09-09 (Rust ecosystem is stable, but Gemini frontend protocol may drift sooner).
