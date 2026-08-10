---
phase: 08-user-profile-preferences
plan: 01
type: execute
wave: 1
depends_on:
  - 07-01
files_modified:
  - src/user_profile.rs
  - src/client.rs
  - src/lib.rs
  - tests/integration_tests.rs
  - tests/fixtures/o30O0e_user_info.txt
  - tests/fixtures/o30O0e_user_info_partial.txt
  - tests/fixtures/L5adhe_last_mode.txt
  - tests/fixtures/L5adhe_null_mode.txt
autonomous: true
requirements:
  - USER-01
  - USER-02
  - PREFS-01
  - PREFS-02
  - PREFS-03
must_haves:
  truths:
    - GeminiClient exposes get_user_info(), get_last_selected_mode(), and set_last_selected_mode(mode_id).
    - get_user_info uses batchexecute RPC o30O0e and returns name, photo_url, and email as Option<String>.
    - get_last_selected_mode and set_last_selected_mode use batchexecute RPC L5adhe with the exact payload shape from spike 009.
    - Missing or null fields in the user info response are tolerated and returned as None.
    - Fixture tests verify request payload shape and response parsing without live cookies.
    - cargo test, cargo clippy --all-targets -- -D warnings, and cargo doc --no-deps pass.
  artifacts:
    - src/user_profile.rs
    - tests/fixtures/o30O0e_user_info.txt
    - tests/fixtures/o30O0e_user_info_partial.txt
    - tests/fixtures/L5adhe_last_mode.txt
    - tests/fixtures/L5adhe_null_mode.txt
  key_links:
    - GeminiClient -> batchexecute_rpc -> o30O0e / L5adhe payload builders.
    - parse_user_info_response / parse_last_selected_mode_response -> anti-XSSI stripper -> Option accessors.
---

<objective>
Expose typed public user-profile and preference methods on `GeminiClient`
(`get_user_info`, `get_last_selected_mode`, `set_last_selected_mode`) backed by
the existing `batchexecute_rpc` helper and RPC ids `o30O0e` and `L5adhe`. Add
`UserInfo` and `LastSelectedMode` response types that tolerate missing or null
payload entries, plus fixture-based tests that verify the request and response
paths without live credentials.

Purpose: Fulfill the second slice of the v0.2 API expansion milestone by giving
consumers programmatic access to the signed-in user's identity and the
last-selected mode preference.
Output: New `src/user_profile.rs` module, three new `GeminiClient` methods,
four fixture files, and integration tests.
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
@.planning/phases/07-conversation-actions/07-01-SUMMARY.md
@src/client.rs
@src/conversation_actions.rs
@src/errors.rs
@src/proto/mod.rs
@src/lib.rs
@tests/integration_tests.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create user profile module with payload builders and response types</name>
  <files>src/user_profile.rs</files>
  <action>
    Create `src/user_profile.rs` and define:

    - Private constants `O30O0E_RPC_ID: &str = "o30O0e"` and `L5ADHE_RPC_ID: &str = "L5adhe"`.
    - A public `UserInfo` struct with private fields `name: Option<String>`, `photo_url: Option<String>`, `email: Option<String>`, and public accessor methods `name()`, `photo_url()`, `email()`. Derive `Debug`, `Clone`, `PartialEq`. Add doc comments on every public item.
    - A public `LastSelectedMode` struct with private field `mode_id: Option<String>` and accessor `mode_id()`. Derive `Debug`, `Clone`, `PartialEq`.
    - Public functions `build_get_user_info_payload() -> serde_json::Value` and `build_get_last_selected_mode_payload(current_mode_id: Option<&str>) -> serde_json::Value` and `build_set_last_selected_mode_payload(mode_id: &str) -> serde_json::Value`.
      - `o30O0e` request payload per spike 009:
        ```json
        [["me"], [[["person.photo","person.name","person.email"], null, [1,7]]]]
        ```
      - `L5adhe` read payload per spike 009 when no current mode is known (use all-null leading slots with the mode id at index 7 only when setting):
        - Read: `[[null,null,null,null,null,null,null,null], [["last_selected_mode_id_on_web"]]]`
        - Set:  `[[null,null,null,null,null,null,null,"{mode_id}"], [["last_selected_mode_id_on_web"]]]`
        Preserve exactly 7 leading `null` entries so the mode id is at index 7.
    - Public functions `parse_user_info_response(body: &str) -> Result<UserInfo>` and `parse_last_selected_mode_response(body: &str) -> Result<LastSelectedMode>` that:
      - Strip the anti-XSSI prefix using `crate::proto::strip_xssi_prefix`.
      - Parse the first JSON line.
      - Locate the RPC entry inside the outer batchexecute array by matching `["wrb.fr", "{rpc_id}", ...]`, handling one extra wrapping level exactly like `parse_conversation_action_response` in `src/conversation_actions.rs`.
      - Extract the inner payload string from index 2, falling back to index 3 when index 2 is empty/null, exactly like the conversation-action parser.
      - For `o30O0e`: parse the inner string as `serde_json::Value`, then read optional string fields `name`, `photoUrl`/`photo_url`, and `email`. Accept both camelCase and snake_case keys to be tolerant. Return `UserInfo` with `Option<String>` for each field.
      - For `L5adhe`: parse the inner string as `serde_json::Value`. If the value is a non-empty string, return `LastSelectedMode { mode_id: Some(...) }`; otherwise return `LastSelectedMode { mode_id: None }`.
    - Add unit tests in the module for payload builders and parsers, including partial/missing field cases for `UserInfo` and a `null` mode case for `LastSelectedMode`.
  </action>
  <verify>
    <automated>cargo test --lib user_profile -- --nocapture</automated>
  </verify>
  <done>
    Module compiles, unit tests for payload builders and parsers pass, and all public items are documented.
  </done>
