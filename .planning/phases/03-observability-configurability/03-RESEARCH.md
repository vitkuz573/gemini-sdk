# Phase 3: Observability & Configurability - Research

**Researched:** 2026-08-10
**Domain:** Rust SDK observability (async hooks, `tracing`, progress streams) and HTML extraction resilience
**Confidence:** HIGH

## Summary

This phase adds production-observability surfaces to the Gemini SDK without changing the underlying protocol. The work splits into four tightly coupled concerns: (1) an async request/response hook API for metering and logging, (2) `tracing` spans across auth, request, parse, and upload paths, (3) allowing callers to inject a shared `reqwest::Client` for connection-pool and timeout control, and (4) exposing upload progress as a `futures` stream. All four concerns touch the same public constructor and the same request/response flow, so the planner should group them by vertical slice rather than by layer.

The most important architectural constraint is that `GeminiClient` is `Clone` and `Send`, holds its state in an `Arc<Inner>`, and already depends on `tracing`, `futures`, and `async-stream`. No new fundamental dependencies are required for hooks, tracing, or progress streams. The hook trait can follow the same object-safe boxed-future pattern already used for `CredentialsProvider` to avoid pulling in `async-trait` unless the user explicitly prefers it. The existing `tracing` dependency means spans can be added immediately. For upload progress, the existing two-step resumable upload can be instrumented at `start_upload` / `upload_chunk` / `finalize_upload` boundaries and wrapped in an `async_stream::stream!` adapter.

HTML extraction fallback resilience (PROTO-03) is a separate concern but interacts with session initialization: the extractors for `SNlM0e`, `cfb2h` (build label), `FdrFJe` (session id), push id, and WAA fingerprint should try a primary key inside `window.WIZ_global_data`, then a list of known aliases, before giving up. Fixture-based tests should cover each alias shape so future Google layout changes are caught early.

**Primary recommendation:** Implement hooks, tracing, HTTP-client injection, and upload progress as a single vertical slice on `GeminiClient`, then layer the HTML extractor fallback refactor on top. Use boxed-future traits to stay consistent with `CredentialsProvider`, instrument public async methods with `#[tracing::instrument]`, and expose progress via `futures::Stream<Item = Result<UploadEvent>>`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Request/response hooks | SDK client | User-provided closure/trait | The client owns the lifecycle and call sites; users provide behavior. |
| `tracing` spans | SDK client | Executor (Tokio) | The SDK emits spans; the runtime's subscriber decides collection/filtering. |
| Injectable HTTP client | SDK client | Caller | Construction-time injection lets callers reuse connection pools. |
| Upload progress stream | SDK client | `futures` runtime | The SDK produces events; callers consume via stream adapters. |
| HTML extraction fallbacks | Session extractor | Parser utilities | Extraction is a parsing concern isolated to `session.rs`. |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tracing` | 0.1.44 | Structured spans/events already used by the SDK. | Already in `Cargo.toml`; de-facto Rust observability standard. |
| `futures` | 0.3.33 | Stream trait and adapters for progress events. | Already a dependency; required by existing streaming code. |
| `async-stream` | 0.3.31 | Ergonomic `async_stream::stream!` macro. | Already a dependency; used for `generate_stream` adapter. |
| `reqwest` | 0.12 | HTTP client being injected. | Already the SDK's HTTP stack. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `async-trait` | 0.1.92 | Optional macro for `HttpHook` if config prefers. | Only if the project explicitly decides the boxed-future pattern is too noisy. Default is boxed future to match `CredentialsProvider`. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `tracing` | `log` + `env_logger` | `tracing` is already in use and provides spans/structured fields. |
| Boxed-future hook trait | `async-trait` | `async-trait` adds a dependency and compile-time macro expansion; boxed future keeps the crate consistent with `CredentialsProvider`. |
| `futures::Stream` progress | Async callback closure | Streams compose better with existing async Rust and can be mapped/filtered. |

**Installation:** No new installs are strictly required; `tracing`, `futures`, `async-stream`, and `reqwest` are already present. If the planner decides on `async-trait`:

```bash
cargo add async-trait
```

**Version verification:**
- `tracing 0.1.44` — cargo search confirmed.
- `async-trait 0.1.92` — cargo search confirmed.
- `futures 0.3.33` — cargo search confirmed.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `tracing` | crates.io | 8 yrs | 13.2M/wk | tokio-rs/tracing | OK | Approved |
| `async-trait` | crates.io | 6 yrs | 9.7M/wk | dtolnay/async-trait | OK | Approved (optional) |
| `futures` | crates.io | 9 yrs | 11.9M/wk | rust-lang/futures-rs | OK | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
Caller-provided hook(s)
         │
         ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  GeminiClient   │────▶│  request build  │────▶│   hook call     │
│  (Arc<Inner>)   │     │  + signing      │     │  (async)        │
└─────────────────┘     └─────────────────┘     └─────────────────┘
         │                                               │
         │                                               ▼
         │                                      ┌─────────────────┐
         │                                      │  HTTP request   │
         │                                      │  (injected or   │
         │                                      │   built-in)     │
         │                                      └─────────────────┘
         │                                               │
         │                                               ▼
         │                                      ┌─────────────────┐
         │                                      │  response parse │
         │                                      │  + retry        │
         │                                      └─────────────────┘
         │                                               │
         │                                               ▼
         │                                      ┌─────────────────┐
         └──────────────────────────────────────│  hook call +    │
                                                │  tracing span   │
                                                │  completion     │
                                                └─────────────────┘
```

