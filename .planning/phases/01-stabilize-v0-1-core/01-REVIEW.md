---
phase: 01-stabilize-v0-1-core
reviewed: 2026-08-09T12:30:00Z
depth: deep
files_reviewed: 16
files_reviewed_list:
  - Cargo.toml
  - README.md
  - examples/multi_turn_chat.rs
  - src/auth.rs
  - src/chat.rs
  - src/client.rs
  - src/errors.rs
  - src/lib.rs
  - src/models.rs
  - src/retry.rs
  - tests/api_stability.rs
  - tests/auth_provider.rs
  - tests/integration_tests.rs
  - tests/proto_tests.rs
  - tests/real_cookies.rs
  - tests/redaction.rs
findings:
  critical: 3
  warning: 5
  info: 2
  total: 10
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-08-09T12:30:00Z
**Depth:** deep
**Files Reviewed:** 16
**Status:** issues_found

## Summary

Reviewed the v0.1 stabilization phase for `gemini-sdk`, covering the public API surface (`auth`, `chat`, `client`, `errors`, `models`, `retry`), examples, and integration tests. The code is well-structured, extensively documented, and the core test suite passes. However, a deep review surfaced a **build-breaking regression** in the `capture-fixtures` example caused by privatizing `ModelInfo` fields, a **probable multi-turn state loss bug** in `ChatBuilder::send_message_with_content`, a **session-state update race** in `generate`/`generate_raw`, and several smaller correctness/robustness issues in retry, parsing, WAA context handling, and the `accept_consent_and_refresh` flow.

## Critical Issues

### CR-01: `capture_fixtures` example fails to compile after `ModelInfo` fields were privatized

**File:** `examples/capture_fixtures.rs:222-226`
**Issue:** In `src/models.rs` the fields of `ModelInfo` were made private and accessor methods were added (`id()`, `title()`, `description()`, `versioned_name()`, `category_enum()`). The `capture_fixtures` example was not updated and still accesses the private fields directly. This breaks `cargo check --examples --features capture-fixtures`, `cargo test --all-targets --features capture-fixtures`, and `cargo clippy --all-features --all-targets`.

```text
error[E0616]: field `id` of struct `ModelInfo` is private
error[E0616]: field `title` of struct `ModelInfo` is private
error[E0616]: field `description` of struct `ModelInfo` is private
error[E0616]: field `versioned_name` of struct `ModelInfo` is private
error[E0616]: field `category_enum` of struct `ModelInfo` is private
```

**Fix:** Replace direct field access with the public accessors:

```rust
format!(
    "[\"{}\",\"{}\",\"{}\",null,null,null,null,null,null,null,null,\"{}\",null,null,null,null,null,{}]",
    m.id(),
    m.title(),
    m.description(),
    m.versioned_name().unwrap_or(m.title()),
    m.category_enum()
)
```

### CR-02: Multi-turn conversation state is discarded when using `ChatBuilder`

**File:** `src/client.rs:1027-1036`
**Issue:** `ChatBuilder::send_message_with_content` calls `self.client.generate(&message, self.category, self.config)`. `generate` internally calls `generate_raw` with `conversation: None`, which means the prepared request will never receive the previous `ConversationState`, even when the builder was created via `continue_conversation(conversation)`. The builder holds the conversation but never passes it to `generate_raw`. As a result, slot 96 remains `1` (fresh conversation) and slot 2 stays empty, so Gemini treats every turn as a first turn and multi-turn continuity is broken.

The `generate` and `generate_raw` public methods accept an explicit `conversation` parameter, but the builder-level API ignores it.

**Fix:** Add a `send_message_with_content_and_conversation` helper (or change the existing method) so the builder passes `self.conversation.as_ref()` into `generate_raw`:

```rust
pub async fn send_message_with_content(self, message: ChatMessage) -> Result<ChatResponse> {
    let response = self
        .client
        .generate_raw(&message, self.conversation.as_ref(), self.category, self.config.clone())
        .await?;
    let parsed = parse_chat_response(&response)?;

    if let Some(mut conversation) = self.conversation {
        conversation.add_message(message);
        conversation.add_model_text(parsed.text.clone());
    }

    Ok(parsed)
}
```

