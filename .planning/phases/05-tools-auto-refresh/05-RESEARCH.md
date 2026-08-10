# Phase 05: Tools & Auto-Refresh - Research

**Researched:** 2026-08-10
**Domain:** Rust SDK, function calling, cookie refresh, OpenTelemetry metrics
**Confidence:** HIGH

## Summary

Phase 5 extends the Gemini SDK with three orthogonal but user-facing capabilities: function calling (tools), explicit credential refresh / consent re-acquisition, and request-level metrics. All three build directly on existing SDK primitives. The codebase already uses boxed-future async traits (`CredentialsProvider`), a 97-slot `inner_req_list`, and response parsing via `parse_response_parts`. This gives us high confidence that the proposed approach in CONTEXT.md is correct and implementable.

The main integration risk is encoding tool declarations and tool-call response parts into the undocumented web-frontend protocol. The planner should reserve space in `build_inner_req_list` (slot 0 and related metadata slots) for tool arrays, and the parser should detect a new `ContentPart::ToolCall` shape without breaking existing text/thinking extraction. Because the protocol is undocumented, the implementation must be guarded by snapshot and integration tests that can be updated when live HAR fixtures change.

Cookie refresh is the least risky: `GeminiClient` already owns a `Mutex<Cookies>` and `Mutex<SessionState>`, so replacing cookies and re-running `init_session()` is mostly wiring. Metrics are also low risk if we keep the public facade minimal and use a no-op recorder by default.

**Primary recommendation:** Build tools/refresh/metrics as additive SDK features that reuse existing `client.rs`/`chat.rs`/`session.rs` machinery, hide OpenTelemetry behind a trait, and gate every new public API with unit + snapshot + integration tests.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Tool trait + invocation | SDK public API (`src/tool.rs`) | Protocol encoding (`src/proto/`) | Public contract belongs to SDK; wire format is protocol-specific |
| Tool request encoding | Protocol layer (`src/proto/slots.rs`) | Chat request builder (`src/chat.rs`) | Slot 0 / inner_req_list is owned by protocol |
| Tool response parsing | Protocol parser (`src/proto/parser.rs`) | Response type (`src/chat.rs`) | Parser knows WIZ response shapes |
| Credential refresh | Client lifecycle (`src/client.rs`) | Auth primitives (`src/auth.rs`) | Refresh mutates client session/cookies |
| Auto-retry on auth error | Request flow (`src/client.rs`) | Builder API (`src/chat.rs`) | Hooked into `generate_with_conversation` retry path |
| Metrics facade | SDK public API (`src/metrics.rs`) | Client request/parse boundaries | Public trait keeps opentelemetry optional |
| OpenTelemetry emission | Metrics internals (`src/metrics.rs`) | Cargo feature gate | Behind `metrics` feature to avoid default dependency |

## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Tools / Function Calling (ADV-01)
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

#### Auto Cookie Refresh (ADV-03)
- Add `GeminiClient::refresh_credentials<P: CredentialsProvider>(&self, provider: P) -> Result<()>` that:
  - Fetches fresh credentials from the provider.
  - Replaces `self.inner.cookies` with the new cookies.
  - Clears and re-initializes session state.
  - Runs consent acquisition if the new session requires it.
- Does not spawn background tasks; callers schedule refresh explicitly.
- Add `ChatBuilder::with_refresh_on_auth_error(bool)` to automatically invoke a registered provider on `NotSignedIn` errors for one retry.

#### Metrics (OBS-03)
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

