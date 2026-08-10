# Phase 05: Tools & Auto-Refresh - Pattern Map

**Mapped:** 2026-08-10
**Files analyzed:** 12
**Analogs found:** 10 / 12

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/tool.rs` | utility / provider | request-response | `src/auth.rs` (`CredentialsProvider` trait) | role-match |
| `src/metrics.rs` | utility / provider | event-driven | `src/client.rs` (`HttpHook` trait) | role-match |
| `src/errors.rs` | utility | error handling | existing `src/errors.rs` | exact |
| `src/chat.rs` | model | transform | existing `src/chat.rs` | exact |
| `src/client.rs` | controller | request-response | existing `src/client.rs` | exact |
| `src/proto/slots.rs` | utility | transform | existing `src/proto/slots.rs` | exact |
| `src/proto/parser.rs` | utility | transform | existing `src/proto/parser.rs` | exact |
| `src/lib.rs` | config | barrel exports | existing `src/lib.rs` | exact |
| `Cargo.toml` | config | dependency declaration | existing `Cargo.toml` | exact |
| `tests/tool.rs` | test | request-response | `tests/auth_provider.rs` | role-match |
| `tests/metrics.rs` | test | event-driven | `tests/tracing.rs` | role-match |
| `tests/integration_tests.rs` | test | request-response | existing `tests/integration_tests.rs` | exact |

## Pattern Assignments

### `src/tool.rs` (utility / provider, request-response)

**Analog:** `src/auth.rs` — `CredentialsProvider` trait

**Imports pattern** (lines 1-14):
```rust
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use serde::{Deserialize, Serialize};
```

**Boxed-future async trait pattern** (lines 303-321):
```rust
pub trait CredentialsProvider: Send + Sync {
    fn credentials(&self) -> Pin<Box<dyn Future<Output = crate::Result<Credentials>> + Send + '_>>;
}

impl CredentialsProvider for Credentials {
    fn credentials(&self) -> Pin<Box<dyn Future<Output = crate::Result<Credentials>> + Send + '_>> {
        let creds = self.clone();
        Box::pin(async move { Ok(creds) })
    }
}
```

**Error handling pattern** (lines 30-47):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialsError {
    MissingPsid,
    MissingPsidcc,
}

impl fmt::Display for CredentialsError { ... }
impl std::error::Error for CredentialsError {}
```

**Apply:** Define `Tool` trait with `Pin<Box<dyn Future>>` invoke method. Add `ToolError` enum with `thiserror` derive. Provide a helper struct for tool-call parts.

---

### `src/metrics.rs` (utility / provider, event-driven)

**Analog:** `src/client.rs` — `HttpHook` trait

**Imports pattern** (lines 1-14):
```rust
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
```

**Object-safe trait with `Arc<dyn ...>` pattern** (lines 35-73):
```rust
pub trait HttpHook: Send + Sync {
    fn on_request<'a>(...)
        -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

impl HttpHook for Arc<dyn HttpHook> {
    fn on_request<'a>(...)
        -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
    {
        (**self).on_request(request)
    }
}
```

**Config storage pattern** (lines 107-128):
```rust
#[derive(Clone)]
struct ClientConfig {
    http_hook: Option<Arc<dyn HttpHook>>,
    fatal_hook_errors: bool,
}
```

**Apply:** Define `MetricsRecorder: Send + Sync` with `&self` methods. Provide `NoOpMetricsRecorder` and feature-gated `OpenTelemetryRecorder`. Store `Option<Arc<dyn MetricsRecorder>>` in `ClientConfig`.

---

### `src/errors.rs` (utility, error handling)

**Analog:** `src/errors.rs`

**Error enum pattern** (lines 12-83):
```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("not signed in: {0}")]
    NotSignedIn(String),
    ...
}
```

**Apply:** Add `#[error("tool error: {0}")]` `Tool(ToolError)` variant. Keep `Error` non-exhaustive.

---

### `src/chat.rs` (model, transform)

**Analog:** `src/chat.rs`

