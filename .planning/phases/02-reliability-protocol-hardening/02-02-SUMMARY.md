---
phase: 02-reliability-protocol-hardening
plan: 02
---

# Plan 02 Summary: Reliability & Protocol Hardening

## Objective

Centralize WIZ protocol slot/part indices, eliminate panic paths in protocol code, and expand parser test coverage for all documented response shapes.

## What Changed

### New file

- `src/proto/indices.rs` — named constants for the 97-slot `StreamGenerate` request list (`builder::*`) and response parsing (`parser::*`). Imported via `pub mod indices;` in `src/proto/mod.rs`.

### Refactored protocol code

- `src/proto/slots.rs` — replaced raw slot indices in `build_inner_req_list`, `build_fallback_base`, and `build_slot0` with constants from `indices::builder`. `SLOT_COUNT = 97` is preserved as the only documented raw count.
- `src/proto/parser.rs` — replaced `PART_TEXT_INDEX`, `PART_THINKING_INDEX`, and numeric payload/part/conversation-id indices with constants from `indices::parser`.
- Removed production `.unwrap()` / `.expect()` paths in `src/proto/slots.rs` and `src/proto/parser.rs`; malformed protocol data now flows through `Error::Parse`. Internal unit tests were updated to use `expect(...)` only inside `#[cfg(test)]`.

### Tests and fixtures

Added fixture files:

- `tests/fixtures/chat_response_thinking.json`
- `tests/fixtures/conversation_state_first_turn.json`
- `tests/fixtures/bard_error_wrapper.json`

Added dedicated tests in `tests/proto_tests.rs`:

- `parse_simple_text_response`
- `parse_concatenated_text_response`
- `parse_thinking_response`
- `parse_chat_response_detects_bard_error_wrapper`
- `extract_first_turn_meta_token`
- `extract_continuation_token_key_21`
- `malformed_response_no_panic`

Also added an inline `malformed_response_no_panic` unit test in `src/proto/parser.rs`.

## Verification

- `cargo test --all-targets --quiet` — all passing.
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo doc --no-deps` — no warnings.

## Requirements

- PROTO-01 — centralized WIZ slot indices in `src/proto/indices.rs`.
- PROTO-02 — parser tests now cover every documented response shape with fixture files.
- PROTO-04 — remaining production `.unwrap()` / `.expect()` paths in protocol code removed; malformed data returns `Error::Parse`.