### Deferred Ideas (OUT OF SCOPE)
- Batch / parallel tool execution with parallel tool calls deferred to v2.
- OAuth / refresh-token flow (AUTH-V2-01) deferred to v2.
- Pluggable credential cache with TTL and encryption (AUTH-V2-02) deferred to v2.
- Publish to crates.io (TOOL-05) deferred to Phase 6.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ADV-01 | Function calling / tools round-trip | Tool trait with boxed futures; slot 0 extension; new parser content parts |
| ADV-03 | Auto cookie refresh / consent re-acquisition | Reuse `Mutex<Cookies>` + `init_session()`; provider-on-retry wired into request path |
| OBS-03 | Metrics facade for requests/retries/parse/attestation | OpenTelemetry counters/histograms wrapped by SDK trait; no-op default |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `opentelemetry` | 0.32.0 | Metrics API (counter, histogram, meter) | OpenTelemetry is the de-facto observability API in Rust [VERIFIED: crates.io] |
| `serde_json` | 1.0.151 | JSON Schema and tool argument serialization | Already in Cargo.toml; no new dependency needed [VERIFIED: existing dependency] |
| `thiserror` | 1.0/2.0 | `ToolError` derivation | Already used for `Error` enum [VERIFIED: existing dependency] |
| `tokio` | 1.40+ | Async runtime for boxed-future trait methods | Already in Cargo.toml [VERIFIED: existing dependency] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `opentelemetry-prometheus` / `opentelemetry-stdout` | 0.29.0+ | Example exporters | Only in examples/tests, not in library API |
| `wiremock` | 0.6 | HTTP mocking for tool-call integration tests | Already in dev-dependencies [VERIFIED: existing dependency] |
| `tokio-test` | 0.4 | Async test runtime | Already in dev-dependencies [VERIFIED: existing dependency] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `opentelemetry` | `metrics` crate (0.24+) | Simpler API, but less ecosystem integration and fewer exporters. OpenTelemetry is preferred because the requirement explicitly names it. |
| `Pin<Box<dyn Future>>` trait | `async-trait` | `async-trait` adds a proc-macro dependency and hides the boxing; codebase already avoids it for `CredentialsProvider`. |
| JSON Schema as `serde_json::Value` | `schemars` | `schemars` is useful for derive-generated schemas, but it adds a dependency and the requirement wants a raw JSON Schema value. |

**Installation (library only):**
```bash
# Add opentelemetry behind an optional feature
cargo add --optional opentelemetry
```

**Version verification:**
```bash
$ cargo search opentelemetry --limit 1
opentelemetry = "0.32.0"    # OpenTelemetry API for Rust
$ cargo info opentelemetry 2>&1 | head -3
version: 0.32.0
license: Apache-2.0
rust-version: 1.75.0
```

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `opentelemetry` | crates.io | 6+ yrs | Very high | github.com/open-telemetry/opentelemetry-rust | OK | Approved |
| `async-trait` | crates.io | 6+ yrs | Very high | github.com/dtolnay/async-trait | OK | Not used — consistent with codebase |
| `metrics` | crates.io | 5+ yrs | High | github.com/metrics-rs/metrics | OK | Rejected in favour of explicit `opentelemetry` requirement |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
+-------------+     +--------------------+     +------------------+
| User code   |---->| ChatBuilder        |---->| GeminiClient     |
| (tools)     |     | - with_tools()     |     | - generate_*()   |
+-------------+     | - with_refresh_... |     | - refresh_creds()|
                    +--------------------+     +--------+---------+
                                                         |
                    +--------------------+               |
                    | MetricsRecorder    |<--------------+ requests/retries/parse/attestation
                    | - counter()        |               |
                    | - histogram()      |               |
                    +--------------------+               |
                                                         v
                    +--------------------+      +------------------+
                    | Tool trait         |<-----| proto parser     |
                    | - name/schema/invoke|     | - ToolCall parts |
                    +--------------------+      +--------+---------+
                                                         |
                                                         v
                                               +------------------+
                                               | build_inner_req_ |
                                               | list (slot 0 +   |
                                               | tool metadata)   |
                                               +------------------+
```

Data flow for a tool call:
1. Caller registers tools via `ChatBuilder::with_tools`.
2. `generate_with_tools` builds a `PreparedRequest` with tool declarations.
3. `build_inner_req_list` encodes tool declarations into slot 0 / metadata.
4. `StreamGenerate` returns deltas; parser detects `ContentPart::ToolCall`.
5. Client invokes all matching tools in parallel (within one async join).
6. Client builds a follow-up user message containing `ContentPart::ToolResult` parts.
7. Second `StreamGenerate` call returns final model text.

### Recommended Project Structure

```
src/
├── tool.rs          # Tool trait, ToolError, ToolCall/ToolResult helpers
├── metrics.rs       # MetricsRecorder trait + no-op + opentelemetry impl
├── chat.rs          # extend ContentPart, GenerationConfig, ChatBuilder
├── client.rs        # refresh_credentials, generate_with_tools, retry-on-auth
├── proto/
│   ├── parser.rs    # parse tool-call parts
│   └── slots.rs     # encode tool declarations
└── errors.rs        # add Error::Tool
```

### Pattern 1: Boxed-Future Async Trait
**What:** Define object-safe async traits without `async-trait` by returning `Pin<Box<dyn Future<...>>>`.  
**When to use:** All new public async traits in this SDK to stay consistent with `CredentialsProvider`.  
**Example:**
```rust
// Source: src/auth.rs
pub trait CredentialsProvider: Send + Sync {
    fn credentials(&self) -> Pin<Box<dyn Future<Output = crate::Result<Credentials>> + Send + '_>>;
}
```

### Pattern 2: Optional Feature-Gated Dependency
**What:** Keep heavy dependencies optional and expose them through a Cargo feature.  
**When to use:** `opentelemetry` should be behind a `metrics` feature so the SDK stays lightweight by default.  
**Example:**
```toml
[features]
default = []
metrics = ["dep:opentelemetry"]

