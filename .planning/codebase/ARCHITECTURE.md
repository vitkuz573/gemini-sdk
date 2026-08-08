<!-- refreshed: 2026-08-08 -->
# Architecture

**Analysis Date:** 2026-08-08

## System Overview

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                          Public API Layer                                │
│   `GeminiClient`  │  `ChatBuilder`  │  `ChatMessage` / `Conversation`  │
│   `src/client.rs`    `src/client.rs`       `src/chat.rs`                │
├─────────────────────────────────────────────────────────────────────────┤
│                          Protocol Layer                                  │
│   `src/proto/mod.rs`  │  `src/proto/slots.rs`  │  `src/proto/parser.rs` │
│   body builders       │  97-slot request list   │  response parsing      │
├─────────────────────────────────────────────────────────────────────────┤
│                          Session & Auth Layer                            │
│   `src/auth.rs` (cookies / credentials)                                  │
│   `src/session.rs` (WIZ-global-data extraction)                          │
│   `src/retry.rs` (exponential backoff)                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                          Transport Layer                                 │
│   `reqwest` HTTP client, `tokio` runtime                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                          External Services                               │
│   gemini.google.com  │  waa-pa.clients6.google.com  │  push.clients6...  │
└─────────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| `GeminiClient` | Main entry point; owns HTTP client, session state, and config | `src/client.rs` |
| `ChatBuilder` | Fluent builder for sending single-turn or multi-turn messages | `src/client.rs` |
| `auth` | Parse, validate, redact, and serialize Google cookies / SAPISIDHASH | `src/auth.rs` |
| `session` | Extract tokens, build labels, session ids, push ids, and WAA context from `/app` HTML | `src/session.rs` |
| `chat` | High-level chat types: messages, content parts, generation config, conversation state | `src/chat.rs` |
| `models` | Model category enum and model-list parsing metadata | `src/models.rs` |
| `proto` | WIZ protocol body builders, 97-slot request list, response parsers | `src/proto/mod.rs`, `src/proto/slots.rs`, `src/proto/parser.rs` |
| `upload` | Resumable upload to `push.clients6.google.com` for inline images | `src/upload.rs` |
| `retry` | Exponential-backoff retry wrapper around `reqwest` calls | `src/retry.rs` |
| `errors` | Strongly-typed SDK error enum with transient detection | `src/errors.rs` |
| `attestation` (feature) | Headless-Chrome CDP payload capture for authentic browser tokens | `src/attestation.rs` |

## Pattern Overview

**Overall:** Builder + async client with shared internal state.

**Key Characteristics:**
- `GeminiClient` is cheaply cloneable via `Arc<Inner>`; clones share session state.
- Fluent `ChatBuilder` configures model category and generation config before sending.
- Session parameters are lazily initialized from `/app` HTML on first use.
- Request bodies are constructed as raw JSON arrays mirroring the undocumented WIZ protobuf layout.
- Response parsing is defensive, scanning multiple possible shapes and indices.

## Layers

**Public API Layer:**
- Purpose: ergonomic surface for consumers.
- Location: `src/lib.rs`, `src/client.rs`, `src/chat.rs`.
- Contains: `GeminiClient`, `ChatBuilder`, `ChatMessage`, `Conversation`, `ChatResponse`, `ModelCategory`, public re-exports.
- Depends on: protocol, auth, session, upload, retry, errors.
- Used by: examples, integration tests, benchmarks.

**Protocol Layer:**
- Purpose: translate high-level requests into the exact byte/JSON shapes expected by Gemini.
- Location: `src/proto/mod.rs`, `src/proto/slots.rs`, `src/proto/parser.rs`.
- Contains: form-body builders, 97-slot `inner_req_list` construction, WIZ response parsing, model-list parsing.
- Depends on: chat types, models, errors.
- Used by: client, upload, tests.

