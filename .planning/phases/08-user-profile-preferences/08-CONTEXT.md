# Phase 8 — User Profile & Preferences

## Scope

Add typed public APIs for retrieving the signed-in user's profile and for
reading/writing the user's last-selected Gemini mode preference. Both features
are thin facades over the existing `batchexecute` transport and reuse the
configurable `base_url` infrastructure established in Phase 7.

## Requirements

- **USER-01**: SDK exposes `get_user_info()` using RPC `o30O0e` and returns name, photo URL, and email.
- **USER-02**: User profile fields are optional and tolerant of missing or null payload entries.
- **PREFS-01**: SDK exposes `get_last_selected_mode()` using RPC `L5adhe`.
- **PREFS-02**: SDK exposes `set_last_selected_mode(mode_id)` using RPC `L5adhe`.
- **PREFS-03**: Preference payloads use the exact shape captured in spike 009.

## Key Decisions

- Reuse the existing private helper `GeminiClient::batchexecute_rpc` in `src/client.rs`; no new transport code.
- Keep the RPC ids `o30O0e` and `L5adhe` as named constants in a new `src/user_profile.rs` module, alongside payload builders and response types.
- Source path for these RPCs is `/` per the HAR capture.
- `get_user_info` returns a typed `UserInfo` struct with optional fields.
- `get_last_selected_mode` returns a typed `LastSelectedMode` struct with an optional `mode_id`.
- `set_last_selected_mode(mode_id)` returns `Result<()>` on HTTP success; no body parsing required beyond status check.
- Use `Option<String>` for user info fields so missing or `null` entries do not fail the call.
- No live network calls in CI; verify with `wiremock` fixtures following the pattern from Phase 7.

## Inputs from Spike

From `.opencode/skills/spike-findings-gemini-sdk/sources/001-har-api-coverage/README.md`:

- RPC: `o30O0e`
- Source path: `/`
- Inner payload (decoded): `[["me"], [[["person.photo","person.name","person.email"]], null, [1,7]]]`
- Likely purpose: user info

- RPC: `L5adhe`
- Source path: `/`
- Inner payload (decoded): `[[null,...null, "cf41b0e0dd7d53e5"], [["last_selected_mode_id_on_web"]]]`
- Likely purpose: user prefs / last mode

The exact `L5adhe` payload shape for reads and writes is captured in more
detail in `08-RESEARCH.md` (spike 009 analysis).

## Constraints

- Public API additions only; must not break v0.1 consumers.
- All new public items need doc comments because the crate uses `#![deny(missing_docs)]`.
- `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` must pass.
- Cookie redaction rules from earlier phases apply; do not log user info or raw payloads at info level.

## Out of Scope

- Updating other user preferences beyond `last_selected_mode_id_on_web`.
- Telemetry / reporting RPCs.
- Settings page data (`jSf9Qc`, `XPSWpd`) — covered in Phase 10.
