# Phase 5: Tools & Auto-Refresh - Context

**Gathered:** 2026-08-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Add function calling and reduce manual auth maintenance. Scope covers ADV-01, ADV-03, OBS-03.

Key outcomes:

- Tools / function calling round-trip.
- Auto cookie refresh / consent re-acquisition.
- Metrics facade for requests, retries, parse failures, attestation.

</domain>

<decisions>
## Implementation Decisions

### Tools / Function Calling (ADV-01)
- Define an async trait `Tool` with:
  - `fn name(&self) -> &str`
  - `fn schema(&self) -> serde_json::Value` (JSON Schema parameters)
  - `async fn invoke(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>`
- Avoid `async-trait` dependency; use `Pin<Box<dyn Future<...>>>` pattern consistent with `CredentialsProvider`.
- Add `ChatBuilder::with_tools(Vec<Arc<dyn Tool>>)` and `GenerationConfig::tools`.
- Add `GeminiClient::generate_with_tools(message, tools)` that:
  - Sends the prompt with tool declarations encoded in the request.
  - Parses tool-call deltas from the response (new parser support).
  - Invokes matching tools asynchronously.
  - Sends a follow-up turn containing tool results and returns the final model response.
- Support parallel tool calls when the response contains multiple calls.
- Add `ToolError` enum that maps to `Error::Tool`.

### Auto Cookie Refresh (ADV-03)
- Add `GeminiClient::refresh_credentials<P: CredentialsProvider>(&self, provider: P) -> Result<()>` that:
  - Fetches fresh credentials from the provider.
  - Replaces `self.inner.cookies` with the new cookies.
  - Clears and re-initializes session state.
  - Runs consent acquisition if the new session requires it.
- Does not spawn background tasks; callers schedule refresh explicitly.
- Add `ChatBuilder::with_refresh_on_auth_error(bool)` to automatically invoke a registered provider on `NotSignedIn` errors for one retry.

### Metrics (OBS-03)
- Integrate `opentelemetry` metrics.
- Add a thin `MetricsRecorder` trait that wraps opentelemetry counters and histograms, so the public API does not depend directly on opentelemetry types.
- Emit counters for: requests, retries, parse failures, attestation outcomes.
- Emit histograms for: request latency, retry count.
- Store `Option<Arc<dyn MetricsRecorder>>` in `ClientConfig`.
- Add `GeminiClient::with_metrics` async builder method.
- Keep default behavior a no-op recorder to avoid overhead when unused.

### the agent's Discretion
- Agent may adjust exact Tool trait shape and how tool calls are encoded in slot 0.
- Agent may decide whether to implement full OpenAI-style tool schema or a minimal Google-web-frontend mapping.
- Agent may choose whether opentelemetry is an optional feature or a default dependency based on dependency cost.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/client.rs` — `GeminiClient`, `ChatBuilder`, request flow.
- `src/chat.rs` — `ChatMessage`, `ContentPart`, `GenerationConfig`, `Conversation`.
- `src/proto/parser.rs` — response parsing; can be extended for tool-call parts.
- `src/proto/slots.rs` — `build_slot0`, `build_inner_req_list`.
- `src/auth.rs` — `CredentialsProvider` pattern with boxed futures.
- `src/session.rs` — `SessionState`, consent handling.
- `src/errors.rs` — `Error` enum.

### Established Patterns
- Async trait avoided via `Pin<Box<dyn Future>>`.
- Builder methods on `ClientConfig` return `Self`.
- Public API additions re-exported from `src/lib.rs`.
- Errors are strongly typed with `thiserror`.
- Protocol changes isolated in `src/proto/`.

### Integration Points
- `Tool` schema → `build_slot0` / `build_inner_req_list`.
- Tool-call parser → `ContentPart::ToolCall` → `Tool::invoke`.
- `refresh_credentials` → `self.inner.cookies`, `self.inner.session`.
- `MetricsRecorder` → `ClientConfig` → emit at request/retry/parse/attestation boundaries.

</code_context>

<specifics>
## Specific Ideas

- Encode tool declarations as a JSON array in slot 0 metadata or a dedicated slot.
- Parse tool calls by detecting a distinct content part shape; add `ContentPart::ToolCall { name, args }` and `ContentPart::ToolResult { name, result }`.
- For metrics, use `opentelemetry::metrics::Meter` and define instrument names under a `gemini_sdk.` prefix.

</specifics>

<deferred>
## Deferred Ideas

- Batch / parallel tool execution with parallel tool calls deferred to v2.
- OAuth / refresh-token flow (AUTH-V2-01) deferred to v2.
- Pluggable credential cache with TTL and encryption (AUTH-V2-02) deferred to v2.
- Publish to crates.io (TOOL-05) deferred to Phase 6.

</deferred>
