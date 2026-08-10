---
phase: 07-conversation-actions
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/conversation_actions.rs
  - src/client.rs
  - src/lib.rs
  - tests/integration_tests.rs
  - tests/fixtures/pcck7e_success.txt
  - tests/fixtures/pcck7e_error.txt
autonomous: true
requirements:
  - CONVACT-01
  - CONVACT-02
  - CONVACT-03
  - CONVACT-04
must_haves:
  truths:
    - GeminiClient exposes regenerate_turn, rate_turn, and delete_turn.
    - All three methods call batchexecute RPC PCck7e with source-path /app/{conversation_id}.
    - Each method returns a typed ConversationActionResult with success/failure status.
    - Fixture tests verify request payload shape and response parsing without live cookies.
    - cargo test, cargo clippy --all-targets -- -D warnings, and cargo doc --no-deps pass.
  artifacts:
    - src/conversation_actions.rs
    - tests/fixtures/pcck7e_success.txt
    - tests/fixtures/pcck7e_error.txt
  key_links:
    - GeminiClient -> batchexecute_rpc -> PCck7e payload builder.
    - parse_conversation_action_response -> anti-XSSI stripper -> success/failure enum.
---

<objective>
Expose typed public conversation-action methods on `GeminiClient` (`regenerate_turn`,
`rate_turn`, `delete_turn`) backed by the existing `batchexecute_rpc` helper and
RPC id `PCck7e`. Add a `ConversationActionResult` type that parses the WIZ-framed
response into success/failure status, plus fixture-based tests that verify the
request and response paths without live credentials.

Purpose: Fulfill the first slice of the v0.2 API expansion milestone by giving
consumers programmatic control over conversation history actions.
Output: New `src/conversation_actions.rs` module, three new `GeminiClient` methods,
two fixture files, and integration tests.
</objective>

<execution_context>
@/home/vitaly/.config/opencode/gsd-core/workflows/execute-plan.md
@/home/vitaly/.config/opencode/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/REQUIREMENTS.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.opencode/skills/spike-findings-gemini-sdk/sources/001-har-api-coverage/README.md
@src/client.rs
@src/errors.rs
@src/proto/mod.rs
@src/lib.rs
@tests/integration_tests.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create conversation actions module with payload builders and response type</name>
  <files>src/conversation_actions.rs</files>
  <action>
    Create `src/conversation_actions.rs` and define:

    - A public `ConversationAction` enum with variants `Regenerate`, `Rate(TurnRating)`, `Delete`.
    - A public `TurnRating` enum with variants `Good`, `Bad`, `Neutral`.
    - A public `ConversationActionResult` struct with private fields `success: bool`, `action: ConversationAction`, `response_id: String`, `raw: serde_json::Value`, and public accessor methods `success()`, `action()`, `response_id()`, `raw()`.
    - A private constant `PCCK7E_RPC_ID: &str = "PCck7e"`.
    - Public functions `build_regenerate_payload(response_id)`, `build_rate_payload(response_id, rating)`, and `build_delete_payload(response_id)` that return `serde_json::Value`.
      - Regenerate payload: `[["r_{response_id}"]]` (use the existing `r_` prefix if not already present, or pass the id through).
      - Rate payload: `[["r_{response_id}", {rating_value}]]` where `Good=1`, `Bad=0`, `Neutral=null`.
      - Delete payload: `[["r_{response_id}"]]`.
    - A public function `parse_conversation_action_response(body: &str) -> Result<ConversationActionResult>` that:
      - Strips the anti-XSSI prefix using `crate::proto::strip_xssi_prefix`.
      - Parses the first JSON line.
      - Locates the `PCck7e` entry inside the outer batchexecute array (handling one extra wrapping level like `parse_model_list` does).
      - Extracts the inner payload string from index 2 or 3.
      - Parses the inner payload.
      - Treats the result as successful unless the payload is an object containing a non-null `error` field or a string starting with `error`.
      - Returns `ConversationActionResult` with `success` inferred, the supplied `action`, the supplied `response_id`, and the parsed inner value as `raw`.
    - Derive `Debug`, `Clone`, `PartialEq` for public enums/structs.
    - Add doc comments on every public item to satisfy `#![deny(missing_docs)]`.
  </action>
  <verify>
    <automated>cargo test --lib conversation_actions -- --nocapture</automated>
  </verify>
  <done>
    Module compiles, unit tests for payload builders and parser pass, and all public items are documented.
  </done>
</task>

