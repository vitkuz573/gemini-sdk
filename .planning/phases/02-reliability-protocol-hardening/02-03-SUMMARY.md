# Phase 02 Plan 03 Summary: Streaming Adapter & System Instructions

**Phase:** 02-reliability-protocol-hardening  
**Plan:** 03  
**Date:** 2026-08-10  
**Requirements:** CHAT-02, CHAT-04

## What Changed

- Added `GeminiClient::generate_stream` returning `Pin<Box<dyn Stream<Item = Result<ChatResponse>> + Send>>`.
  - Internally calls `stream_generate_raw`, buffers the byte stream line-by-line, parses each complete WIZ frame with `parse_response_parts`, and yields incremental `ChatResponse` chunks.
  - After the stream ends, calls `ingest_conversation_state` so multi-turn state persists.
  - Extracted the stream adapter into a private `GeminiClient::stream_responses` helper that accepts any `Stream<Item = Result<bytes::Bytes, reqwest::Error>>`, making it testable without a live HTTP client.

- Extended `GenerationConfig` with an optional `system_instruction` field.
  - Added `GenerationConfig::with_system_instruction` builder method.
  - Added `ChatBuilder::with_system_instruction` convenience method that creates a default `GenerationConfig` if needed.

- Added async `GeminiClient::with_system_instruction` as a client-level default.
  - Stored in `ClientConfig::system_instruction`.
  - Applied only when no per-turn `GenerationConfig` is provided; per-turn config always wins.

- Wired system instruction into slot 0 via `build_slot0` in `src/proto/slots.rs`.
  - When present, the instruction is prepended to the prompt text separated by a newline.
  - Slot shape is unchanged when no instruction is set.

- Added explicit `bytes = "1.7"` dependency to Cargo.toml.

## Files Modified

- `src/client.rs`
- `src/chat.rs`
- `src/proto/slots.rs`
- `tests/integration_tests.rs`
- `tests/proto_tests.rs`
- `Cargo.toml`

## Tests Added

- `client::client_tests::stream_responses_yields_text_and_ingests_state`
- `client::client_tests::stream_responses_handles_empty_body`
- `proto_tests::system_instruction_in_slot0`
- `proto_tests::no_system_instruction_preserved`
- `integration_tests::generate_stream_yields_response_chunks`
- `integration_tests::generate_stream_handles_empty_body`
- `integration_tests::client_default_system_instruction_reaches_request`
- `integration_tests::system_instruction_override_wins`

## Verification

```text
cargo test --all-targets --quiet     # passed
cargo clippy --all-targets -- -D warnings  # passed
cargo doc --no-deps                  # passed
```

Total test results: 120 passed, 0 failed, 2 ignored.

## Notes

- `stream_generate_raw` and `stream_generate` were kept unchanged as required.
- The streaming adapter treats upstream chunks as line-delimited WIZ frames. Multi-byte UTF-8 characters that straddle chunk boundaries are handled by `String::from_utf8_lossy` on each chunk and line-buffering until a newline is seen.
- The system instruction is treated as opaque text and concatenated verbatim; it is not interpreted as WIZ structure.
