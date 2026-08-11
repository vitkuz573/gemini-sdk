# Phase 18: Auth Header Parity for Usage Stats - Context

**Gathered:** 2026-08-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Compute and send the browser-matching `Authorization: SAPISIDHASH` header and `x-goog-authuser: 0` on the `jSf9Qc` batchexecute request, without affecting other batchexecute RPCs. Update HAR redaction tests and quality gates.

</domain>

<decisions>
## Implementation Decisions

### Auth Scope
- Scope SAPISIDHASH + `x-goog-authuser: 0` **only** to the `jSf9Qc` usage-stats RPC (requirement AUTH-03).
- Other batchexecute RPCs (`get_user_info`, `list_models`, etc.) must remain unchanged.

### Header Generation
- Reuse existing `Credentials::sapisid_hash(origin)` to compute the `Authorization: SAPISIDHASH <ts>_<sha1>` value.
- Pass the computed authorization into `build_headers` via the existing `authorization: Option<&str>` parameter, which is currently only used by WAA/ogads flows.
- Add named constants for the `x-goog-authuser` header name and value `0` in `src/constants.rs` (requirement REQ-02).

### API Surface
- Do not change public `GeminiClient` signatures.
- Internal `build_headers` signature can be extended with an opt-in flag if needed, but prefer using the existing `authorization` parameter.

### Testing
- Add a wiremock-style integration test that verifies `Authorization` and `x-goog-authuser` appear on the `jSf9Qc` request.
- Verify other RPCs (e.g., `get_user_info`) do **not** receive those headers.
- Add a HAR redaction unit test covering the `Authorization` header (requirement TEST-03).

### the agent's Discretion
- Exact constant naming and module placement in `src/constants.rs` is left to implementation.
- How `get_usage_stats` extracts the SAPISID source from credentials is left to implementation (prefer `credentials_to_sapisid_hash` helper already used by ogads).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Credentials::sapisid_hash(origin)` in `src/auth.rs` computes the browser-matching hash.
- `credentials_to_sapisid_hash(cookies, origin)` in `src/client.rs` adapts `Cookies` -> `Credentials` -> hash.
- HAR redaction in `src/har.rs` already treats `Authorization` as a secret header.

### Established Patterns
- `GeminiClient::build_headers(reqid, waa_context, authorization, endpoint)` returns a `Vec<(String, String)>` of headers.
- All batchexecute RPC methods call `build_headers(None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))`.
- Constants are centralized in `src/constants.rs` as `pub(crate)` named constants.

### Integration Points
- `src/client.rs::get_usage_stats` builds the `jSf9Qc` request and calls `build_headers`.
- `src/client.rs::ogads_get_async_data` already uses `credentials_to_sapisid_hash` and passes `authorization` into `build_headers` indirectly via manual reqwest building.
- `src/har.rs::is_secret_header` controls HAR redaction.

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond the ROADMAP success criteria and REQUIREMENTS.md acceptance criteria. Use existing SAPISIDHASH helper and keep changes scoped to `jSf9Qc`.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
