---
phase: 17-streamgenerate-slot-hardening
plan: P01
type: execute
status: complete
subsystem: proto
tags: [slots, constants, refactor, regression-gate, wiz]
dependency_graph:
  requires: []
  provides: [SLOT-01, SLOT-02, SLOT-03, SLOT-04, QUAL-01, QUAL-02, QUAL-03, QUAL-04, QUAL-05, QUAL-06]
  affects: [src/proto/indices.rs, src/proto/slots.rs]
tech_stack:
  added: []
  patterns:
    - Named HAR-backed constants in src/proto/indices.rs builder module
    - Self-referential regression gate via include_str! in src/proto/slots.rs
key_files:
  created: []
  modified:
    - src/proto/indices.rs
    - src/proto/slots.rs
decisions: []
metrics:
  duration: 22m
  completed_date: "2026-08-11"
  tasks_completed: 3
  files_modified: 2
---

# Phase 17 Plan 01: StreamGenerate Slot Hardening Summary

Refactored `src/proto/indices.rs` and `src/proto/slots.rs` so the 97-slot `StreamGenerate` builder uses named, HAR-backed constants everywhere, renamed misleading legacy constants, and added a regression gate that fails the suite if raw numeric slot assignments reappear in production code.

## What Changed

### `src/proto/indices.rs`

- Renamed legacy constants to match observed HAR semantics:
  - `SLOT_CONTINUATION_FLAG` → `SLOT_NEW_DIALOG_FLAG` (6)
  - `SLOT_CATEGORY` → `SLOT_REQUEST_MODE` (7)
  - `SLOT_REQUEST_UUID` → `SLOT_PROTOCOL_VERSION` (10)
  - `SLOT_FRESH_FLAG` → `SLOT_PROTOCOL_SUBVERSION` (11)
  - `SLOT_THINKING_FLAG` → `SLOT_MODE_PICKER` (41)
  - `SLOT_CONVERSATION_TYPE` → `SLOT_FRESH_CONVERSATION_FLAG` (96)
- Added named constants for previously raw indices:
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
- Preserved unchanged constants: `SLOT_PROMPT`, `SLOT_LANGUAGE`, `SLOT_CONVERSATION_STATE`, `SLOT_WAA_TOKEN`, `SLOT_NONCE`, `SLOT_REQUEST_CATEGORY`, `SLOT_THINKING_LEVEL`, `SLOT_TOOL_DECLARATIONS`.
- Added doc comments citing HAR-observed values and semantic roles for every new or renamed constant.

### `src/proto/slots.rs`

- Refactored `build_inner_req_list` to use named constants for slots 7, 10, 11, 18, 27, 41, 53, 59, 61, 66, 68, 79, 91, and 96.
- Refactored `build_fallback_base` to use `SLOT_TURN_COUNTER` instead of `slots[17]`.
- Added `no_raw_slot_indices_in_production_code` regression test in the `#[cfg(test)]` module:
  - Reads the source file via `include_str!("slots.rs")`.
  - Ignores lines inside the `#[cfg(test)]` block.
  - Fails if any production line matches `inner[N]` or `slots[N]` where `N` is a numeric literal.
  - Prints offending lines on failure.
- Left test assertions using raw indices intact (they are inside `#[cfg(test)]` and the gate ignores them).

## Verification

- `cargo test --all-targets` passes: **279 passed, 0 failed, 2 ignored** across 19 test targets.
- `cargo clippy --all-targets -- -D warnings` passes with no warnings.
- `cargo doc --no-deps` passes with no warnings.
- The regression gate passes on the refactored code.
- `grep -v '^#' src/proto/slots.rs | grep -cE 'inner\[[0-9]+\]|slots\[[0-9]+\]'` returns **12**, all inside the `#[cfg(test)]` module; no raw indices remain in production code.

## Commits

- `8aca03a` — `refactor(17-01): rename slot constants and add new named indices`
- `bd06a60` — `test(17-01): add raw slot index regression gate`

## Deviations from Plan

None. Plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

No new security-relevant surface was introduced beyond renaming internal constants and adding a self-check test. No threat flags.

## Self-Check: PASSED

- [x] `src/proto/indices.rs` modified
- [x] `src/proto/slots.rs` modified
- [x] Commit `8aca03a` exists
- [x] Commit `bd06a60` exists
- [x] `cargo test --all-targets` passes
- [x] `cargo clippy --all-targets -- -D warnings` passes
- [x] `cargo doc --no-deps` passes