**ContentPart enum pattern** (lines 184-197):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentPart {
    Text(String),
    Thinking(String),
    Image(ImageSource),
    Audio(AudioSource),
    Video(VideoSource),
}
```

**Builder pattern** (lines 225-231):
```rust
impl GenerationConfig {
    pub fn with_system_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.system_instruction = Some(instruction.into());
        self
    }
}
```

**Apply:** Add `ContentPart::ToolCall { name, args }` and `ContentPart::ToolResult { name, result }`. Add `tools: Option<Vec<Arc<dyn Tool>>>` to `GenerationConfig` with `with_tools`. Add `ChatBuilder::with_tools` and `with_refresh_on_auth_error`.

---

### `src/client.rs` (controller, request-response)

**Analog:** `src/client.rs`

**Config builder pattern** (lines 220-269):
```rust
pub async fn with_language(self, language: impl Into<String>) -> Self {
    let language = language.into();
    let mut config = self.inner.config.write().await;
    config.language.clone_from(&language);
    drop(config);
    self
}
```

**Session init pattern** (lines 875-905):
```rust
async fn init_session(&self) -> Result<()> {
    let body = self.fetch_app_page().await?;
    if extract_signed_in_state(&body).is_none() {
        return Err(Error::NotSignedIn(...));
    }
    let final_body = if let Some(save_url) = extract_consent_save_url(&body) {
        self.accept_consent_and_refresh(&save_url).await?
    } else { body };
    let extracted = extract_from_app_html(&final_body);
    {
        let mut session = self.inner.session.lock().await;
        session.access_token = ...;
    }
    self.run_waa_init_chain().await?;
    Ok(())
}
```

**Cookie replacement pattern** (lines 1182-1185):
```rust
{
    let mut guard = self.inner.cookies.lock().await;
    guard.merge_response_cookies(response.cookies());
}
```

**Apply:** Add `with_metrics`, `refresh_credentials`, `generate_with_tools`, and hook retry-on-auth-error inside `generate_with_conversation`. Use existing `init_session` for refresh.

---

### `src/proto/slots.rs` (utility, transform)

**Analog:** `src/proto/slots.rs`

**Slot 0 builder pattern** (lines 153-179):
```rust
fn build_slot0(prompt: &str, attachments: &[WebAttachment], system_instruction: Option<&str>) -> Value {
    let prompt = match system_instruction {
        Some(instruction) => format!("{instruction}\n{prompt}"),
        None => prompt.to_string(),
    };
    if attachments.is_empty() {
        json!([prompt, 0, null, null, null, null, 0])
    } else { ... }
}
```

**build_inner_req_list signature** (lines 58-67):
```rust
pub fn build_inner_req_list(
    request: &PreparedRequest,
    conversation_state: Option<&ConversationState>,
    browser_payload: Option<&[Value]>,
    attachments: &[WebAttachment],
    request_uuid: &str,
    language: &str,
    waa_token: Option<&str>,
    nonce: &str,
) -> Vec<Value>
```

**Apply:** Extend `PreparedRequest` to carry tool declarations. Pass tools into `build_inner_req_list` and encode them in a dedicated slot or in slot 0 metadata. Preserve existing slot shapes when tools are absent.

---

### `src/proto/parser.rs` (utility, transform)

**Analog:** `src/proto/parser.rs`

**Part extraction pattern** (lines 22-55):
```rust
fn extract_part_content(part_arr: &[Value]) -> PartContent {
    let mut content = PartContent::default();
    if let Some(chunks) = part_arr.get(PART_TEXT).and_then(|v| v.as_array()) {
        for c in chunks { ... }
    }
    if let Some(fragments) = part_arr.get(PART_THINKING).and_then(...) {
        for c in fragments { ... }
    }
    content
}
```

**Response parts accumulation** (lines 472-518):
```rust
for part in parts_json {
    let part_arr = part.as_array()?;
    let id = part_arr.get(PART_ID).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let content = extract_part_content(part_arr);
    ...
}
```

**Apply:** Add tool-call detection branch in `extract_part_content`. Return `ContentPart::ToolCall` when a candidate part contains a function-call shape. Keep text/thinking extraction unchanged when no tool shape is present.

---

### `src/lib.rs` (config, barrel exports)

**Analog:** `src/lib.rs`

**Re-export pattern** (lines 83-98):
```rust
pub use auth::{Cookies, CookieHeaderProvider, Credentials, CredentialsError, CredentialsProvider};
pub use chat::{AudioSource, ChatMessage, ChatResponse, ...};
pub use client::{GeminiClient, HttpHook};
pub use errors::{Error, Result};
```

**Apply:** Add `pub mod tool; pub mod metrics;` and re-export `Tool`, `ToolError`, `MetricsRecorder`, `NoOpMetricsRecorder`, `OpenTelemetryRecorder`. Add `metrics` feature to module docs.

---

### `Cargo.toml` (config, dependency declaration)

**Analog:** `Cargo.toml`

**Optional feature pattern** (lines 65-68):
```toml
[features]
default = []
browser-attestation = ["dep:tokio-tungstenite", "dep:serde_urlencoded", "dep:tracing-subscriber"]
capture-fixtures = ["dep:regex"]
```

**Optional dependency pattern** (line 36):
```toml
tracing-subscriber = { version = "0.3", features = ["env-filter"], optional = true }
```

**Apply:** Add `opentelemetry = { version = "0.32", optional = true }` and `metrics = ["dep:opentelemetry"]` feature.

---

### `tests/tool.rs` (test, request-response)

**Analog:** `tests/auth_provider.rs`

**Test structure pattern** (read from `tests/auth_provider.rs`):
```rust
use gemini_sdk::{...};

