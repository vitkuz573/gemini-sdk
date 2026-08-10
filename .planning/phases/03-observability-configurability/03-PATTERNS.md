# Phase 3: Observability & Configurability - Pattern Map

**Mapped:** 2026-08-10
**Files analyzed:** 7
**Analogs found:** 7 / 7

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/client.rs` | service | request-response | `src/client.rs` (existing constructors/config) | exact |
| `src/chat.rs` | model | transform | `src/chat.rs` (GenerationConfig) | exact |
| `src/upload.rs` | service | streaming | `src/upload.rs` (existing upload flow) | exact |
| `src/session.rs` | utility | transform | `src/session.rs` (existing extractors) | exact |
| `src/lib.rs` | config | barrel exports | `src/lib.rs` (existing re-exports) | exact |
| `tests/unit/*.rs` | test | request-response / streaming | `tests/integration_tests.rs`, `tests/proto_tests.rs` | role-match |
| `examples/*.rs` | example | request-response / streaming | `examples/image_chat.rs`, `examples/stream_chat.rs` | role-match |

## Pattern Assignments

### `src/client.rs` (service, request-response)

**Analog:** `src/client.rs` lines 58-200

**Config storage pattern** (lines 58-81):

```rust
struct Inner {
    http: Client,
    cookies: Mutex<Cookies>,
    session: Mutex<SessionState>,
    config: RwLock<ClientConfig>,
}

#[derive(Debug, Clone)]
struct ClientConfig {
    language: String,
    max_retries: usize,
    timeout: Duration,
    system_instruction: Option<String>,
}
```

**Constructor pattern** (lines 84-114):

```rust
impl GeminiClient {
    pub fn from_cookie_header(header: &str) -> Result<Self> { ... }
    pub fn from_credentials(credentials: Credentials) -> Result<Self> { ... }
    pub fn from_cookies(cookies: impl Into<Cookies>) -> Result<Self> { ... }
    pub fn from_hashmap(cookies: HashMap<String, String>) -> Result<Self> { ... }
    pub async fn from_provider<P>(provider: P) -> Result<Self> where P: CredentialsProvider + 'static { ... }

    fn with_config(cookies: Cookies, config: ClientConfig) -> Result<Self> { ... }
}
```

**Async builder pattern** (lines 144-177):

```rust
pub async fn with_language(self, language: impl Into<String>) -> Self {
    let language = language.into();
    let mut config = self.inner.config.write().await;
    config.language.clone_from(&language);
    drop(config);
    self
}
```

**Core pattern:** Add new `ClientConfig` fields and constructors exactly where existing config and constructors live. Use `Arc<dyn HttpHook + Send + Sync>` for trait object storage to match `Arc<Inner>`.

---

### `src/chat.rs` (model, transform)

**Analog:** `src/chat.rs` lines 104-128

**Config struct pattern**:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    // ...
}
```

**Builder pattern** (lines 130-136):

```rust
impl GenerationConfig {
    pub fn with_system_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.system_instruction = Some(instruction.into());
        self
    }
}
```

**Core pattern:** Add new optional public fields to `GenerationConfig` and a `with_*` builder method. Use `#[serde(skip_serializing_if = "Option::is_none")]` for any field that should not appear in JSON when unset.

---

### `src/upload.rs` (service, streaming)

**Analog:** `src/upload.rs` lines 17-119

**Resumable upload step pattern**:

```rust
pub(crate) async fn upload_file(
    client: &reqwest::Client,
    cookies: &Cookies,
    session: &SessionState,
    filename: &str,
    mime_type: &str,
    bytes: Vec<u8>,
) -> Result<String> {
    // start_upload ...
    // finalize_upload ...
}
```

**Batch attachment upload pattern** (lines 121-140):

```rust
pub(crate) async fn upload_attachments(
    client: &reqwest::Client,
    cookies: &Cookies,
    session: &SessionState,
    prepared: &crate::chat::PreparedRequest,
) -> Result<Vec<WebAttachment>> { ... }
```

**Core pattern:** Keep existing `upload_file`/`upload_attachments` paths unchanged. Add a new `upload_with_progress` streaming variant that instruments the same HTTP calls and yields `UploadEvent`.

---

### `src/session.rs` (utility, transform)

**Analog:** `src/session.rs` lines 141-223

**Primary + fallback extractor pattern**:

```rust
fn extract_snlim0e(body: &str) -> Option<String> {
    if let Some(block) = extract_wiz_global_data_block(body) {
        if let Some(token) = extract_quoted_value(block, "SNlM0e") {
            if is_valid_snlim0e(&token) { return Some(token); }
        }
    }
    if let Some(token) = extract_quoted_value(body, "SNlM0e") {
        if is_valid_snlim0e(&token) { return Some(token); }
    }
    None
}
```

**Core pattern:** Refactor each extractor to iterate a prioritized list of keys/selectors. Extract a shared helper `try_keys(block, body, &[primary, ...], validator)` to avoid repetition.

---

### `src/lib.rs` (config, barrel exports)

**Analog:** `src/lib.rs` lines 83-96

**Re-export pattern**:

```rust
pub use chat::{
    ChatMessage, ChatResponse, ContentPart, Conversation, GenerationConfig, ImageSource,
    ThinkingLevel,
};
pub use client::GeminiClient;
pub use errors::{Error, Result};
```

**Core pattern:** Add new public types (`HttpHook`, `UploadEvent`) to the appropriate `pub use` block. Add new modules only if necessary.

---

### `tests/unit/*.rs` (test, request-response / streaming)

**Analog:** `tests/integration_tests.rs` lines 1-100

**Unit test pattern**:

```rust
#[test]
fn image_source_from_bytes() {
    let image = ImageSource::from_bytes("image/png", b"fake");
    assert_eq!(image.mime_type(), Some("image/png"));
}

#[tokio::test]
async fn config_builder_async_sets_language_retries_and_timeout() { ... }
```

**Fixture loading pattern** (from `tests/proto_tests.rs`):

```rust
fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}")).unwrap()
}
```

**Core pattern:** Create focused unit test files under `tests/unit/`. Use `#[test]` for sync tests and `#[tokio::test]` for async. Load fixtures from `tests/fixtures/`.

---

### `examples/*.rs` (example, request-response / streaming)

**Analog:** `examples/image_chat.rs`, `examples/stream_chat.rs`

**Example pattern**:

```rust
#[tokio::main]
async fn main() -> gemini_sdk::Result<()> {
    let client = GeminiClient::from_cookie_header(&cookies)?;
    let response = client.chat().send_message("...").await?;
    println!("{}", response.text());
    Ok(())
}
```

**Core pattern:** Add a small example demonstrating the new hook/tracing/progress feature. Keep examples compile-only by default (no live cookies required at build time).

## Shared Patterns

### Async Trait Object Safety via Boxed Futures

**Source:** `src/auth.rs` (`CredentialsProvider`)
**Apply to:** `HttpHook` in `src/client.rs`

```rust
pub trait CredentialsProvider: Send + Sync {
    fn credentials(&self) -> Pin<Box<dyn Future<Output = Result<Credentials, CredentialsError>> + Send + '_>>;
}
```

### Error Handling with `crate::Result`

**Source:** `src/errors.rs`
**Apply to:** All new public methods

```rust
pub type Result<T> = std::result::Result<T, Error>;
```

### Tracing Debug Logging

**Source:** `src/client.rs` line 12
**Apply to:** All new internal paths

```rust
use tracing::debug;
```

### Forward Compatibility via `#[non_exhaustive]`

**Source:** `src/chat.rs` lines 11, 139, 166; `src/client.rs` line 53
**Apply to:** New public enums/structs (`UploadEvent`, `HttpHook` trait not applicable)

```rust
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ChatMessage { ... }
```

## No Analog Found

All planned files have direct analogs in the existing codebase.

## Metadata

**Analog search scope:** `src/`, `tests/`, `examples/`, `Cargo.toml`
**Files scanned:** 12
**Pattern extraction date:** 2026-08-10