### Recommended Project Structure

No new modules are required beyond expanding existing ones:

```
src/
├── client.rs      # hook storage, from_http_client, span attributes
├── chat.rs        # GenerationConfig progress callback field, PreparedRequest exposure
├── upload.rs      # upload_with_progress, UploadEvent, WebAttachment
├── session.rs     # fallback extractors
└── lib.rs         # re-exports
```

### Pattern 1: Object-Safe Async Hook with Boxed Futures

**What:** Define `HttpHook` as a trait returning `Pin<Box<dyn Future<Output = Result<()>> + Send>>` to match `CredentialsProvider`.

**When to use:** When the project avoids `async-trait` to keep dependencies minimal.

**Example:**

```rust
use std::future::Future;
use std::pin::Pin;
use crate::errors::Result;

pub trait HttpHook: Send + Sync {
    fn on_request<'a>(
        &'a self,
        request: &'a PreparedRequest,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    fn on_response<'a>(
        &'a self,
        response: &'a ChatResponse,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}
```

Source: existing `CredentialsProvider` pattern in `src/auth.rs`.

### Pattern 2: Tracing Span on Public Async Method

**What:** Add `#[tracing::instrument(level = "info", skip_all, fields(operation = "..."))]` to public `async` methods.

**When to use:** For every user-facing async entry point.

**Example:**

```rust
#[tracing::instrument(level = "info", skip_all, fields(operation = "chat.generate"))]
pub async fn generate(&self, prompt: impl Into<String>) -> Result<ChatResponse> { ... }
```

### Pattern 3: Progress Stream Adapter

**What:** Wrap the upload steps in `async_stream::stream!` yielding `UploadEvent`.

**When to use:** When callers need observable progress for long-running uploads.

**Example:**

```rust
pub fn upload_with_progress(
    &self,
    mime_type: impl Into<String>,
    bytes: Vec<u8>,
) -> impl Stream<Item = Result<UploadEvent>> {
    let client = self.clone();
    async_stream::stream! {
        let total = bytes.len() as u64;
        yield Ok(UploadEvent::Progress { uploaded: 0, total: Some(total) });
        // ... start, chunk, finalize ...
        yield Ok(UploadEvent::Complete { attachment });
    }
}
```

### Pattern 4: HTML Extractor Fallback Chain

**What:** Try a list of known keys/selectors in order, returning the first valid value.

**When to use:** Any extractor that depends on undocumented Google HTML layout.

**Example:**

```rust
fn extract_snlim0e(body: &str) -> Option<String> {
    let block = extract_wiz_global_data_block(body).unwrap_or(body);
    for key in ["SNlM0e", "SnlM0e", "snlM0e"] {
        if let Some(t) = extract_quoted_value(block, key) {
            if is_valid_snlim0e(&t) { return Some(t); }
        }
    }
    None
}
```

### Anti-Patterns to Avoid