</task>

<task type="auto">
  <name>Task 2: Wire GeminiClient methods and expose module</name>
  <files>src/client.rs, src/lib.rs</files>
  <action>
    In `src/client.rs`:

    - Import the new helpers:
      ```rust
      use crate::user_profile::{
          build_get_last_selected_mode_payload, build_get_user_info_payload,
          build_set_last_selected_mode_payload, parse_last_selected_mode_response,
          parse_user_info_response, LastSelectedMode, UserInfo, L5ADHE_RPC_ID,
          O30O0E_RPC_ID,
      };
      ```
    - Add three async public methods on `GeminiClient`:
      - `pub async fn get_user_info(&self) -> Result<UserInfo>`
      - `pub async fn get_last_selected_mode(&self) -> Result<LastSelectedMode>`
      - `pub async fn set_last_selected_mode(&self, mode_id: impl AsRef<str>) -> Result<()>`
    - `get_user_info`:
      1. Calls `self.ensure_session().await?`.
      2. Builds the inner payload with `build_get_user_info_payload()`.
      3. Calls `self.batchexecute_rpc(O30O0E_RPC_ID, build_batchexecute_body_for_rpc(...), ..., Some("/"))`.
      4. Passes the result to `parse_user_info_response(&text)` and returns it.
    - `get_last_selected_mode`:
      1. Calls `self.ensure_session().await?`.
      2. Builds the inner payload with `build_get_last_selected_mode_payload(None)`.
      3. Calls `self.batchexecute_rpc(L5ADHE_RPC_ID, build_batchexecute_body_for_rpc(...), ..., Some("/"))`.
      4. Passes the result to `parse_last_selected_mode_response(&text)` and returns it.
    - `set_last_selected_mode`:
      1. Calls `self.ensure_session().await?`.
      2. Builds the inner payload with `build_set_last_selected_mode_payload(mode_id.as_ref())`.
      3. Calls `self.batchexecute_rpc(L5ADHE_RPC_ID, build_batchexecute_body_for_rpc(...), ..., Some("/"))`.
      4. Verifies HTTP success; returns `Ok(())` without parsing a response body.
    - Add tracing spans named `gemini.get_user_info`, `gemini.get_last_selected_mode`, `gemini.set_last_selected_mode` with `skip_all`.
    - In `src/lib.rs`:
      - Add `pub mod user_profile;`.
      - Re-export the new public types at crate root:
        ```rust
        pub use user_profile::{LastSelectedMode, UserInfo};
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
  <files>tests/integration_tests.rs, tests/fixtures/o30O0e_user_info.txt, tests/fixtures/o30O0e_user_info_partial.txt, tests/fixtures/L5adhe_last_mode.txt, tests/fixtures/L5adhe_null_mode.txt</files>
  <action>
    Create four fixture files:

    - `tests/fixtures/o30O0e_user_info.txt` — standard batchexecute response for `o30O0e` with all three fields:
      ```text
      )] } '

      [["wrb.fr","o30O0e","{\"name\":\"Jane Doe\",\"photoUrl\":\"https://example.com/photo.jpg\",\"email\":\"jane@example.com\"}",null,null,null,"generic"]]
      ```
    - `tests/fixtures/o30O0e_user_info_partial.txt` — response with one field present and one explicitly `null` to test tolerance:
      ```text
      )] } '

      [["wrb.fr","o30O0e","{\"name\":\"Jane Doe\",\"email\":null}",null,null,null,"generic"]]
      ```
    - `tests/fixtures/L5adhe_last_mode.txt` — response containing a mode id string:
      ```text
      )] } '

      [["wrb.fr","L5adhe","\"cf41b0e0dd7d53e5\"",null,null,null,"generic"]]
      ```
    - `tests/fixtures/L5adhe_null_mode.txt` — response with explicit `null`:
      ```text
      )] } '

      [["wrb.fr","L5adhe","null",null,null,null,"generic"]]
      ```

    Append tests to `tests/integration_tests.rs`:

    - `get_user_info_parses_full_profile`:
      - Start a `wiremock::MockServer`.
      - Mount a `POST /_/BardChatUi/data/batchexecute` responder that returns `o30O0e_user_info.txt` with status 200.
      - Build a client with `with_base_url(&mock_uri).await.with_max_retries(0).await`.
      - Inject session state (`build_label`, `session_id`, `access_token`) via `inner_session_for_tests()` so the client skips the live `/app` init flow.
      - Call `client.get_user_info().await`.
      - Assert `name()`, `photo_url()`, and `email()` match the fixture values.
    - `get_user_info_tolerates_missing_and_null_fields`:
      - Same mock setup returning `o30O0e_user_info_partial.txt`.
      - Assert `name()` is `Some("Jane Doe")`, `photo_url()` is `None`, and `email()` is `None`.
    - `get_last_selected_mode_returns_mode_id`:
      - Mock returning `L5adhe_last_mode.txt`.
      - Assert `mode_id()` is `Some("cf41b0e0dd7d53e5")`.
    - `get_last_selected_mode_returns_none_for_null`:
      - Mock returning `L5adhe_null_mode.txt`.
      - Assert `mode_id()` is `None`.
    - `set_last_selected_mode_sends_l5adhe_payload`:
      - Mock returning status 200 with an empty body.
      - Call `client.set_last_selected_mode("cf41b0e0dd7d53e5").await`.
      - Assert the result is `Ok(())` and that the captured request body contains `"L5adhe"`, `"cf41b0e0dd7d53e5"`, and `"last_selected_mode_id_on_web"`.
  </action>
  <verify>
    <automated>cargo test --test integration_tests user_profile -- --nocapture</automated>
  </verify>
  <done>
    All new integration tests pass, fixtures are read correctly, request shapes match the spike 009 payload, and missing/null fields are tolerated.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| SDK consumer -> GeminiClient | Untrusted identifiers (mode_id) are forwarded to Google; must be length-limited and never logged at info level. |
| GeminiClient -> gemini.google.com | Cookie header crosses the boundary; existing redaction rules apply. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-08-01 | Tampering | mode_id parameter | low | mitigate | Treat as an opaque string; pass through JSON serialization; do not interpret as a path. |
| T-08-02 | Information Disclosure | UserInfo / LastSelectedMode fields | medium | mitigate | Fields are returned as typed Options; Debug output may include email/photo URL. Documentation warns callers not to log these values. |
| T-08-03 | Information Disclosure | Raw batchexecute response body | low | accept | Parser is internal; only typed values are exposed. Errors use Error::parse without echoing the full body. |
| T-08-SC | Tampering | npm/pip/cargo installs | high | mitigate | No new dependencies in this phase; existing dev-dependencies (wiremock) are already audited. |
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

# Optional: run only the user-profile tests
cargo test --test integration_tests user_profile
```

## Expected Results

- `cargo test` passes with no failures.
- `cargo clippy` exits with code 0.
- `cargo doc` produces no warnings.
</verification>

<success_criteria>
- `GeminiClient::get_user_info`, `get_last_selected_mode`, and `set_last_selected_mode` are public, documented, and callable.
- `get_user_info` issues an `o30O0e` batchexecute request to `/` and returns a typed `UserInfo` with optional name, photo_url, and email.
- Missing or null user info fields do not cause a parse error and are returned as `None`.
- `get_last_selected_mode` and `set_last_selected_mode` issue `L5adhe` batchexecute requests with the exact spike 009 payload shape.
- Fixture tests cover full profile, partial/null profile, mode read, null mode read, and mode set.
- Quality gates (`cargo test`, `cargo clippy`, `cargo doc`) are green.
</success_criteria>

<output>
Create `.planning/phases/08-user-profile-preferences/08-01-SUMMARY.md` when done.
</output>