[dependencies]
opentelemetry = { version = "0.32", optional = true }
```

### Pattern 3: Builder Methods Return `Self`
**What:** Configuration methods on `ChatBuilder` and `GenerationConfig` consume and return `self`.  
**When to use:** All new builder methods, matching existing `with_*` API.  
**Example:**
```rust
impl GenerationConfig {
    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools = Some(tools);
        self
    }
}
```

### Anti-Patterns to Avoid
- **Leaking OpenTelemetry types in public API:** Public methods should only see `dyn MetricsRecorder`, not `opentelemetry::metrics::*`.
- **Spawning background refresh tasks:** Context explicitly forbids this; refresh is caller-driven.
- **Tool schemas as strings:** Use `serde_json::Value` so callers can compose schemas with their preferred library.
- **Hard-coding tool shapes in parser:** Detect tool-call parts defensively and fall back to text if shape is unknown.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP retries | Custom retry loop | `crate::retry::with_backoff` | Already tested and instrumented |
| Cookie parsing / merging | Manual string split | `Cookies::from_header` / `merge_response_cookies` | Handles edge cases like `Set-Cookie` semantics |
| JSON serialization of tool args | Ad-hoc string building | `serde_json::Value` | Standard, composable, type-safe |
| OpenTelemetry meter creation | Manual global singleton | `opentelemetry::metrics::Meter` | Lifecycle managed by OpenTelemetry SDK |

## Common Pitfalls

### Pitfall 1: Slot 0 shape mismatch breaks live requests
**What goes wrong:** Tool metadata placed at the wrong index or with the wrong nesting causes `StreamGenerate` to return 400 or empty responses.  
**Why it happens:** The 97-slot format is undocumented and strict.  
**How to avoid:** Keep tool declarations in a dedicated optional slot; only mutate slot 0 prompt wrapper when tools are present. Capture snapshot tests and a real HAR fixture before release.  
**Warning signs:** Existing integration tests for plain chat still pass, but tool tests fail with 400.

### Pitfall 2: Recursive tool calls hang
**What goes wrong:** Model returns another tool call after receiving tool results, and the client loops indefinitely.  
**Why it happens:** The follow-up turn may again trigger tool calls.  
**How to avoid:** Cap tool turns in `generate_with_tools` (e.g., max 5). Document the cap and expose an escape hatch for callers who want manual turn management.  
**Warning signs:** Test with a mock that always returns a tool call; ensure it terminates.

### Pitfall 3: Metrics trait leaks `Send + Sync` bounds incorrectly
**What goes wrong:** `MetricsRecorder` is not object-safe or not `Send + Sync`, blocking use in `ClientConfig`.  
**Why it happens:** Methods with generic parameters or `self` by value break object safety.  
**How to avoid:** Use `&self` receiver, concrete `u64`/duration arguments, and explicit `Send + Sync` supertrait.  
**Warning signs:** Compiler error `the trait `MetricsRecorder` cannot be made into an object`.

### Pitfall 4: Credential refresh races with in-flight requests
**What goes wrong:** Replacing cookies/session while another request reads them causes transient failures or mismatched auth.  
**Why it happens:** `Mutex` guards are held for short periods, but request building spans multiple lock/unlock cycles.  
**How to avoid:** Take locks only to clone state; build the request outside the lock. For refresh, replace both cookies and session atomically under a single lock or document best-effort consistency.  
**Warning signs:** Flaky integration tests under concurrent refresh + chat.

### Pitfall 5: Optional opentelemetry feature still compiles when disabled
**What goes wrong:** `metrics.rs` references `opentelemetry` types unconditionally, breaking `--no-default-features` builds.  
**Why it happens:** Missing `#[cfg(feature = "metrics")]` guards.  
**How to avoid:** Put the OpenTelemetry implementation in a `#[cfg(feature = "metrics")]` module and provide a no-op recorder unconditionally.  
**Warning signs:** `cargo check --no-default-features` fails.