<task type="auto">
  <name>Task 2: Wire GeminiClient methods and expose module</name>
  <files>src/client.rs, src/lib.rs</files>
  <action>
    In `src/client.rs`:

    - Import the new helpers:
      ```rust
      use crate::conversation_actions::{
          build_delete_payload, build_rate_payload, build_regenerate_payload,
          parse_conversation_action_response, ConversationAction, ConversationActionResult,
          TurnRating,
      };
      ```
    - Add three async public methods on `GeminiClient`:
      - `pub async fn regenerate_turn(&self, conversation_id: impl AsRef<str>, response_id: impl AsRef<str>) -> Result<ConversationActionResult>`
      - `pub async fn rate_turn(&self, conversation_id: impl AsRef<str>, response_id: impl AsRef<str>, rating: TurnRating) -> Result<ConversationActionResult>`
      - `pub async fn delete_turn(&self, conversation_id: impl AsRef<str>, response_id: impl AsRef<str>) -> Result<ConversationActionResult>`
    - Each method:
      1. Calls `self.ensure_session().await?`.
      2. Acquires the current session (`language`, `build_label`, `session_id`) and cookie header.
      3. Builds the inner JSON payload with the appropriate builder.
      4. Calls `self.batchexecute_rpc(PCCK7E_RPC_ID, build_batchexecute_body_for_rpc(...), ..., Some(source_path))` where `source_path = format!("/app/{conversation_id}")`.
      5. Passes the result to `parse_conversation_action_response(&text, action, response_id.into())`.
      6. Returns the parsed `ConversationActionResult`.
    - Add tracing spans named `gemini.regenerate_turn`, `gemini.rate_turn`, `gemini.delete_turn` with `skip_all` and a `response_id` field.

    In `src/lib.rs`:

    - Add `pub mod conversation_actions;`.
    - Re-export the new public types at crate root:
      ```rust
      pub use conversation_actions::{ConversationAction, ConversationActionResult, TurnRating};
      ```
  </action>
  <verify>
    <automated>cargo check --all-targets</automated>
  </verify>
  <done>
    Crate compiles with the new methods and re-exports visible, and no doc or clippy warnings are introduced.
  </done>
</task>

<task type="auto">
  <name>Task 3: Add fixture-based integration tests</name>
  <files>tests/integration_tests.rs, tests/fixtures/pcck7e_success.txt, tests/fixtures/pcck7e_error.txt</files>
  <action>
    Create two fixture files:

    - `tests/fixtures/pcck7e_success.txt` — a standard batchexecute response for `PCck7e` that parses as success:
      ```text
      )] } ' 

      [["wrb.fr","PCck7e","[1]",null,null,null,"generic"]]
      ```
    - `tests/fixtures/pcck7e_error.txt` — a batchexecute response that parses as failure:
      ```text
      )] } ' 

      [["wrb.fr","PCck7e","{\"error\":\"turn not found\"}",null,null,null,"generic"]]
      ```

    Append tests to `tests/integration_tests.rs`:

    - `regenerate_turn_sends_pcck7e_payload`:
      - Start a `wiremock::MockServer`.
      - Mount a `POST /_/BardChatUi/data/batchexecute` responder that returns `pcck7e_success.txt` with status 200 and `Content-Type: application/json`.
      - Build a client with `with_max_retries(0)`.
      - Call `client.regenerate_turn("conv_123", "r_abc").await`.
      - Assert the result `success()` is true.
      - Capture the incoming request body and assert it contains `PCck7e`, `/app/conv_123`, and `"r_abc"`.
    - `rate_turn_sends_rating_value`:
      - Same mock setup.
      - Call `client.rate_turn("conv_123", "r_abc", TurnRating::Good).await`.
      - Assert success and assert the request body contains the rating value (`1` for Good).
    - `delete_turn_reports_failure_on_error_payload`:
      - Same mock setup returning `pcck7e_error.txt`.
      - Call `client.delete_turn("conv_123", "r_abc").await`.
      - Assert the result `success()` is false.
    - `parse_conversation_action_response_handles_wrapped_array`:
      - Unit-style assertion inside the integration test file that feeds a double-wrapped outer array (the extra wrapping level observed for batchexecute) and checks success.
  </action>
  <verify>
    <automated>cargo test --test integration_tests conversation_action -- --nocapture</automated>
  </verify>
  <done>
    All new integration tests pass, fixtures are read correctly, and request shapes match the expected payload format.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| SDK consumer → GeminiClient | Untrusted identifiers (conversation_id, response_id) are forwarded to Google; must be length-limited and never logged at info level. |
| GeminiClient → gemini.google.com | Cookie header crosses the boundary; existing redaction rules apply. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-07-01 | Tampering | conversation_id / response_id parameters | low | mitigate | Treat as opaque strings; pass through URL encoding and JSON serialization; do not interpret as paths. |
| T-07-02 | Information Disclosure | ConversationActionResult::raw | low | accept | Raw response is intentionally exposed for debugging; documentation warns callers not to log it if it contains sensitive data. |
| T-07-03 | Denial of Service | Untrusted response id length | medium | mitigate | Document that ids are forwarded unchanged; callers must validate lengths before calling. No allocation amplification beyond one JSON string. |
| T-07-SC | Tampering | npm/pip/cargo installs | high | mitigate | No new dependencies in this phase; existing dev-dependencies (`wiremock`) are already audited. |
</threat_model>

<verification>
## Commands

```bash
# Run all tests including the new fixtures
cargo test

# Run clippy with the project's strict warning policy
cargo clippy --all-targets -- -D warnings

# Build docs without warnings
cargo doc --no-deps

# Optional: run only the conversation-action tests
cargo test --test integration_tests conversation_action
```

## Expected Results

- `cargo test` passes with no failures.
- `cargo clippy` exits with code 0.
- `cargo doc` produces no warnings.
</verification>

<success_criteria>
- `GeminiClient::regenerate_turn`, `rate_turn`, and `delete_turn` are public, documented, and callable.
- Each method issues a `PCck7e` batchexecute request to `/app/{conversation_id}`.
- Responses parse into `ConversationActionResult` with a correct `success()` value.
- Fixture tests cover success, failure, and rating-value encoding.
- Quality gates (`cargo test`, `cargo clippy`, `cargo doc`) are green.
</success_criteria>

<output>
Create `.planning/phases/07-conversation-actions/07-01-SUMMARY.md` when done.
</output>