- **Calling hooks inside a lock:** Hooks are async; do not hold `Mutex`/`RwLock` guards across `.await`.
- **Including secrets in span fields:** Never put cookies, tokens, or prompt text in `tracing` fields above `debug` level.
- **Making hook errors fatal by default:** Default should be log-and-continue; fatal opt-in only.
- **Blocking the upload stream with large reads:** Stream chunks must be yielded between I/O operations so consumers see progress.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async trait object safety | Macro-free trait that isn't object-safe | Boxed future or `async-trait` | Object-safe async traits require indirection; boxed futures keep the pattern consistent. |
| Progress event delivery | Callback-based polling | `futures::Stream` | Streams are the standard compositional primitive in async Rust. |
| Span/context propagation | Manual thread-local juggling | `tracing` span API | `tracing` already handles context propagation across `.await`. |
| HTML key fallback parsing | Fragile regex chains | Balanced-brace block extraction + `extract_quoted_value` | Regex on minified HTML is brittle; the existing brace-aware parser is safer. |

## Runtime State Inventory

Not a rename/refactor/migration phase — skipped.

## Common Pitfalls

### Pitfall 1: Hook Errors Aborting User Requests
**What goes wrong:** A buggy user hook panics or returns an error, aborting a chat request.
**Why it happens:** Hooks run inside the critical request path.
**How to avoid:** Default to log-and-continue; fatal behavior behind an explicit config flag.
**Warning signs:** Integration tests start failing when a hook is attached.

### Pitfall 2: Tracing Spans Leaking Secrets
**What goes wrong:** A span field captures a cookie or prompt string and gets emitted to a log aggregator.
**Why it happens:** `tracing::instrument` auto-captures arguments; `skip_all` must be used and fields chosen carefully.
**How to avoid:** Use `skip_all` on public methods; only include non-secret metadata (operation name, model category, byte counts).
**Warning signs:** Logs contain base64 image data or cookie fragments.

### Pitfall 3: Injected Client Losing Required Configuration
**What goes wrong:** Callers inject a `reqwest::Client` without cookie or redirect handling that the SDK assumed.
**Why it happens:** The SDK currently builds its own client with `.timeout(...)` but does not set a cookie store.
**How to avoid:** Document that the SDK uses the injected client as-is; it is the caller's responsibility to configure cookies/redirects if needed. The SDK sets headers per-request.
**Warning signs:** Cookie-based auth fails after switching to `from_http_client`.

### Pitfall 4: Progress Stream Dropping on Early Cancellation
**What goes wrong:** A consumer drops the stream before `Complete`, leaving a partially uploaded file on Google's side.
**Why it happens:** Upload is a side-effecting stream; cancellation does not roll back server state.
**How to avoid:** Document that progress streams are best-effort and that the upload continues until the stream is polled to completion or the underlying request fails.
**Warning signs:** Tests see `Progress` events but never `Complete`.

## Code Examples

### Adding a Hook to ClientConfig

```rust
// In src/client.rs
struct ClientConfig {
    // ... existing fields ...
    http_hook: Option<Arc<dyn HttpHook>>,
    fatal_hook_errors: bool,
}
```

### Instrumenting the Upload Path

```rust
#[tracing::instrument(level = "debug", skip_all, fields(bytes = bytes.len()))]
async fn upload_file(...) -> Result<String> { ... }
```

### HTML Fallback Test Fixture