**Session & Auth Layer:**
- Purpose: maintain authentication material and per-session tokens.
- Location: `src/auth.rs`, `src/session.rs`, `src/retry.rs`.
- Contains: `Credentials`, `Cookies`, `SessionState`, consent extraction, exponential backoff.
- Depends on: reqwest cookie types, sha1, base64, tokio sync.
- Used by: client, upload.

**Transport Layer:**
- Purpose: execute HTTP requests and surface transport errors.
- Location: `reqwest::Client` owned in `src/client.rs` (`Inner::http`).
- Contains: connection pool, header builders, cookie header injection.
- Depends on: tokio, reqwest.
- Used by: all network-facing modules.

## Data Flow

### Primary Request Path (send a chat message)

1. Caller constructs `GeminiClient::from_cookie_header(...)` (`src/client.rs:80`).
2. Caller invokes `client.chat().send_message("...")` (`src/client.rs:172` → `src/client.rs:979`).
3. `ChatBuilder::send_message_with_content` calls `GeminiClient::generate` (`src/client.rs:254`).
4. `ensure_session` lazily loads `/app`, extracts tokens, accepts consent if needed, and runs the WAA init chain (`src/client.rs:436`).
5. `generate_raw` prepares a `PreparedRequest` (`src/chat.rs:259`) and calls `stream_generate_raw`.
6. `build_stream_generate_request` uploads inline images (`src/upload.rs:115`) and builds the 97-slot `inner_req_list` (`src/proto/slots.rs:57`).
7. `build_stream_generate_body` URL-encodes `f.req` (`src/proto/mod.rs:38`).
8. `send_with_retry` posts to `assistant.lamda.BardFrontendService/StreamGenerate` (`src/client.rs:362`).
9. `generate_raw` consumes the stream and parses the response (`src/proto/parser.rs:173`).
10. `extract_conversation_state` extracts multi-turn ids and stores them in `SessionState` (`src/client.rs:290`).

### Model Listing Path

1. `GeminiClient::list_models` builds batchexecute params and body (`src/client.rs:196`).
2. POST to `/_/BardChatUi/data/batchexecute` with RPC id `otAQ7b`.
3. `parse_model_list` strips XSSI prefix, locates the `otAQ7b` entry, and maps mode arrays to `ModelInfo` (`src/proto/parser.rs:72`).

### WAA Init Chain

1. `otAQ7b` warm-up / model list (`src/client.rs:508`).
2. `sJBwce` prerequisite (`src/client.rs:523`).
3. `Waa/Create` to obtain WAA token (`src/client.rs:536`).
4. `GetAsyncData` to obtain WAA context (`src/client.rs:542`).
5. `ESY5D` feature flags (`src/client.rs:548`).

**State Management:**
- Shared `Arc<Inner>` holds `tokio::sync::Mutex<SessionState>` and `tokio::sync::Mutex<ClientConfig>`.
- Conversation state extracted from responses is written back to `SessionState` for the next turn.
- Configuration (`language`, `max_retries`, `timeout`) is mutated through `update_config_blocking`.

## Key Abstractions

**`GeminiClient`:**
- Purpose: shared, cloneable handle to the SDK.
- Examples: `src/client.rs:45`.
- Pattern: inner-struct wrapped in `Arc`; public facade with builder methods.

**`PreparedRequest`:**
- Purpose: normalized intermediate representation between chat API and protocol builders.
- Examples: `src/chat.rs:247`.
- Pattern: struct with flattened prompt, inline images, config, and category.

**`ConversationState` / multi-turn state:**
- Purpose: carry `conversation_id`, `response_id`, `response_part_id`, and `continuation_token` across turns.
- Examples: `src/session.rs:33`, `src/proto/slots.rs:24`.
- Pattern: parsed from response → stored in session → serialized into slot 2 of the next request.

**`ContentPart`:**
- Purpose: unify text, reasoning/thinking, and image parts in chat messages and responses.
- Examples: `src/chat.rs:84`.
- Pattern: enum; only `Text` is supported for requests, while responses may include `Thinking`.

