---
phase: 17-streamgenerate-slot-hardening
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/proto/indices.rs
  - src/proto/slots.rs
autonomous: true
requirements:
  - SLOT-01
  - SLOT-02
  - SLOT-03
  - SLOT-04
  - QUAL-01
  - QUAL-02
  - QUAL-03
  - QUAL-04
  - QUAL-05
  - QUAL-06
must_haves:
  truths:
    - Every non-null slot used by the SDK in `src/proto/slots.rs` is referenced by a named constant from `src/proto/indices.rs`.
    - No raw `inner[N]` or `slots[N]` assignments exist in production code inside `src/proto/slots.rs`.
    - All new and renamed constants have doc comments citing the observed HAR value and semantic role.
    - A regression gate fails the test suite if raw numeric slot assignments are reintroduced.
    - `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo doc --no-deps` pass without new warnings.
    - Public API signatures and external behavior are unchanged.
  artifacts:
    - src/proto/indices.rs
    - src/proto/slots.rs
  key_links:
    - src/proto/indices.rs `builder` module ↔ `src/proto/slots.rs` `build_inner_req_list` and `build_fallback_base`
    - Regression test ↔ production source in `src/proto/slots.rs`
---

<objective>
Replace every raw numeric slot index in `src/proto/slots.rs` with named constants defined in `src/proto/indices.rs`, rename legacy misleading constants to match HAR-observed semantics, and add a regression gate that prevents raw numeric slot assignments from reappearing in the production builder code.

Purpose: Close the magic-number gap left in v0.3 so the 97-slot StreamGenerate builder is maintainable, auditable, and protected against silent drift.
Output: Refactored `src/proto/indices.rs` and `src/proto/slots.rs` with a passing test suite and regression gate.
</objective>