## Code Examples

### Tool trait and boxed future
```rust
// Source: derived from src/auth.rs CredentialsProvider pattern
use std::future::Future;
use std::pin::Pin;
use serde_json::Value;

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn schema(&self) -> Value;
    fn invoke<'a>(
        &'a self,
        args: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value, ToolError>> + Send + 'a>>;
}
```

### MetricsRecorder trait (no-op default)
```rust
// Source: proposed SDK facade
use std::sync::Arc;
use std::time::Duration;

pub trait MetricsRecorder: Send + Sync {
    fn increment_counter(&self, name: &str, attributes: &[(&str, &str)]);
    fn record_histogram(&self, name: &str, value: Duration, attributes: &[(&str, &str)]);
}

#[derive(Debug, Clone)]
pub struct NoOpMetricsRecorder;

impl MetricsRecorder for NoOpMetricsRecorder {
    fn increment_counter(&self, _name: &str, _attributes: &[(&str, &str)]) {}
    fn record_histogram(&self, _name: &str, _value: Duration, _attributes: &[(&str, &str)]) {}
}
```

### OpenTelemetry-backed recorder (feature-gated)
```rust
// Source: proposed implementation (opentelemetry 0.32)
#[cfg(feature = "metrics")]
pub struct OpenTelemetryRecorder {
    meter: opentelemetry::metrics::Meter,
}

#[cfg(feature = "metrics")]
impl MetricsRecorder for OpenTelemetryRecorder {
    fn increment_counter(&self, name: &str, attributes: &[(&str, &str)]) {
        let attrs: Vec<opentelemetry::KeyValue> = attributes
            .iter()
            .map(|(k, v)| opentelemetry::KeyValue::new(k.to_string(), v.to_string()))
            .collect();
        self.meter.u64_counter(name.to_string()).add(1, &attrs);
    }

    fn record_histogram(&self, name: &str, value: Duration, attributes: &[(&str, &str)]) {
        let attrs: Vec<opentelemetry::KeyValue> = attributes
            .iter()
            .map(|(k, v)| opentelemetry::KeyValue::new(k.to_string(), v.to_string()))
            .collect();
        self.meter
            .f64_histogram(name.to_string())
            .record(value.as_secs_f64(), &attrs);
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `async-trait` macro | `Pin<Box<dyn Future>>` object-safe traits | Phase 1+ | Keeps dependency tree small |
| Manual retry loops | `crate::retry::with_backoff` | Phase 2 | Centralised transient handling |
| Hard-coded session init | Cookie refresh + re-init | Phase 5 | Callers can recover expired sessions explicitly |

**Deprecated/outdated:**
- None for this phase.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | OpenTelemetry 0.32 is compatible with this crate's MSRV (1.80) and existing `thiserror` 1.0 | Standard Stack | Build fails or trait incompatibilities; can downgrade or feature-gate further |
| A2 | Tool declarations can be encoded as a JSON array in slot 0 or an adjacent metadata slot without altering the 97-slot count | Specific Ideas | Live requests fail; need HAR capture to correct shape |
| A3 | The existing `parse_response_parts` output can be extended with `ContentPart::ToolCall` without breaking text/thinking extraction | Specific Ideas | Parser regressions; snapshot tests catch this |

## Open Questions

1. **Exact tool-call shape in WIZ response**
   - What we know: Responses are line-delimited WIZ frames with nested arrays; text is in a predictable location.
   - What's unclear: The precise array path for a model-generated tool call (function name + arguments).
   - Recommendation: Start with a minimal unit test using a hand-crafted WIZ frame that matches the expected shape; capture a real fixture as soon as possible and update the test.

2. **Tool result encoding in the follow-up turn**
   - What we know: Slot 0 currently wraps the prompt string with optional attachments.
   - What's unclear: Whether tool results should be appended to slot 0 as text or placed in a separate slot.
   - Recommendation: Treat tool results as special `ContentPart::ToolResult` parts rendered into slot 0; keep the implementation easy to adjust when a fixture is available.

3. **Metrics attribute cardinality**
   - What we know: OpenTelemetry recommends low-cardinality attributes.
   - What's unclear: Which request attributes (model category, tool name, error kind) are safe to emit.
   - Recommendation: Keep attributes minimal in the first implementation (`operation`, `outcome`); review before Phase 6.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All | ✓ | 1.95.0 | — |
| Cargo | Dependency management | ✓ | 1.95.0 | — |
| crates.io network | `cargo search` / `cargo add` | ✓ | — | Offline index already cached |
| OpenTelemetry crate | Metrics feature | ✓ | 0.32.0 | Feature-gate or no-op |

**Missing dependencies with no fallback:** none
**Missing dependencies with fallback:** none

## Validation Architecture

> `workflow.nyquist_validation` is enabled (assumed default).

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in Rust test harness) |
| Config file | none — standard `cargo test` |
| Quick run command | `cargo test --lib tool` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ADV-01 | Tool trait is object-safe and invokable | unit | `cargo test --lib tool` | ❌ Wave 0 |
| ADV-01 | Tool declarations encode into inner_req_list | unit | `cargo test --lib proto_slots` | ❌ Wave 0 |
| ADV-01 | Parser extracts tool-call parts from WIZ frame | unit | `cargo test --lib parser` | ❌ Wave 0 |
| ADV-01 | generate_with_tools sends follow-up turn | integration | `cargo test --test integration_tests tools` | ❌ Wave 0 |
| ADV-03 | refresh_credentials replaces cookies and re-inits session | integration | `cargo test --test auth_provider refresh` | ❌ Wave 0 |
| ADV-03 | with_refresh_on_auth_error retries once on NotSignedIn | integration | `cargo test --test integration_tests refresh_retry` | ❌ Wave 0 |
| OBS-03 | MetricsRecorder no-op has zero overhead | unit | `cargo test --lib metrics` | ❌ Wave 0 |
| OBS-03 | Counter/histogram record under opentelemetry feature | unit | `cargo test --lib metrics --features metrics` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib <module>`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `src/tool.rs` + `tests/tool.rs` — covers ADV-01
- [ ] `src/metrics.rs` + `tests/metrics.rs` — covers OBS-03
- [ ] Extend `src/proto/parser.rs` tests for tool-call parts — covers ADV-01
- [ ] Extend `src/proto/slots.rs` tests for tool metadata — covers ADV-01
- [ ] Extend integration tests in `tests/integration_tests.rs` — covers ADV-01, ADV-03
- [ ] Add `metrics` feature to `Cargo.toml` — covers OBS-03

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Cookie refresh must not persist credentials in logs; reuse existing redaction |
| V3 Session Management | yes | Session re-initialization after refresh; consent re-acquisition |
| V4 Access Control | no | No role/permission changes |
| V5 Input Validation | yes | Tool arguments are `serde_json::Value` — validate in `Tool::invoke`; schemas guide callers |
| V6 Cryptography | no | No new crypto |
| V7 Error Handling | yes | Tool errors map to `Error::Tool`; no sensitive data in error messages |
| V10 Malicious Code | yes | Avoid `postinstall`/proc-macro risks by preferring well-known crates |