Also consider whether `GeminiClient::generate` should expose a conversation-aware overload for consistency.

### CR-03: Race between state extraction and state update in `generate` / `generate_raw`

**File:** `src/client.rs:273-315`
**Issue:** Both `generate` and `generate_raw` call `stream_generate_raw`, then consume the full body, and then attempt to extract and store conversation state. However, `stream_generate_raw` itself does **not** update `session.conversation_state`. The state update is only attempted in `generate`/`generate_raw`. For `stream_generate` (the streaming public API) the caller is responsible for parsing the body, but the doc comment says "Conversation state is extracted from the consumed body by the caller or by `GeminiClient::generate_raw`" — yet there is no helper provided to do this, so streaming callers will almost certainly leave `session.conversation_state` stale.

More importantly, because `generate_raw` updates state *after* consuming the body, and `generate` also parses the body and updates state again, a caller that uses `generate_raw` directly followed by `parse_chat_response` and then later calls `continue_conversation` on a `Conversation` built from an old snapshot will have a stale session. But the bigger correctness risk is that `stream_generate_raw` builds the request using the session's `conversation_state` (line 419), updates it nowhere, and therefore a subsequent `stream_generate` call will reuse the same stale state. This contradicts the intended "multi-turn conversation state" feature.

**Fix:** Extract conversation state immediately after the request completes and before returning from `stream_generate_raw`, then store it in session state. Provide a public helper to parse streaming bodies for callers who consume the stream manually. Ensure the builder uses this updated state.

## Warnings

### WR-01: `with_backoff` retries all `reqwest::Error`s that carry a 5xx/429 status but discards the response body needed for downstream error classification

**File:** `src/retry.rs:26-60`
**Issue:** `with_backoff` returns a `reqwest::Response` only on success. When `reqwest::Error` is returned (e.g., from `error_for_status()`), the original HTTP response body is lost because the operation closure does not read it before the error is produced. Downstream code in `client.rs` then reconstructs an `Error::api` without details in some paths (e.g., `stream_generate_raw` reads body for error, but `batchexecute_rpc` reads body after retry returns). More subtly, if a transient 5xx response contains a `BardErrorInfo` code in the body, that body is never inspected by the retry layer, so the SDK may retry a permanent Gemini-side error.

**Fix:** Do not call `error_for_status()` inside the retry closure. Instead, return the full `Response` from the closure and let the SDK classify status codes and bodies after retry completes. This keeps the body available for parsing `BardErrorInfo` and producing richer errors.

### WR-02: `build_waa_context_header` can produce an invalid JSON array when mutating an ogads context

**File:** `src/client.rs:861-900`
**Issue:** If `context` is a JSON array with length >= 16, the function mutates index 15 with the UUID and index 4 with the fingerprint and returns it. If the original array is malformed or has different semantics for those indices, the header sent to Gemini will be incorrect. There is no validation that the array actually matches the expected `x-goog-ext-525001261-jspb` shape. The fallback path hardcodes the same 17-element template, but the mutation path trusts the upstream response unconditionally.

Additionally, `run_waa_init_chain` falls back to `build_default_waa_context()` when ogads fails (line 564), but the default context uses a hardcoded fingerprint that may not match the live session, which the spike findings flag as fragile (see skill requirement: WAA context must be obtained without Chrome automation).

**Fix:** Validate the ogads response array length and element types against the known header shape before mutating. If validation fails, fall back to the default template and log a warning. Document that the default fingerprint is a best-effort fallback.

### WR-03: `parse_response_parts` JSON bracket scanner can slice in the middle of a UTF-8 multibyte character

