# Phase 2: Reliability & Protocol Hardening - Pattern Map

**Mapped:** 2026-08-10
**Files analyzed:** 8
**Analogs found:** 8 / 8

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/client.rs` | service | request-response | `src/client.rs` (existing) | exact |
| `src/errors.rs` | model | transform | `src/errors.rs` (existing) | exact |
| `src/auth.rs` | model | transform | `src/auth.rs` (existing) | exact |
| `src/chat.rs` | model | transform | `src/chat.rs` (existing) | exact |
| `src/proto/indices.rs` | config | transform | `src/models.rs` (named constants) | role-match |
| `src/proto/slots.rs` | utility | transform | `src/proto/slots.rs` (existing) | exact |
| `src/proto/parser.rs` | utility | transform | `src/proto/parser.rs` (existing) | exact |
| `tests/proto_tests.rs` | test | request-response | `tests/proto_tests.rs` (existing) | exact |
| `tests/integration_tests.rs` | test | request-response | `tests/integration_tests.rs` (existing) | exact |

## Pattern Assignments

### `src/client.rs` (service, request-response)

**Analog:** `src/client.rs` (current implementation)

**Imports pattern** (lines 1-31):
```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::debug;

use std::sync::Mutex as StdMutex;
```

**Config lock pattern** (lines 55-59):
```rust
struct Inner {
    http: Client,
    cookies: Mutex<Cookies>,
    session: Mutex<SessionState>,
    config: StdMutex<ClientConfig>,
}
```

**Builder pattern** (lines 139-158):
```rust
pub fn with_language(self, language: impl Into<String>) -> Self {
    let language = language.into();
    self.update_config_blocking(|config| {
        config.language.clone_from(&language);
    });
    self
}
```

**Cookie merge pattern** (lines 788-819):
```rust
async fn accept_consent_and_refresh(&self, save_url: &str) -> Result<String> {
    // ...post to save_url...
    {
        let mut cookies = self.cookies().await;
        cookies.merge_response_cookies(response.cookies());
        let mut guard = self.inner.cookies.lock().await;
        *guard = cookies;
    }
    self.fetch_app_page().await
}
```

**WAA chain pattern** (lines 543-625):
```rust
async fn run_waa_init_chain(&self) -> Result<()> {
    // ...
    let waa_context = self
        .ogads_get_async_data(&cookie_header, &credentials, &waa_token)
        .await
        .unwrap_or_else(|_| build_default_waa_context());
    // ...
}
```

**Error handling pattern** (lines 596-602):
```rust
.map_err(|e| Error::Transient(format!("WAA Create failed: {e}")))?;
```

### `src/errors.rs` (model, transform)

**Analog:** `src/errors.rs` (current implementation)

**Error enum pattern** (lines 12-72):
```rust
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

**Constructor pattern** (lines 84-105):
```rust
pub(crate) fn api(status: StatusCode, message: impl fmt::Display) -> Self {
    Self::Api { status, message: message.to_string() }
}
```

### `src/auth.rs` (model, transform)

**Analog:** `src/auth.rs` (current implementation)

**Cookie merge pattern** (lines 229-250):
```rust
pub(crate) fn merge_response_cookies<'a>(
    &mut self,
    cookies: impl Iterator<Item = reqwest::cookie::Cookie<'a>>,
) {
    for cookie in cookies {
        // match on known names, insert unknowns into extra
    }
}
```

### `src/chat.rs` (model, transform)

**Analog:** `src/chat.rs` (current implementation)

**GenerationConfig pattern** (lines 104-125):
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    // ...
}
```

**Builder pattern** (lines 1062-1073):
```rust
impl<'a> ChatBuilder<'a> {
    pub fn with_category(mut self, category: ModelCategory) -> Self {
        self.category = category;
        self
    }
}
```

### `src/proto/indices.rs` (config, transform)

**Analog:** `src/models.rs` (named constants)

**Constant pattern** (from `src/models.rs` lines 10-48):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ModelCategory {
    // ...
}

impl ModelCategory {
    pub fn as_enum_value(self) -> u64 {
        // ...
    }
}
```