### Known Threat Patterns for Rust SDK

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Credential leakage in logs | Information Disclosure | `Debug` redaction already in `Credentials`; apply same to tool args that contain secrets |
| Tool injection via malicious model response | Tampering | Parse tool calls defensively; reject unknown tool names; never invoke tools not registered by caller |
| Metrics cardinality blow-up | Denial of Service | Keep attributes low-cardinality; do not emit user content or raw tool args as attributes |
| Unbounded tool-call recursion | Denial of Service | Cap tool turns in `generate_with_tools` |

## Sources

### Primary (HIGH confidence)
- Existing codebase (`src/client.rs`, `src/auth.rs`, `src/chat.rs`, `src/proto/slots.rs`, `src/proto/parser.rs`) — patterns and integration points.
- crates.io / `cargo info opentelemetry` — version 0.32.0, MSRV 1.75, Apache-2.0.

### Secondary (MEDIUM confidence)
- Project spike findings (`spike-findings-gemini-sdk`) — slot count and protocol constraints.

### Tertiary (LOW confidence)
- Exact WIZ frame shape for tool calls — will be verified with captured fixture during implementation.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — opentelemetry is standard and verified on crates.io.
- Architecture: HIGH — extends existing client/session/proto patterns.
- Pitfalls: MEDIUM — protocol encoding for tools is undocumented and needs fixture validation.

**Research date:** 2026-08-10
**Valid until:** 2026-09-10 (30 days for stable Rust ecosystem; sooner if Google changes web-frontend protocol)