**File:** `src/proto/parser.rs:352-378`
**Issue:** The scanner finds the first `'['` and then scans character-by-character to find the balanced outer array. It then slices with `&json_line[s..e]`. `char_indices()` yields byte offsets, and slicing at those offsets is valid only if the start and end positions fall on character boundaries. While `char_indices()` guarantees this for the characters it yields, if `json_start` is not on a char boundary (possible if the line contains a multibyte character before `'['`), then `json_line` itself starts mid-character and subsequent slices may be invalid. This is a latent panic risk on non-ASCII response content.

**Fix:** Ensure `json_start` is at a char boundary before slicing, or use `line.find('[')` on the original string and then slice from a validated boundary. Prefer `serde_json` streaming or `memchr` over hand-rolled bracket counting.

### WR-04: `accept_consent_and_refresh` merges response cookies into a local clone that is immediately dropped

**File:** `src/client.rs:773-778`
**Issue:** The function creates a local clone of the client's cookies, merges `Set-Cookie` headers from the consent response into it, and then drops the clone. The client's actual `self.inner.cookies` is never updated, so the newly acquired `SOCS` cookie is lost. This defeats the "Consent / SOCS cookie auto-acquisition" feature advertised in the README and `lib.rs`.

**Fix:** Update `self.inner.cookies` directly. Because `Inner::cookies` is not behind a lock, this can be done with a clone-and-replace:

```rust
{
    let mut cookies = self.inner.cookies.clone();
    cookies.merge_response_cookies(response.cookies());
    self.inner.cookies = cookies;
}
```

Then re-fetch `/app` so the updated cookies are used for subsequent extraction.

### WR-05: `Conversation::add_message` chain mutates `parts` directly, bypassing builder invariants

**File:** `src/chat.rs:222-238`
**Issue:** `ChatBuilder::send_message_with_images` pushes `ContentPart::Image` directly into `message.parts` (line 1012) and `add_user_text`/`add_model_text` push messages into `Conversation::messages`. Because `parts` and `messages` are public, callers can construct messages with empty prompts, unsupported image URLs, or invalid role strings, and the validation in `extract_prompt` / `prepare_request` only runs at request time. This is acceptable for a low-level SDK, but the public docs claim "Multi-turn conversation state" without clarifying that callers are responsible for keeping roles consistent.

**Fix:** Consider making `ChatMessage::parts` private (or crate-public) and exposing only constructors/append helpers that enforce invariants. At minimum, document that `parts` and `messages` are low-level escape hatches and that malformed conversations will fail at send time.

## Info

### IN-01: `PreparedRequest` is publicly exported but marked `#[doc(hidden)]` and contains public fields

**File:** `src/chat.rs:254-265`, `src/lib.rs:88-89`
**Issue:** `PreparedRequest` is re-exported from the crate root (implied by the comment) and used by benchmarks and tests. Its fields are public and it is `#[doc(hidden)]`, which is an awkward API surface: downstream code can depend on it without documentation, and future field additions are breaking changes. The `api_stability.rs` test does not cover it.

**Fix:** Either fully document `PreparedRequest` and commit to its stability, or keep it crate-private and expose a dedicated `gemini_sdk::chat::PreparedRequest` only under a benchmark/test-only gate.

### IN-02: `tests/real_cookies.rs` skips image upload test silently when `GEMINI_PUSH_ID` is absent, but docs imply image uploads work with default push id

**File:** `tests/real_cookies.rs:91-94`, `src/session.rs:48-54`
**Issue:** `SessionState::effective_push_id` already falls back to a hardcoded default when `GEMINI_PUSH_ID` is unset, and the upload endpoint itself does not strictly require an env-provided push id. The test skips image upload unless `GEMINI_PUSH_ID` is set, which reduces coverage. More importantly, `real_cookies.rs` reads cookies from `/tmp/opencode/gemini_cookies.env` but the README and other tests use the `GEMINI_COOKIES` environment variable, creating a documentation/test inconsistency.

**Fix:** Unify cookie loading to use `std::env::var("GEMINI_COOKIES")` like `integration_tests.rs`, and remove the `GEMINI_PUSH_ID` skip (or document why it is required). Ensure the documented test commands in `README.md` actually exercise the upload test.
