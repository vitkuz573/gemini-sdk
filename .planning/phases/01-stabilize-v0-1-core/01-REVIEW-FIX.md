---
phase: 01-stabilize-v0-1-core
fixed_at: 2026-08-09T16:21:48Z
review_path: .planning/phases/01-stabilize-v0-1-core/01-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 7
skipped: 1
status: partial
---

# Phase 01: Code Review Fix Report

**Fixed at:** 2026-08-09T16:21:48Z
**Source review:** `.planning/phases/01-stabilize-v0-1-core/01-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 8
- Fixed: 7
- Skipped: 1

## Fixed Issues

### CR-01: `capture_fixtures` example fails to compile after `ModelInfo` fields were privatized

**Files modified:** `examples/capture_fixtures.rs`
**Commit:** 5229179
**Applied fix:** Replaced direct field access on `ModelInfo` with the public accessor methods `id()`, `title()`, `description()`, `versioned_name()`, and `category_enum()`.

### CR-02: Multi-turn conversation state is discarded when using `ChatBuilder`

**Files modified:** `src/client.rs`
**Commit:** 685bf97
**Applied fix:** `ChatBuilder::send_message_with_content` now calls `generate_raw` with `self.conversation.as_ref()` and passes the parsed response through `parse_chat_response`, then appends the user message and model text to the local `Conversation`. This preserves multi-turn continuity through the session's conversation state.

### CR-03: Race between state extraction and state update in `generate` / `generate_raw`

**Files modified:** `src/client.rs`
**Commit:** 73d5764
**Applied fix:** Conversation state extraction is now centralized in `generate_raw` via a new `ingest_conversation_state` helper that updates the session. `generate` no longer re-extracts state, and streaming callers that consume `stream_generate_raw` can call `ingest_conversation_state` to keep the session current.

### WR-02: `build_waa_context_header` can produce an invalid JSON array when mutating an ogads context

**Files modified:** `src/client.rs`
**Commit:** aed7722
**Applied fix:** Added `is_valid_waa_context_array` to verify the 17-element ogads array shape and the scalar/null types at indices 4 and 15 before mutation. Invalid contexts now fall back to the default template with a logged warning. Also documented that `WAA_FINGERPRINT_DEFAULT` is a best-effort fallback.

### WR-03: `parse_response_parts` JSON bracket scanner can slice in the middle of a UTF-8 multibyte character

**Files modified:** `src/proto/parser.rs`
**Commit:** de29a7a
**Applied fix:** The bracket scanner now aligns `json_start` to the nearest char boundary before slicing, avoiding latent panics on non-ASCII response content.

### WR-04: `accept_consent_and_refresh` merges response cookies into a local clone that is immediately dropped

**Files modified:** `src/client.rs`
**Commit:** b884374
**Applied fix:** Put `Inner::cookies` behind a `std::sync::Mutex<Cookies>` so the consent flow can atomically clone, merge `Set-Cookie` headers, and write the updated cookies back to the shared client state. All cookie reads were routed through a `cookies()` helper to keep clones consistent.

### WR-05: `Conversation::add_message` chain mutates `parts` directly, bypassing builder invariants

**Files modified:** `src/chat.rs`, `src/client.rs`
**Commit:** 64686e8
**Applied fix:** Added `ChatMessage::with_part` and updated `ChatBuilder::send_message_with_images` to use it. Added doc comments on `ChatMessage::parts` and `Conversation` clarifying that the public fields are low-level escape hatches and that callers are responsible for valid roles and part types; malformed conversations will fail at send time when `extract_prompt` runs.

## Skipped Issues

### WR-01: `with_backoff` retries all `reqwest::Error`s that carry a 5xx/429 status but discards the response body needed for downstream error classification

**File:** `src/retry.rs:26-60`
**Reason:** The suggested change (return the full `Response` from the retry closure instead of `reqwest::Error`) requires redesigning the `with_backoff` signature and all call sites in `client.rs`, including `send_with_retry`, `batchexecute_rpc`, `stream_generate_raw`, and the retry unit tests. It also affects how `error_for_status()` is applied and how `BardErrorInfo` bodies are parsed in error paths. This is a medium-sized cross-file refactor with behavioral implications for retry classification and error body handling that is better reviewed and validated with additional integration tests before landing. The current retry behavior still matches the original design intent (retry transient HTTP statuses) and the issue is acknowledged.
**Original issue:** `with_backoff` returns a `reqwest::Response` only on success. When `reqwest::Error` is returned (e.g., from `error_for_status()`), the original HTTP response body is lost because the operation closure does not read it before the error is produced.

---

_Fixed: 2026-08-09T16:21:48Z_
_Fixer: the agent (gsd-code-fixer)_
_Iteration: 1_
