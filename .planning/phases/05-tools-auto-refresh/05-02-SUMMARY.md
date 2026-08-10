# Phase 5, Plan 2 Summary: Tool Wiring, Slot Encoding, Parser Extraction, and Round-Trip

**Phase:** 05-tools-auto-refresh
**Plan:** 05-02
**Status:** Completed
**Date:** 2026-08-10

## Objective
Wire tool declarations into request building, parse tool calls from responses, and implement the round-trip `generate_with_tools` flow on top of the trait contracts from Plan 05-01.

## Files Changed
- `src/chat.rs`
  - Added `ContentPart::ToolCall` and `ContentPart::ToolResult` variants.
  - Added `tools` and `max_tool_turns` fields to `GenerationConfig` with `with_tools` / `with_max_tool_turns` builders.
  - Extended `PreparedRequest` with `tools: Option<Vec<Arc<dyn Tool>>>` and `refresh_on_auth_error: bool`.
  - Updated `extract_prompt` and `prepare_request` match arms.
- `src/proto/indices.rs`
  - Added `SLOT_TOOL_DECLARATIONS = 89`.
  - Added `PART_FUNCTION_CALL = 7`.
- `src/proto/slots.rs`
  - `build_inner_req_list` now encodes tool declarations in slot 89 when `request.tools` is present.
  - Added `build_tool_declarations` helper.
  - Updated existing tests for the new `PreparedRequest` fields.
  - Added tests for slot 89 with and without tools.
- `src/proto/parser.rs`
  - Extended `PartContent` and `extract_part_content` to detect tool-call shapes at index 7.
  - `parse_response_parts` now yields `ContentPart::ToolCall` entries.
  - Updated all `ContentPart` match arms.
- `src/client.rs`
  - Added `ChatBuilder::with_tools` and `ChatBuilder::with_refresh_on_auth_error`.
  - Added `GeminiClient::generate_with_tools` with recursion cap (default 5), parallel tool invocation, follow-up turns, and `ToolError::NotFound` for unregistered tools.
  - Updated `ChatBuilder` struct and constructor sites.
- `tests/integration_tests.rs`
  - Added `generate_with_tools_round_trip` and `parser_extracts_tool_call_from_wiz_frame` tests.
- `tests/proto_tests.rs`
  - Refactored `PreparedRequest` construction to a helper; updated all struct literals.
- `benches/slot_building.rs`
  - Updated `PreparedRequest` literal for new fields.

## Verification
- `cargo test --all-targets` — passed
- `cargo clippy --all-targets -- -D warnings` — passed
- `cargo doc --no-deps` — passed

## Commit
`feat(tools): wire tool declarations, parse tool calls, add generate_with_tools (05-02)`

## Notes
- Tool declarations are placed in slot 89 as a wrapped JSON array to preserve the existing slot-0 shape when no tools are present.
- Tool calls are parsed from candidate part index 7 (`PART_FUNCTION_CALL`) using the shape `[[name, args], ...]`.
- `generate_with_tools` stops after `max_tool_turns` (default 5) even if the model keeps requesting tool calls.