#[test]
fn provider_returns_credentials() {
    ...
}
```

**Apply:** Create unit tests for `Tool` trait object safety, schema passthrough, and mock tool invocation.

---

### `tests/metrics.rs` (test, event-driven)

**Analog:** `tests/tracing.rs`

**Apply:** Test no-op recorder does not panic and feature-gated OpenTelemetry recorder emits expected counter/histogram calls.

---

### `tests/integration_tests.rs` (test, request-response)

**Analog:** `tests/integration_tests.rs`

**WireMock integration pattern** (lines 194-238):
```rust
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

let mock_server = MockServer::start().await;
Mock::given(method("POST"))
    .and(path(consent_path))
    .respond_with(ResponseTemplate::new(204).append_header("Set-Cookie", "SOCS=saved-consent-value"))
    .mount(&mock_server)
    .await;
```

**Apply:** Add wiremock-based tests for `generate_with_tools` follow-up turn and `with_refresh_on_auth_error` retry path.

## Shared Patterns

### Boxed-Future Async Traits
**Source:** `src/auth.rs` (`CredentialsProvider`) and `src/client.rs` (`HttpHook`)
**Apply to:** `src/tool.rs`
```rust
fn invoke<'a>(&'a self, args: Value)
    -> Pin<Box<dyn Future<Output = Result<Value, ToolError>> + Send + 'a>>;
```

### Object-Safe Trait Stored in `ClientConfig`
**Source:** `src/client.rs` (`HttpHook`)
**Apply to:** `src/metrics.rs`
```rust
metrics_recorder: Option<Arc<dyn MetricsRecorder>>,
```

### Error Enum Extension
**Source:** `src/errors.rs`
**Apply to:** `src/tool.rs`, `src/errors.rs`
```rust
#[error("tool error: {0}")]
Tool(#[from] ToolError),
```

### Optional Cargo Feature
**Source:** `Cargo.toml`
**Apply to:** `Cargo.toml`, `src/metrics.rs`
```toml
metrics = ["dep:opentelemetry"]
```

### Builder Method Returning `Self`
**Source:** `src/chat.rs` (`GenerationConfig`)
**Apply to:** `src/chat.rs`, `src/client.rs`
```rust
pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self { self.tools = Some(tools); self }
pub async fn with_metrics(self, recorder: impl MetricsRecorder + 'static) -> Self { ... }
```

### Defensive Parsing with Fallback
**Source:** `src/proto/parser.rs` (`extract_part_content`)
**Apply to:** `src/proto/parser.rs`
```rust
// Detect tool-call shape; if not present, fall through to text/thinking.
```

## No Analog Found

Files with no close match in the codebase (planner should use RESEARCH.md patterns instead):

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/tool.rs` (tool-call round-trip orchestration) | controller | request-response | No existing function-calling flow; closest is `CredentialsProvider` but semantically different |
| `src/metrics.rs` (OpenTelemetry integration) | provider | event-driven | No existing metrics/Observability integration beyond `tracing` |

## Metadata

**Analog search scope:** `src/`, `tests/`, `Cargo.toml`
**Files scanned:** ~20
**Pattern extraction date:** 2026-08-10