<execution_context>
@/home/vitaly/.config/opencode/gsd-core/workflows/execute-plan.md
@/home/vitaly/.config/opencode/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/17-streamgenerate-slot-hardening/SPEC.md
@.planning/REQUIREMENTS.md
@.opencode/skills/spike-findings-gemini-sdk/SKILL.md
@.opencode/skills/spike-findings-gemini-sdk/references/protocol.md
@src/proto/indices.rs
@src/proto/slots.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add and rename named slot constants in src/proto/indices.rs</name>
  <files>src/proto/indices.rs</files>
  <read_first>
    - src/proto/indices.rs
    - src/proto/slots.rs
    - .opencode/skills/spike-findings-gemini-sdk/references/protocol.md
    - .planning/phases/17-streamgenerate-slot-hardening/SPEC.md
  </read_first>
  <action>
    Update the `builder` module in `src/proto/indices.rs` to:
    - Rename legacy constants per SPEC.md:
      - `SLOT_CONTINUATION_FLAG` → `SLOT_NEW_DIALOG_FLAG` (value 6)
      - `SLOT_CATEGORY` → `SLOT_REQUEST_MODE` (value 7)
      - `SLOT_REQUEST_UUID` → `SLOT_PROTOCOL_VERSION` (value 10)
      - `SLOT_FRESH_FLAG` → `SLOT_PROTOCOL_SUBVERSION` (value 11)
      - `SLOT_THINKING_FLAG` → `SLOT_MODE_PICKER` (value 41)
      - `SLOT_CONVERSATION_TYPE` → `SLOT_FRESH_CONVERSATION_FLAG` (value 96)
    - Add new constants for previously raw indices:
      - `SLOT_TURN_COUNTER` = 17
      - `SLOT_TURN_COUNTER_MODE` = 18
      - `SLOT_STREAMING_FLAG` = 27
      - `SLOT_TOOL_EXECUTION_MODE` = 53
      - `SLOT_REQUEST_UUID` = 59
      - `SLOT_EMPTY_CONTEXT_LIST` = 61
      - `SLOT_UNUSED_PLACEHOLDER` = 66
      - `SLOT_RESPONSE_VERSION` = 68
      - `SLOT_CANDIDATE_COUNT` = 79
      - `SLOT_SAFETY_FILTER_LEVEL` = 91
    - Preserve existing unchanged constants: `SLOT_PROMPT` (0), `SLOT_LANGUAGE` (1), `SLOT_CONVERSATION_STATE` (2), `SLOT_WAA_TOKEN` (3), `SLOT_NONCE` (4), `SLOT_REQUEST_CATEGORY` (30), `SLOT_MODE_PICKER` (41), `SLOT_TOOL_DECLARATIONS` (89), `SLOT_THINKING_LEVEL` (80), `SLOT_FRESH_CONVERSATION_FLAG` (96).
    - Add doc comments to each new or renamed constant citing the HAR-observed value and semantic role as listed in SPEC.md (e.g. `SLOT_TURN_COUNTER` fresh `[[0]]`, continuation `[[1]]`). For uncertain semantics, note "HAR-observed value" and avoid inventing certainty.
    - Keep constants `pub const` inside the `builder` module; do not change visibility.
    - Do not add any code fences or inline implementations in doc comments.
  </action>
  <verify>
    <automated>cargo check 2>&1 | tail -n 5</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c 'SLOT_NEW_DIALOG_FLAG\|SLOT_REQUEST_MODE\|SLOT_PROTOCOL_VERSION\|SLOT_PROTOCOL_SUBVERSION\|SLOT_MODE_PICKER\|SLOT_FRESH_CONVERSATION_FLAG' src/proto/indices.rs` returns at least 6.
    - `grep -cE 'SLOT_TURN_COUNTER|SLOT_TURN_COUNTER_MODE|SLOT_STREAMING_FLAG|SLOT_TOOL_EXECUTION_MODE|SLOT_REQUEST_UUID|SLOT_EMPTY_CONTEXT_LIST|SLOT_UNUSED_PLACEHOLDER|SLOT_RESPONSE_VERSION|SLOT_CANDIDATE_COUNT|SLOT_SAFETY_FILTER_LEVEL' src/proto/indices.rs` returns at least 10.
    - `cargo check` succeeds with no errors.
  </acceptance_criteria>
  <done>
    All legacy constants are renamed, all new constants exist with HAR-cited doc comments, and the crate compiles.
  </done>
</task>

<task type="auto">
  <name>Task 2: Refactor src/proto/slots.rs to use named constants</name>
  <files>src/proto/slots.rs</files>
  <read_first>
    - src/proto/slots.rs
    - src/proto/indices.rs
    - .planning/phases/17-streamgenerate-slot-hardening/SPEC.md
  </read_first>
  <action>
    Refactor `src/proto/slots.rs` so production code uses only named constants:
    - Replace raw `inner[18]`, `inner[27]`, `inner[53]`, `inner[59]`, `inner[61]`, `inner[66]`, `inner[68]`, `inner[79]`, `inner[91]` in `build_inner_req_list` with the corresponding new constants from `src/proto/indices.rs`.
    - Replace raw `slots[17]` in `build_fallback_base` with `SLOT_TURN_COUNTER`.
    - Replace legacy names used in `src/proto/slots.rs` with their renamed counterparts:
      - `SLOT_CONTINUATION_FLAG` → `SLOT_NEW_DIALOG_FLAG`
      - `SLOT_CATEGORY` → `SLOT_REQUEST_MODE`
      - `SLOT_REQUEST_UUID` (slot 10) → `SLOT_PROTOCOL_VERSION`
      - `SLOT_FRESH_FLAG` → `SLOT_PROTOCOL_SUBVERSION`
      - `SLOT_THINKING_FLAG` → `SLOT_MODE_PICKER`
      - `SLOT_CONVERSATION_TYPE` → `SLOT_FRESH_CONVERSATION_FLAG`
    - Leave test code as-is for now; raw numeric indexing in tests is acceptable and will be addressed separately.
    - Do not change any values, JSON shapes, control flow, or public signatures.
    - Run `cargo test --all-targets` and `cargo clippy --all-targets -- -D warnings` and fix any failures introduced by the refactor.
  </action>
  <verify>
    <automated>cargo test --all-targets 2>&1 | tail -n 10</automated>
    <automated>cargo clippy --all-targets -- -D warnings 2>&1 | tail -n 10</automated>
  </verify>
  <acceptance_criteria>
    - `grep -v '^#' src/proto/slots.rs | grep -cE 'inner\[[0-9]+\]|slots\[[0-9]+\]'` equals 0 (only test code is excluded by the comment filter; if tests still contain raw indices, verify that no raw index appears outside `#[cfg(test)]` blocks).
    - `cargo test --all-targets` passes.
    - `cargo clippy --all-targets -- -D warnings` passes.
  </acceptance_criteria>
  <done>
    Production builder code uses only named slot constants, all tests pass, and clippy is clean.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Add regression gate against raw numeric slot assignments</name>
  <files>src/proto/slots.rs</files>
  <read_first>
    - src/proto/slots.rs
    - src/proto/indices.rs
    - .planning/REQUIREMENTS.md
  </read_first>
  <behavior>
    - The regression gate must scan the production portion of `src/proto/slots.rs` and fail if any line matches `inner[N]` or `slots[N]` where N is a numeric literal.
    - It must ignore comments, string literals, and code inside `#[cfg(test)]` blocks.
    - It must produce a clear assertion message listing any offending lines.
  </behavior>
  <action>
    Add a regression gate inside the existing `#[cfg(test)]` module in `src/proto/slots.rs`:
    - Implement a test named `no_raw_slot_indices_in_production_code` that reads the current source file via `include_str!(file!())`.
    - Parse the file content line by line. Track whether the current line is inside a `#[cfg(test)]` block (e.g. after a line containing `#[cfg(test)]` and before the matching closing brace depth returns to the module level).
    - For each production line, check for the regex patterns `inner\s*\[\s*\d+\s*\]` and `slots\s*\[\s*\d+\s*\]`. Ignore matches inside line comments (`//`) or within string literals if feasible; a simple exclusion of lines where the match appears after `//` is sufficient.
    - Collect any offending lines and assert the collection is empty, printing them on failure.
    - Run `cargo test --all-targets` to confirm the gate passes with the current refactored code.
    - Then deliberately introduce a raw `inner[99] = json!(0);` in a production function, verify the gate test fails, remove the temporary violation, and confirm the gate passes again.
    - Do not leave the temporary violation in the committed code.
  </action>
  <verify>
    <automated>cargo test --all-targets no_raw_slot_indices_in_production_code 2>&1 | tail -n 10</automated>
  </verify>
  <acceptance_criteria>
    - The test `no_raw_slot_indices_in_production_code` exists in `src/proto/slots.rs` and passes on the refactored code.
    - Inserting a raw `inner[99] = json!(0);` in production code causes the test to fail with the offending line in the output.
    - After removing the temporary violation, `cargo test --all-targets` passes.
  </acceptance_criteria>
  <done>
    Regression gate is present, detects raw numeric slot assignments in production code, and the suite passes.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Source code → Test suite | The regression gate is a self-check inside the crate; no external input crosses this boundary. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-17-01 | Tampering | `src/proto/slots.rs` production code reintroduces raw slot indices | medium | mitigate | Regression gate `no_raw_slot_indices_in_production_code` fails the build if raw numeric indices appear outside `#[cfg(test)]` blocks. |