## Entry Points

**Library entry point:**
- Location: `src/lib.rs`.
- Triggers: `use gemini_sdk::...`.
- Responsibilities: module re-exports, crate documentation, lint configuration.

**Example entry points:**
- `examples/text_chat.rs` — simple text chat.
- `examples/image_chat.rs` — chat with an inline image.
- `examples/stream_chat.rs` — consume raw streaming response.
- `examples/test_attestation.rs` — CDP attestation capture.
- `examples/capture_fixtures.rs` — regenerate test fixtures from live traffic.

**Benchmark entry point:**
- `benches/slot_building.rs` — criterion benchmark for `build_inner_req_list`.

## Architectural Constraints

- **Threading:** Tokio multi-thread runtime assumed; `GeminiClient` is `Send + Sync` via `Arc` + `tokio::sync::Mutex`.
- **Global state:** No global statics; all state owned by `GeminiClient`. `SessionState::generate_reqid` uses wall-clock time.
- **Circular imports:** None detected; modules depend downward toward `errors` and `proto`.
- **Feature gating:** `attestation.rs` and the `Error::Attestation` variant are compiled only when `browser-attestation` is enabled.
- **Hard-coded constants:** API keys, default fingerprints, user-agent strings, and header values are embedded in `src/client.rs`.

## Anti-Patterns

### Large Request Construction in a Single Function

**What happens:** `GeminiClient::stream_generate_raw` and `build_stream_generate_request` combine session locking, attachment upload, header construction, query param building, and retry invocation in one long method (`src/client.rs:316`–`433`).
**Why it's wrong:** Increases cognitive load and makes unit testing individual steps difficult.
**Do this instead:** Extract smaller, pure or async helpers for request assembly (`build_stream_generate_request` already partially does this; push further).

### Silent Fallback for WAA Context

**What happens:** `ogads_get_async_data` failures fall back to `build_default_waa_context()` silently (`src/client.rs:545`).
**Why it's wrong:** Hides degraded attestation state from callers; image uploads / multi-turn may behave differently without a real context.
**Do this instead:** Log at `warn` level and expose whether the context is real or default via an internal flag, or surface it through `tracing`.

### Blocking Lock on Config Updates

**What happens:** `update_config_blocking` calls `Mutex::blocking_lock()` inside non-async builder methods (`src/client.rs:136`).
**Why it's wrong:** Panics if called inside a Tokio runtime context without a blocking thread pool; current call sites are synchronous but fragile.
**Do this instead:** Store config in a `RwLock` or return a fallible/async builder stage when runtime context is uncertain.

## Error Handling

**Strategy:** A single `Error` enum with `thiserror` derive, plus a `Result<T>` type alias.

**Patterns:**
- Transport errors from `reqwest` are converted via `#[from]` (`src/errors.rs:21`).
- JSON errors from `serde_json` are converted via `#[from]` (`src/errors.rs:38`).
- `Error::is_transient` determines retry eligibility (`src/errors.rs:74`).
- HTTP 429 / 5xx and `Transient`/`RateLimited`/`Timeout` variants are retried (`src/retry.rs`).
- API-level errors parsed from response bodies are returned as `Error::Api` with `BAD_REQUEST` status (`src/proto/parser.rs:191`).

## Cross-Cutting Concerns

**Logging:**
- `tracing::debug!` used for consent flow and WAA init diagnostics.
- No structured span propagation; library does not initialize a subscriber.

**Validation:**
- Cookie validation in `Credentials::validate` (`src/auth.rs:152`).
- Prompt non-empty check in `extract_prompt` (`src/chat.rs:185`).
- Signed-in HTML validation in `extract_signed_in_state` (`src/client.rs:919`).

**Authentication:**
- Cookie header is rebuilt from `Cookies`/`Credentials` for every request.
- `Authorization: SAPISIDHASH` is computed on demand for grpc-web calls.

---

*Architecture analysis: 2026-08-08*
