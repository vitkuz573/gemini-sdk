# SPEC: Phase 17 — StreamGenerate Slot Hardening

## What This Phase Delivers

A single internal refactor that removes every raw numeric slot index from `src/proto/slots.rs` and gives every actively used `StreamGenerate` slot a named, HAR-backed constant in `src/proto/indices.rs`.

## Scope

- `src/proto/indices.rs` — add/rename constants.
- `src/proto/slots.rs` — refactor builder and fallback base to use constants.
- Tests and regression gate in `src/proto/slots.rs`.
- No public API changes; no behavioral changes.

## Naming Map

| Slot | Legacy name | New name | HAR observation |
|------|-------------|----------|-----------------|
| 0 | `SLOT_PROMPT` | `SLOT_PROMPT` | prompt + attachments tuple |
| 1 | `SLOT_LANGUAGE` | `SLOT_LANGUAGE` | `["ru"]` |
| 2 | `SLOT_CONVERSATION_STATE` | `SLOT_CONVERSATION_STATE` | single array, fresh or continuation |
| 3 | `SLOT_WAA_TOKEN` | `SLOT_WAA_TOKEN` | `!...` WAA/PoW token |
| 4 | `SLOT_NONCE` | `SLOT_NONCE` | 32-char hex |
| 6 | `SLOT_CONTINUATION_FLAG` | `SLOT_NEW_DIALOG_FLAG` | `[1]` in fresh and continuation |
| 7 | `SLOT_CATEGORY` | `SLOT_REQUEST_MODE` | `1` |
| 10 | `SLOT_REQUEST_UUID` | `SLOT_PROTOCOL_VERSION` | `1` |
| 11 | `SLOT_FRESH_FLAG` | `SLOT_PROTOCOL_SUBVERSION` | `0` |
| 17 | — | `SLOT_TURN_COUNTER` | `[[0]]` fresh, `[[1]]` continuation |
| 18 | — | `SLOT_TURN_COUNTER_MODE` | `0` |
| 27 | — | `SLOT_STREAMING_FLAG` | `1` (frontend always streams) |
| 30 | `SLOT_REQUEST_CATEGORY` | `SLOT_REQUEST_CATEGORY` | `[4]` for Auto |
| 41 | `SLOT_THINKING_FLAG` | `SLOT_MODE_PICKER` | `[1]` |
| 53 | — | `SLOT_TOOL_EXECUTION_MODE` | `0` |
| 59 | — | `SLOT_REQUEST_UUID` | uppercase UUID, matches `_reqid` query and `525005358` header |
| 61 | — | `SLOT_EMPTY_CONTEXT_LIST` | `[]` |
| 66 | — | `SLOT_UNUSED_PLACEHOLDER` | `null` |
| 68 | — | `SLOT_RESPONSE_VERSION` | `2` |
| 79 | — | `SLOT_CANDIDATE_COUNT` | `3` |
| 80 | `SLOT_THINKING_LEVEL` | `SLOT_THINKING_LEVEL` | `1` Standard, `2` Extended, `3` DeepThink |
| 89 | `SLOT_TOOL_DECLARATIONS` | `SLOT_TOOL_DECLARATIONS` | tool declarations when tools present |
| 91 | — | `SLOT_SAFETY_FILTER_LEVEL` | `0` |
| 96 | `SLOT_CONVERSATION_TYPE` | `SLOT_FRESH_CONVERSATION_FLAG` | `1` fresh, `0` continuation |

## Acceptance Criteria

1. `cargo test --all-targets` passes.
2. `cargo clippy --all-targets -- -D warnings` passes.
3. `cargo doc --no-deps` passes.
4. No raw `inner[\d+]` or `slots[\d+]` assignments remain in production code in `src/proto/slots.rs`.
5. A regression gate fails the build if raw numeric slot assignments are reintroduced.

## Risks

- Renaming constants is internal-only, but any external code re-exporting `indices::builder` could break. The module is not part of the public API surface.
- HAR-based names are our best interpretation; Google may change slot semantics without notice. Doc comments must note uncertainty where appropriate.