| T-17-02 | Information Disclosure | Renamed constants could leak internal semantics in rustdoc | low | accept | Constants are `pub` within a `pub mod builder` already; renaming does not change visibility. Doc comments describe only observed HAR values, not secrets. |
| T-17-03 | Denial of Service | Regression gate parse logic panics on malformed source | low | mitigate | Gate uses simple regex scanning; avoid unwrap on file read by using `include_str!` at compile time. |
</threat_model>

<verification>
- `cargo test --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
- `cargo doc --no-deps` passes without new warnings.
- `grep -v '^#' src/proto/slots.rs | grep -cE 'inner\[[0-9]+\]|slots\[[0-9]+\]'` equals 0.
- Regression gate `no_raw_slot_indices_in_production_code` passes on refactored code and fails when a raw production index is temporarily inserted.
</verification>

<success_criteria>
- All raw numeric slot indices removed from production code in `src/proto/slots.rs`.
- All actively used slots have named constants in `src/proto/indices.rs` with HAR-cited doc comments.
- Legacy misleading constant names renamed per SPEC.md.
- Regression gate active and green.
- `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo doc --no-deps` all pass.
- No public API changes.
</success_criteria>

<output>
Create `.planning/phases/17-streamgenerate-slot-hardening/17-01-SUMMARY.md` when done.
</output>

## Artifacts this phase produces

- `src/proto/indices.rs`: renamed and expanded `builder` module constants.
  - Renamed symbols: `SLOT_NEW_DIALOG_FLAG`, `SLOT_REQUEST_MODE`, `SLOT_PROTOCOL_VERSION`, `SLOT_PROTOCOL_SUBVERSION`, `SLOT_MODE_PICKER`, `SLOT_FRESH_CONVERSATION_FLAG`.
  - New symbols: `SLOT_TURN_COUNTER`, `SLOT_TURN_COUNTER_MODE`, `SLOT_STREAMING_FLAG`, `SLOT_TOOL_EXECUTION_MODE`, `SLOT_REQUEST_UUID`, `SLOT_EMPTY_CONTEXT_LIST`, `SLOT_UNUSED_PLACEHOLDER`, `SLOT_RESPONSE_VERSION`, `SLOT_CANDIDATE_COUNT`, `SLOT_SAFETY_FILTER_LEVEL`.
- `src/proto/slots.rs`: refactored `build_inner_req_list` and `build_fallback_base` using named constants only.
- `src/proto/slots.rs` test module: regression gate `no_raw_slot_indices_in_production_code`.
