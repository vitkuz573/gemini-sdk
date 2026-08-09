# Phase 2: Reliability & Protocol Hardening - Context

**Gathered:** 2026-08-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Eliminate known fragility from CONCERNS.md and make the SDK resilient to Google's protocol drift. Scope covers REL-02, REL-03, PROTO-01, PROTO-02, PROTO-04, CHAT-02, CHAT-04, AUTH-04.

Key outcomes:

- Cookie merge persists back into client state (AUTH-04).
- Blocking locks removed from synchronous builder methods (REL-02).
- WAA / ogads failures surface typed errors (REL-03).
- WIZ slot indices centralized and parser tests expanded (PROTO-01, PROTO-02).
- Generation config and system instructions exposed (CHAT-04).
- Streaming chat yields ChatResponse chunks via futures::Stream (CHAT-02).

</domain>

<decisions>
## Implementation Decisions

### Config Updates (REL-02)
- Replace `StdMutex<ClientConfig>` with `tokio::sync::RwLock<ClientConfig>`.
- Convert synchronous `with_language` / `with_max_retries` / `with_timeout` builder methods on `GeminiClient` into `async` methods that await `write().await`.
- This is an acceptable breaking change for a pre-1.0 async SDK; aligns with Rust async conventions and fixes the Tokio panic risk.

### WAA / ogads Failures (REL-03)
- Surface WAA/ogads failures as a typed public error instead of silently falling back to synthetic context.
- Add `Error::AttestationFailed { reason: String }` variant.
- Make `run_waa_init_chain` propagate the error; callers can still proceed by constructing the client without attestation if they choose to handle it, but the SDK will not hide the failure.

### WIZ Slot Indices (PROTO-01)
- Create `src/proto/indices.rs` containing named constants for every magic index used when building and parsing the 97-slot request list and response arrays.
- Update `src/proto/slots.rs` and `src/proto/parser.rs` to import and use these constants.
- Ensure constants are grouped by builder/parser usage and documented with a short rationale.

### Parser Tests (PROTO-02)
- Add fixture files and unit tests for every documented response shape, including:
  - simple text response
  - thinking response
  - concatenated text chunks
  - first-turn meta entry
  - continuation-token entry with keys "26" and "21"
  - error-code wrapper
- Reuse existing fixtures where possible; add missing ones.

### Streaming (CHAT-02)
- Add `GeminiClient::generate_stream` that returns `Pin<Box<dyn Stream<Item = Result<ChatResponse>> + Send>>`.
- Internally consume the raw `reqwest::Response` stream line by line, parse each WIZ frame, and yield incremental `ChatResponse` deltas.
- Keep the raw `stream_generate` / `stream_generate_raw` methods available for callers that want the byte stream.

### Generation Config & System Instructions (CHAT-04)
- Extend `GenerationConfig` with `system_instruction: Option<String>`.
- Wire system instruction into slot 0 / request payload so it is sent with the prompt.
- Expose `ChatBuilder::with_system_instruction` and a client-level default via async config builder.

### Cookie Merge (AUTH-04)
- Audit `accept_consent_and_refresh`: merge response cookies into `self.inner.cookies` before refetching `/app`.
- Verify the merged clone is persisted and not dropped on the floor.

### Error Handling / Protocol Drift (PROTO-04)
- Convert remaining `.unwrap()` / `.expect()` paths in parser and slot builders into `Error::Parse` with descriptive messages.
- Ensure missing or unexpected protocol fields produce structured errors, not panics.

### the agent's Discretion
- Agent may adjust exact method names, visibility, and helper placement to match existing conventions.
- Agent may decide whether to expose attestation status as a public getter or keep it internal based on API review.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/proto/slots.rs` already builds a 97-slot `inner_req_list`; many magic indices are currently inline.
- `src/proto/parser.rs` already extracts text/thinking and handles several response shapes.
- `src/chat.rs` has `GenerationConfig`, `ChatResponse`, `ContentPart`, `Conversation`.
- `src/client.rs` has `GeminiClient`, `ChatBuilder`, session init, WAA chain, retry wrapper.
- `src/errors.rs` has a strongly typed `Error` enum with transient detection.

### Established Patterns
- Builder methods return `Self` and are chained.
- Async fallible methods return `crate::Result<T>`.
- `tokio::sync::Mutex` is already used for `cookies` and `session`; switching `config` to `tokio::sync::RwLock` is consistent.
- Protocol constants live in `src/proto/`.
- Tests use inline `#[cfg(test)]` modules plus integration tests in `tests/`.

### Integration Points
- `GeminiClient::with_*` config methods → `Inner::config`.
- `ChatBuilder::with_config` / `with_category` → `GeminiClient::generate_raw`.
- `stream_generate_raw` → `build_stream_generate_request` → `build_inner_req_list`.
- WAA chain → `session.waa_context` / `session.waa_token` / WAA header in `build_headers`.

</code_context>

<specifics>
## Specific Ideas

- Add `Error::AttestationFailed` with a clear message that includes the upstream error.
- For streaming, use `futures::stream::unfold` or `async-stream` to avoid manual state machines.
- Prefer keeping breaking changes scoped to async builder methods; document them in CHANGELOG for v0.1→v0.2.
- For system instructions, prepend to prompt in `build_slot0` or add a dedicated slot if protocol requires it; start by appending as a preamble to the prompt text for v0.1 compatibility.

</specifics>

<deferred>
## Deferred Ideas

- Configurable proxy / custom HTTP client (REL-04) deferred to Phase 3.
- Upload progress callbacks (MEDIA-02) deferred to Phase 3.
- Audio/video uploads (MEDIA-03) deferred to Phase 4.
- Tools / function calling (ADV-01) deferred to Phase 5.
- Session persistence (ADV-02) deferred to Phase 4.
- Auto cookie refresh (ADV-03) deferred to Phase 5.

</deferred>
