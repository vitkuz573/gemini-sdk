# Phase 7 — Conversation Actions

## Scope

Add typed public APIs for conversation turn actions on the Gemini web frontend.
These actions operate on an existing conversation turn identified by its response
id and are sent via the undocumented `batchexecute` RPC id `PCck7e`.

## Requirements

- **CONVACT-01**: SDK exposes `regenerate_turn(conversation_id, response_id)` using RPC `PCck7e`.
- **CONVACT-02**: SDK exposes `rate_turn(conversation_id, response_id, rating)` using RPC `PCck7e`.
- **CONVACT-03**: SDK exposes `delete_turn(conversation_id, response_id)` using RPC `PCck7e`.
- **CONVACT-04**: Action responses are parsed into a typed `ConversationActionResult` with success/failure status.

## Key Decisions

- Reuse the existing private helper `GeminiClient::batchexecute_rpc` in `src/client.rs`; no new transport code.
- Keep the RPC id `PCck7e` as a named constant in a new `src/conversation_actions.rs` module, alongside payload builders and the response type.
- Source path for these RPCs must be `/app/{conversation_id}` per the HAR capture.
- `regenerate_turn`, `rate_turn`, and `delete_turn` are async public methods on `GeminiClient` returning `Result<ConversationActionResult>`.
- Rating is represented as a new `TurnRating` enum with variants `Good`, `Bad`, and `Neutral` to match the frontend thumbs-up / thumbs-down / undo semantics.
- `ConversationActionResult` exposes:
  - `success: bool`
  - `action: ConversationAction`
  - `response_id: String`
  - `raw: serde_json::Value` for forward compatibility.
- Response parsing must tolerate the common batchexecute anti-XSSI prefix and nested WIZ framing.
- No live network calls in CI; verify with `wiremock` fixtures (see `tests/integration_tests.rs` for the established pattern).

## Inputs from Spike

From `.opencode/skills/spike-findings-gemini-sdk/sources/001-har-api-coverage/README.md`:

- RPC: `PCck7e`
- Source path: `/app/<conversation>`
- Inner payload (decoded): `["r_0958d664053635a6"]` (the response id)
- Likely purpose: conversation action (regenerate, rating, delete turn)

Additional captures in spike 002 show the same RPC with payload `["r_0d35e86934785889"]` for rating/feedback.

The exact payload shape for the three actions is inferred from the HAR and
reverse-engineering notes:

| Action | Inner payload shape |
|--------|---------------------|
| Regenerate | `[["r_{response_id}"]]` |
| Rate | `[["r_{response_id}", {rating}]]` |
| Delete | `[["r_{response_id}"]]` with a deletion marker |

Because the HAR only captures a single example and the frontend may distinguish
actions by a numeric opcode inside the payload array, the implementation must:

1. Start with the simplest shape that matches the captured sample:
   - regenerate: `[["r_{response_id}"]]`
   - rate: `[["r_{response_id}", {rating_value}]]`
   - delete: `[["r_{response_id}"]]`
2. If tests against fixtures reveal that Google requires a discriminator value
   (e.g. `[1]` for regenerate, `[2]` for delete), document the discovered opcode
   in `07-RESEARCH.md` and update `build_*_payload`.

## Constraints

- Public API additions only; must not break v0.1 consumers.
- All new public items need doc comments because the crate uses `#![deny(missing_docs)]`.
- `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` must pass.
- Cookie redaction rules from earlier phases apply; do not log response ids or raw payloads at info level.

## Out of Scope

- Telemetry / reporting RPCs (`aPya6c`, `maGuAc`, etc.).
- Actual regeneration content returned by the server (the SDK only reports whether the action was accepted; callers can poll the conversation separately if needed).
- UI-level feedback flow beyond a numeric rating.
