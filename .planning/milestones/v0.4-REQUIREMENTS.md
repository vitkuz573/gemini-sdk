# Requirements: v0.4 StreamGenerate Slot Hardening

## Goal

Eliminate every raw numeric index in the 97-slot `StreamGenerate` request builder by introducing semantically named, HAR-backed constants. Close the magic-number gap that v0.3 left in `src/proto/slots.rs`.

## Functional Requirements

### SLOT — Slot Constants

- **[SLOT-01]**: Every non-null slot observed in the live HAR capture (`/home/vitaly/mitm.har`) must have a named constant in `src/proto/indices.rs`.
- **[SLOT-02]**: Misleading legacy constant names must be renamed to match HAR-observed semantics:
  - `SLOT_CONTINUATION_FLAG` (slot 6) → `SLOT_NEW_DIALOG_FLAG`
  - `SLOT_CATEGORY` (slot 7) → `SLOT_REQUEST_MODE`
  - `SLOT_REQUEST_UUID` (slot 10) → `SLOT_PROTOCOL_VERSION`
  - `SLOT_FRESH_FLAG` (slot 11) → `SLOT_PROTOCOL_SUBVERSION`
  - `SLOT_THINKING_FLAG` (slot 41) → `SLOT_MODE_PICKER`
  - `SLOT_CONVERSATION_TYPE` (slot 96) → `SLOT_FRESH_CONVERSATION_FLAG`
- **[SLOT-03]**: New constants must be added for previously raw indices:
  - `SLOT_TURN_COUNTER` (17)
  - `SLOT_TURN_COUNTER_MODE` (18)
  - `SLOT_STREAMING_FLAG` (27)
  - `SLOT_TOOL_EXECUTION_MODE` (53)
  - `SLOT_REQUEST_UUID` (59)
  - `SLOT_EMPTY_CONTEXT_LIST` (61)
  - `SLOT_UNUSED_PLACEHOLDER` (66)
  - `SLOT_RESPONSE_VERSION` (68)
  - `SLOT_CANDIDATE_COUNT` (79)
  - `SLOT_SAFETY_FILTER_LEVEL` (91)
- **[SLOT-04]**: Each new or renamed constant must include a doc comment citing the HAR-observed value and, where inferable, its semantic role.

## Non-Functional Requirements

### QUAL — Quality & Regression

- **[QUAL-01]**: `src/proto/slots.rs` must contain no raw `inner[\d+]` or `slots[\d+]` assignments in production code.
- **[QUAL-02]**: A regression gate must fail the test suite if raw numeric slot assignments reappear in the production builder.
- **[QUAL-03]**: `cargo test --all-targets` must pass.
- **[QUAL-04]**: `cargo clippy --all-targets -- -D warnings` must pass.
- **[QUAL-05]**: `cargo doc --no-deps` must pass without new warnings.
- **[QUAL-06]**: Public API signatures and external behavior must remain unchanged; this is an internal refactor.

## Traceability

| Requirement | Phase |
|-------------|-------|
| SLOT-01 — SLOT-04 | Phase 17: StreamGenerate Slot Hardening |
| QUAL-01 — QUAL-06 | Phase 17: StreamGenerate Slot Hardening |
