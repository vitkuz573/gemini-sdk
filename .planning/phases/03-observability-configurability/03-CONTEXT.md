# Phase 3: Observability & Configurability - Context

**Gathered:** 2026-08-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Let production users observe, meter, and tune the SDK without forking it. Scope covers PROTO-03, REL-04, OBS-01, OBS-02, MEDIA-02.

Key outcomes:

- Request/response hooks API.
- `tracing` spans across auth, request, parse, upload.
- Injectable `reqwest::Client` for connection pool control.
- Upload progress callbacks via a stream of progress events.
- Robust HTML extraction with multiple fallbacks.

</domain>

<decisions>
## Implementation Decisions

### Request/Response Hooks (OBS-01)
- Define an async trait `HttpHook` with methods `on_request(&self, &PreparedRequest) -> Result<()>` and `on_response(&self, &ChatResponse) -> Result<()>`.
- Store `Option<Arc<dyn HttpHook + Send + Sync>>` in `ClientConfig`.
- Apply hooks inside `GeminiClient::generate`, `generate_with_conversation`, and `generate_stream` after parsing.
- Keep hook errors non-fatal by default (log warning, continue), but allow opt-in fatal hook errors via a config flag if simple.

### Tracing Spans (OBS-02)
- Add `#[tracing::instrument]` to public async methods: `generate`, `generate_stream`, `list_models`, `verify_signed_in`.
- Add manual `tracing::info_span!` / `tracing::debug_span!` around upload, WAA init chain, and response parsing.
- Ensure spans do not include cookies, tokens, or prompt content at levels above debug.
- Use `tracing::debug!` for payload sizes and timing; no secret values.

### Injectable HTTP Client (REL-04)
- Add `GeminiClient::from_http_client(client: reqwest::Client, credentials: impl Into<Cookies>) -> Result<Self>` constructor.
- Store the injected client in `Inner::http` without rebuilding it.
- Add `ChatBuilder::with_http_client` is not needed; injection is at construction time.

### Upload Progress (MEDIA-02)
- Expose upload progress as a `futures::Stream<Item = Result<UploadEvent>>`.
- Define `UploadEvent` enum with `Progress { uploaded: u64, total: Option<u64> }` and `Complete { attachment: WebAttachment }`.
- Add `GeminiClient::upload_with_progress(...)` returning the stream, and wire it into `generate_stream` / `generate` via an optional progress callback in `GenerationConfig`.
- Keep existing `upload_attachments` path unchanged; add a new streaming variant.

### HTML Extraction Fallbacks (PROTO-03)
- Refactor `src/session.rs` extractors to try multiple keys/selectors for each token.
- Add fallback keys for `SNlM0e`, `cfb2h`, `FdrFJe`, `S06Grb`, `oPEP7c`, push id.
- If primary key missing, try known aliases before returning `None` / error.
- Add fixture tests for each fallback shape.

### the agent's Discretion
- Agent may adjust exact trait method signatures, event shape, and span names to match existing conventions.
- Agent may decide whether hooks are called before/after retries and whether to expose raw bytes in hook interface.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/client.rs` — `GeminiClient`, `Inner`, `ClientConfig`, builder methods, request/response flow.
- `src/upload.rs` — resumable upload helpers.
- `src/session.rs` — HTML extraction functions.
- `src/chat.rs` — `PreparedRequest`, `ChatResponse`, `GenerationConfig`.
- `src/errors.rs` — `Error`, `Result`.
- `tracing` is already a dependency.
- `futures` / `async-stream` already used for streaming adapter.

### Established Patterns
- Builder constructors return `Result<Self>`.
- Async methods return `crate::Result<T>`.
- `tokio::sync::RwLock` now used for `ClientConfig`.
- Public API additions go through `src/lib.rs` re-exports.
- Tests use inline modules and integration tests.

### Integration Points
- `from_http_client` → `with_config` path.
- `HttpHook` → `ClientConfig` → `generate` / `generate_stream`.
- `UploadEvent` stream → `upload.rs` → `build_stream_generate_request`.
- `tracing::instrument` → public async methods.
- Session extractors → `init_session` and `verify_signed_in`.

</code_context>

<specifics>
## Specific Ideas

- Use `async-trait` crate for `HttpHook` if config permits; otherwise use `Pin<Box<dyn Future>>` pattern consistent with `CredentialsProvider`.
- For upload progress, instrument the existing `start_upload`, `upload_chunk`, and `finalize_upload` steps.
- For HTML fallbacks, document each key alias in a comment with the observed source (e.g., spike finding).

</specifics>

<deferred>
## Deferred Ideas

- Audio/video uploads (MEDIA-03) deferred to Phase 4.
- Tools / function calling (ADV-01) deferred to Phase 5.
- Session persistence (ADV-02) deferred to Phase 4.
- Auto cookie refresh (ADV-03) deferred to Phase 5.
- Publish to crates.io (TOOL-05) deferred to Phase 6.

</deferred>
