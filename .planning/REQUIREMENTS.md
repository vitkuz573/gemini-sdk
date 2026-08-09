# Requirements: Gemini SDK

**Defined:** 2026-08-08
**Core Value:** Developers can reliably integrate Gemini into Rust applications using a stable, documented, semver-respecting SDK that handles auth, protocol quirks, retries, and common content types out of the box.

## v1 Requirements

### Public API

- [ ] **API-01**: `GeminiClient`, `ChatBuilder`, `Conversation`, and `ChatResponse` types are marked `#[non_exhaustive]` or otherwise documented for forward compatibility.
- [ ] **API-02**: All public error types live in a dedicated module and implement `std::error::Error` + `Send` + `Sync` + `'static`.
- [ ] **API-03**: Breaking changes to public types are only introduced in minor 0.x versions before v1.0 and in major versions after v1.0.
- [ ] **API-04**: Crate compiles with `#![deny(missing_docs)]` on public items.

### Auth

- [ ] **AUTH-01**: Cookie-based auth accepts a header string and validates required cookies (`__Secure-1PSID`, `__Secure-1PSIDCC`).
- [ ] **AUTH-02**: A `CredentialsProvider` trait allows custom auth sources (env, file, keyring) without changing `GeminiClient` API.
- [ ] **AUTH-03**: Credentials are fully redacted in `Debug` output.
- [ ] **AUTH-04**: Consent flow (`SOCS` cookie acquisition) persists merged cookies back into the client state.

### Chat

- [ ] **CHAT-01**: Text-only chat returns a complete `ChatResponse` with text and optional reasoning content.
- [ ] **CHAT-02**: Streaming chat yields `ChatResponse` chunks via `futures::Stream`.
- [ ] **CHAT-03**: Multi-turn `Conversation` preserves state across turns and can be cloned/shared safely.
- [ ] **CHAT-04**: System instructions and generation config (temperature, top_p, max_tokens) can be set per chat or per client.
- [ ] **CHAT-05**: Model category selection (`Auto`, `Pro`, `Flash`, etc.) is preserved and validated.

### Media

- [ ] **MEDIA-01**: Inline image uploads encode data and produce a usable upload ID.
- [ ] **MEDIA-02**: Upload progress is observable through an async callback or stream.
- [ ] **MEDIA-03**: Audio and video uploads are supported with the same progress semantics as images.

### Protocol Resilience

- [ ] **PROTO-01**: WIZ slot indices are centralized as named constants in one module.
- [ ] **PROTO-02**: Parser tests cover every documented response shape with fixture files.
- [ ] **PROTO-03**: HTML extraction falls back through multiple selectors/keys when Google changes `window.WIZ_global_data` layout.
- [ ] **PROTO-04**: Missing or unexpected protocol fields produce structured errors instead of panics.

### Reliability

- [ ] **REL-01**: Retries use exponential backoff with jitter for transient HTTP errors and rate limits.
- [ ] **REL-02**: Non-async builder methods do not call `blocking_lock` inside a Tokio runtime.
- [ ] **REL-03**: WAA / ogads attestation failures surface a typed error instead of silently falling back to synthetic context.
- [ ] **REL-04**: A shared `reqwest::Client` can be injected to control connection pooling.

### Observability

- [ ] **OBS-01**: Request and response hooks allow callers to log, meter, or transform traffic.
- [ ] **OBS-02**: `tracing` spans cover major operations (auth, request, parse, upload).
- [ ] **OBS-03**: Metrics facade exposes counters for requests, retries, parse failures, and attestation outcomes.

### Advanced Features

- [ ] **ADV-01**: Tools / function calling parses tool declarations, invokes local handlers, and sends results back to the model.
- [ ] **ADV-02**: Session persistence helpers save/restore conversation and auth state.
- [ ] **ADV-03**: Auto cookie refresh detects expiry and re-runs consent/auth flow where possible.

### Tooling & Release

- [ ] **TOOL-01**: `cargo test` passes without live cookies using fixtures and mocked fixtures.
- [ ] **TOOL-02**: `cargo clippy --all-targets -- -D warnings` passes.
- [ ] **TOOL-03**: `cargo doc --no-deps` builds with no warnings.
- [ ] **TOOL-04**: Examples compile and demonstrate text chat, streaming, image upload, and multi-turn.
- [ ] **TOOL-05**: Crate is published to crates.io with a valid `Cargo.toml` manifest and LICENSE file.

## v2 Requirements

### Auth

- **AUTH-V2-01**: OAuth / refresh-token flow as an alternative to cookie strings.
- **AUTH-V2-02**: Pluggable credential cache with TTL and encryption.

### Media

- **MEDIA-V2-01**: Resumable upload with explicit chunk size control.
- **MEDIA-V2-02**: URL-based media attachments (Google Drive / Photos integration deferred).

### Protocol

- **PROTO-V2-01**: Schema-aware WIZ payload validation before sending.
- **PROTO-V2-02**: Automatic protocol drift detection from live HAR captures.

### Advanced

- **ADV-V2-01**: Batch / async tool execution with parallel tool calls.
- **ADV-V2-02**: Conversation branching and history pruning.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Official REST / Vertex AI client | This SDK intentionally targets the undocumented web frontend protocol. |
| Real-time voice / video calls | Requires WebRTC or a different transport; not a chat SDK concern. |
| Mobile platform bindings | Out of scope for a Rust crate; could be a separate FFI wrapper. |
| Quota / billing management | Owned by Google; SDK only wraps frontend access. |
| UI automation beyond attestation | CDP is only used for token extraction, not general browser control. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| API-01 | Phase 1 | Pending |
| API-02 | Phase 1 | Pending |
| API-03 | Phase 1 | Pending |
| API-04 | Phase 1 | Pending |
| AUTH-01 | Phase 1 | Pending |
| AUTH-02 | Phase 1 | Pending |
| AUTH-03 | Phase 1 | Pending |
| AUTH-04 | Phase 2 | Pending |
| CHAT-01 | Phase 1 | Pending |
| CHAT-02 | Phase 2 | Pending |
| CHAT-03 | Phase 1 | Pending |
| CHAT-04 | Phase 2 | Pending |
| CHAT-05 | Phase 1 | Pending |
| MEDIA-01 | Phase 1 | Pending |
| MEDIA-02 | Phase 3 | Pending |
| MEDIA-03 | Phase 4 | Pending |
| PROTO-01 | Phase 2 | Pending |
| PROTO-02 | Phase 2 | Pending |
| PROTO-03 | Phase 3 | Pending |
| PROTO-04 | Phase 2 | Pending |
| REL-01 | Phase 1 | Pending |
| REL-02 | Phase 2 | Pending |
| REL-03 | Phase 2 | Pending |
| REL-04 | Phase 3 | Pending |
| OBS-01 | Phase 3 | Pending |
| OBS-02 | Phase 3 | Pending |
| OBS-03 | Phase 4 | Pending |
| ADV-01 | Phase 5 | Pending |
| ADV-02 | Phase 4 | Pending |
| ADV-03 | Phase 5 | Pending |
| TOOL-01 | Phase 1 | Pending |
| TOOL-02 | Phase 1 | Pending |
| TOOL-03 | Phase 1 | Pending |
| TOOL-04 | Phase 1 | Pending |
| TOOL-05 | Phase 6 | Pending |

**Coverage:**
- v1 requirements: 34 total
- Mapped to phases: 34
- Unmapped: 0 ✓

---
*Requirements defined: 2026-08-08*
*Last updated: 2026-08-08 after initial definition*