Use plain `pub const` items grouped by builder/parser usage:
```rust
/// Slot indices for building the 97-slot StreamGenerate request list.
pub mod builder {
    pub const SLOT_PROMPT: usize = 0;
    pub const SLOT_LANGUAGE: usize = 1;
    // ...
}

/// Indices for parsing StreamGenerate response parts.
pub mod parser {
    pub const PART_TEXT: usize = 1;
    pub const PART_THINKING: usize = 37;
    // ...
}
```

### `src/proto/slots.rs` (utility, transform)

**Analog:** `src/proto/slots.rs` (current implementation)

**Slot assignment pattern** (lines 72-96):
```rust
inner[0] = build_slot0(&request.prompt, attachments);
inner[1] = json!([language]);
inner[3] = waa_token.map_or_else(|| Value::Null, |t| json!(t));
// ...
```

**Error handling pattern** (lines 192-197):
```rust
pub fn base64_decode(data: &str) -> crate::Result<Vec<u8>> {
    // ...
    .map_err(|e| crate::errors::Error::bad_request(format!("invalid base64 data: {e}")))
}
```

### `src/proto/parser.rs` (utility, transform)

**Analog:** `src/proto/parser.rs` (current implementation)

**Defensive parsing pattern** (lines 36-68):
```rust
fn extract_part_content(part_arr: &[Value]) -> PartContent {
    let mut content = PartContent::default();
    if let Some(chunks) = part_arr.get(PART_TEXT_INDEX).and_then(|v| v.as_array()) {
        // ...
    }
    // ...
}
```

**Error propagation pattern** (lines 289-325):
```rust
let main = main_entry.ok_or_else(|| Error::parse("StreamGenerate response missing main entry"))?;
```

### `tests/proto_tests.rs` (test, request-response)

**Analog:** `tests/proto_tests.rs` (current implementation)

**Fixture test pattern** (lines 16-92):
```rust
#[test]
fn parse_simple_text_response() {
    let body = include_str!("fixtures/chat_response_minimal.json");
    let response = parse_chat_response(body).unwrap();
    assert_eq!(response.text(), "Hello, world!");
}
```

### `tests/integration_tests.rs` (test, request-response)

**Analog:** `tests/integration_tests.rs` (current implementation)

**Async test pattern** (from `tests/auth_provider.rs` lines 1-30):
```rust
#[tokio::test]
async fn from_provider_reads_credentials() {
    // ...
}
```

## Shared Patterns

### Async Locking
**Source:** `src/client.rs` lines 55-59
**Apply to:** `src/client.rs`
Use `tokio::sync::Mutex` for `cookies`/`session` and switch `config` to `tokio::sync::RwLock`.

### Error Constructors
**Source:** `src/errors.rs` lines 84-105
**Apply to:** `src/errors.rs`, `src/client.rs`, `src/proto/slots.rs`, `src/proto/parser.rs`
Use typed `pub(crate) fn` constructors for `Error::Parse`, `Error::AttestationFailed`, etc.

### Cookie Merge
**Source:** `src/auth.rs` lines 229-250
**Apply to:** `src/client.rs`
Call `merge_response_cookies` on a clone, then assign back to the shared mutex guard.

### Fixture-Based Parser Tests
**Source:** `tests/proto_tests.rs` lines 16-92
**Apply to:** `tests/proto_tests.rs`
Add JSON/txt fixtures under `tests/fixtures/` and reference them with `include_str!`.

## No Analog Found

No files in this phase lack a close analog; all modifications extend existing files or use existing patterns.

## Metadata

**Analog search scope:** `src/**/*.rs`, `tests/**/*.rs`
**Files scanned:** 13
**Pattern extraction date:** 2026-08-10
