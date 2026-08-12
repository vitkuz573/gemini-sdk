# Roadmap: v0.5 Usage Stats Reliability

**Milestone:** v0.5 Usage Stats Reliability  
**Goal:** Fix `GeminiClient::get_usage_stats` so it returns real usage statistics instead of an empty object, matching the live Gemini frontend request shape and auth requirements.

## Phase Overview

| # | Phase | Goal | Requirements | Success Criteria |
|---|-------|------|--------------|------------------|
| 18 | Auth Header Parity for Usage Stats | Send SAPISIDHASH + `x-goog-authuser: 0` on the `jSf9Qc` RPC only. | AUTH-01, AUTH-02, AUTH-03, REQ-02, TEST-03, TEST-04 | 4 |
| 19 | Payload & Parser Alignment | Update `jSf9Qc` request payload and parser to handle the array-shaped response and expose typed accessors. | REQ-01, PARSER-01, PARSER-02, PARSER-03, API-01, API-02, TEST-01, TEST-04 | 4 |
| 20 | Live Verification & CLI Contract | Confirm real cookies return real counts and the companion CLI surfaces them. | TEST-02, CLI-01, CLI-02, TEST-04 | 3 |

**Total phases:** 3  
**Total requirements mapped:** 16  
**Coverage:** 100%

## Phase Details

### Phase 18: Auth Header Parity for Usage Stats

**Goal:** Compute and send the browser-matching `Authorization: SAPISIDHASH`
header and `x-goog-authuser: 0` on the `jSf9Qc` request, without affecting
other batchexecute RPCs.

**Requirements:** AUTH-01, AUTH-02, AUTH-03, REQ-02, TEST-03, TEST-04

**Success criteria:**
1. `get_usage_stats` includes a non-empty `Authorization: SAPISIDHASH ...` header.
2. `get_usage_stats` includes `x-goog-authuser: 0`.
3. Other batchexecute RPCs (e.g., `get_user_info`, `list_models`) are unchanged.
4. HAR redaction removes or masks the `Authorization` header value.
5. `cargo test --all-targets`, `cargo clippy`, and `cargo doc` pass.

### Phase 19: Payload & Parser Alignment

**Goal:** Align the `jSf9Qc` request payload with the captured browser shape
and update the parser to unwrap the array-shaped response, while preserving
empty-data semantics and exposing a typed-but-drift-tolerant API.

**Requirements:** REQ-01, PARSER-01, PARSER-02, PARSER-03, API-01, API-02, TEST-01, TEST-04

**Success criteria:**
1. The inner `f.req` payload for `jSf9Qc` matches the HAR-captured shape.
2. A fixture test with the HAR-observed response `[2,[[999999,0,5,...]],false]` passes.
3. Null payloads still return an empty JSON object.
4. `UsageStats` exposes accessors for daily and total request counts.
5. `UsageStats::value()` still returns the raw `serde_json::Value`.
6. All quality gates pass.

### Phase 20: Live Verification & CLI Contract

**Goal:** Verify the fix against a live signed-in account and confirm the
companion `gemini-cli usage` subcommand prints real counts.

**Requirements:** TEST-02, CLI-01, CLI-02, TEST-04

**Success criteria:**
1. The `get_usage_stats_works` live-cookie integration test returns a non-empty value.
2. `examples/live_probe.rs` reports the parsed usage counts.
3. `gemini-cli usage` outputs real counts for a valid cookie set.
4. The CLI shows a clear message when the account has no usage data.
5. All quality gates pass.

## Traceability

| Requirement | Phase |
|-------------|-------|
| AUTH-01 | 18 |
| AUTH-02 | 18 |
| AUTH-03 | 18 |
| REQ-01 | 19 |
| REQ-02 | 18 |
| PARSER-01 | 19 |
| PARSER-02 | 19 |
| PARSER-03 | 19 |
| API-01 | 19 |
| API-02 | 19 |
| TEST-01 | 19 |
| TEST-02 | 20 |
| TEST-03 | 18 |
| TEST-04 | 18, 19, 20 |
| CLI-01 | 20 |
| CLI-02 | 20 |

## Open Questions / Research Flags

- Phase 19 requires inspection of `/home/vitaly/mitm.har` to extract the exact
  browser request payload and confirm the response field semantics.
- Phase 20 requires access to a live signed-in cookie set and the companion
  CLI repository at `/home/vitaly/projects/gemini-cli`.
