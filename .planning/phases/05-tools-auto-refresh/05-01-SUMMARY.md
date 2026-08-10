# Phase 5, Plan 1 Summary: Tool Trait and Public API

**Phase:** 05-tools-auto-refresh
**Plan:** 05-01
**Status:** Completed
**Date:** 2026-08-10

## Objective
Define the public `Tool` API and error type for function calling, establishing the foundational ADV-01 contracts for later wiring into request encoding, parsing, and round-trip invocation.

## Files Changed
- `src/tool.rs` (new)
  - Object-safe `Tool` trait using boxed futures (no `async-trait` dependency).
  - `ToolCall`, `ToolResult` serializable helper structs.
  - `ToolError` enum (`InvalidArgs`, `InvokeFailed`, `NotFound`).
  - `tool_declaration` helper for building tool metadata.
  - Unit tests covering invocation, object safety, and serialization.
- `src/errors.rs`
  - Added `Error::Tool(#[from] ToolError)` variant at the end of the enum.
  - Added test asserting `Error::Tool` is not transient.
- `src/lib.rs`
  - Declared `pub mod tool`.
  - Re-exported `Tool`, `ToolCall`, `ToolResult`, `ToolError`, `tool_declaration`.
- `src/upload.rs`
  - Fixed broken intra-doc link to `GeminiClient::upload_with_progress` so `cargo doc` passes.
- `tests/tool.rs` (new)
  - Integration tests for mock tools, object safety, error messages, and serialization.

## Verification
- `cargo test --lib tool` — passed
- `cargo test --test tool` — passed
- `cargo clippy --all-targets -- -D warnings` — passed
- `cargo check --no-default-features` — passed
- `cargo doc --no-deps` — passed

## Commit
`feat(tools): define Tool trait, ToolError, and public re-exports (05-01)`

## Notes
- `Tool` uses the same boxed-future pattern as `CredentialsProvider`.
- `Error::Tool` was appended to preserve existing discriminants on the `#[non_exhaustive]` enum.