```rust
#[test]
fn extract_snlim0e_alias() {
    let body = r#"window.WIZ_global_data = {"SnlM0e":"abc:1234567890123"};"#;
    let state = extract_from_app_html(body);
    assert!(state.access_token.is_some());
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `log` crate | `tracing` | SDK bootstrap | Spans + structured fields + subscriber filtering. |
| Hand-rolled retry | `backoff` crate | Phase 1 | Reusable exponential backoff with jitter. |
| Inline async-trait | Boxed futures | Phase 1 | Keeps dependency surface minimal. |

**Deprecated/outdated:**
- `async-trait` is still widely used but not required here.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Boxed-future hook trait is preferred over `async-trait` to match `CredentialsProvider`. | Standard Stack | If the user changes their mind, a small refactor to `async-trait` is straightforward. |
| A2 | `reqwest::Client` injection happens at construction time, not per-request. | Injectable HTTP Client | Per-request injection would require larger API changes; constructor injection matches the existing pattern. |
| A3 | Existing `futures`/`async-stream` versions support the required `Stream` combinators. | Standard Stack | Confirmed by `Cargo.toml` and existing `generate_stream` usage. |

**If this table is empty:** Not applicable — assumptions exist and are listed above.

## Open Questions

1. **Should `HttpHook` receive raw bytes or typed structures?**
   - What we know: CONTEXT.md says `PreparedRequest` and `ChatResponse`.
   - What's unclear: Whether raw request/response bytes are useful for logging.
   - Recommendation: Start with typed structures; add raw bytes later if requested.

2. **Should hook errors be fatal by default?**
   - What we know: CONTEXT.md says non-fatal by default with opt-in fatal flag.
   - What's unclear: Exact config flag name and shape.
   - Recommendation: Add `ClientConfig::fatal_hook_errors: bool` defaulting to `false`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Cargo / Rust | Build/test | ✓ | 1.95.0 | — |
| `cargo test` | Validation | ✓ | — | — |
| `cargo clippy` | Validation | ✓ | — | — |
| `cargo doc` | Validation | ✓ | — | — |

**Missing dependencies with no fallback:** none
**Missing dependencies with fallback:** none

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in) + `tokio-test` for async |
| Config file | none — standard Cargo layout |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test --all-targets` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| OBS-01 | Hook is called on request and response | unit | `cargo test --lib hooks` | ❌ Wave 0 |
| OBS-02 | Tracing span covers `generate` / `generate_stream` / `list_models` / `verify_signed_in` | unit | `cargo test --lib tracing` | ❌ Wave 0 |
| REL-04 | `GeminiClient::from_http_client` returns a working client | unit/integration | `cargo test --lib from_http_client` | ❌ Wave 0 |
| MEDIA-02 | `upload_with_progress` yields `Progress` then `Complete` | unit/integration | `cargo test --lib upload_progress` | ❌ Wave 0 |
| PROTO-03 | Extractors fall back through alias keys | unit | `cargo test --lib session_extractors` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo test --all-targets`
- **Phase gate:** `cargo test --all-targets` green before `/gsd-verify-work`

### Wave 0 Gaps

- `tests/unit/hooks.rs` — covers OBS-01
- `tests/unit/tracing.rs` — covers OBS-02
- `tests/unit/http_client.rs` — covers REL-04
- `tests/unit/upload_progress.rs` — covers MEDIA-02
- `tests/unit/session_extractors.rs` — covers PROTO-03
- Shared fixtures for each HTML alias shape

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Auth unchanged this phase. |
| V3 Session Management | no | Session handling unchanged. |
| V4 Access Control | no | No authorization decisions added. |
| V5 Input Validation | yes | Hook inputs are caller-controlled; validate no panic on malformed `ChatResponse`. |
| V6 Cryptography | no | No crypto added. |
| V7 Error Handling | yes | Hook errors must not leak secrets; tracing fields must exclude credentials. |
| V8 Data Protection | yes | Spans and hook payloads must not log cookies, tokens, or prompt content. |

### Known Threat Patterns for Rust SDKs

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Secret leakage in logs | Information Disclosure | `skip_all` on spans; never emit credentials or prompt text. |
| Hook panic aborts request | Denial of Service | Catch panics or treat hook errors as non-fatal by default. |
| Malicious hook captures response | Information Disclosure | Hooks run in caller's process; document least-privilege. |
| Progress stream resource exhaustion | Denial of Service | Bound buffer sizes; yield control between chunks. |

## Sources

### Primary (HIGH confidence)
- `Cargo.toml` — confirms `tracing`, `futures`, `async-stream`, `reqwest` are already dependencies.
- `src/auth.rs` — confirms boxed-future `CredentialsProvider` pattern.
- `src/client.rs` — confirms `Arc<Inner>` structure and `ClientConfig` fields.
- `src/upload.rs` — confirms two-step resumable upload flow.
- `src/session.rs` — confirms existing extractor patterns and fallback keys.

### Secondary (MEDIUM confidence)
- `tracing` official docs — span/event API best practices.
- `futures` crate docs — `Stream` combinators.

### Tertiary (LOW confidence)
- none

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already present and verified.
- Architecture: HIGH — patterns are consistent with existing code.
- Pitfalls: MEDIUM — derived from general Rust/tracing experience and code review.

**Research date:** 2026-08-10
**Valid until:** 2026-09-10 (stable stack)
