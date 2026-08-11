# Project Research Summary

**Project:** Gemini SDK  
**Domain:** Reverse-engineered client SDK for the Google Gemini web frontend  
**Researched:** 2026-08-11  
**Confidence:** MEDIUM

## Executive Summary

The `get_usage_stats` endpoint currently returns `{}` because the SDK request
omits the `Authorization: SAPISIDHASH` header and `x-goog-authuser: 0` that
the browser sends for the `jSf9Qc` RPC, and because the inner payload / parser
may not match the live frontend shape `[2,[[999999,0,5,...]],false]`. The fix
requires no new dependencies: the SDK already computes SAPISIDHASH in
`src/auth.rs` and can thread it through `build_headers`. The work should be
scoped tightly to the usage-stats path so other batchexecute RPCs are not
affected. A typed but drift-tolerant `UsageStats` API should expose confirmed
count fields while keeping a raw `serde_json::Value` escape hatch.

## Key Findings

### Recommended Stack

- **Existing `sha1` / `Credentials::sapisid_hash`** — already in `src/auth.rs`;
  reuse it instead of adding a new dependency.
- **Existing `reqwest` header plumbing** — `build_headers` accepts an optional
  `authorization` argument; reuse it and add `x-goog-authuser` only at the
  `get_usage_stats` call site.
- **New internal constant `x-goog-authuser`** — add in `src/constants.rs`;
  keep `pub(crate)`.

### Expected Features

**Must have:**
- Non-empty `UsageStats` for live signed-in accounts with usage data.
- Typed accessors for confirmed count fields (e.g., daily/total usage).
- Backward-compatible null-payload → empty object behavior.
- HAR-matching request payload for `jSf9Qc`.

**Should have:**
- HAR-backed named payload constant with citation.
- Live probe output that prints actual counts.

**Defer:**
- Full schema modeling of every array slot.
- Applying SAPISIDHASH to every batchexecute RPC.
- CLI feature expansion beyond printing the returned counts.

### Architecture Approach

The change is localized:

1. `src/client.rs::get_usage_stats` computes SAPISIDHASH and passes it to
   `build_headers`, then adds `x-goog-authuser: 0` to the outgoing request.
2. `src/settings.rs` updates the request payload and parser to handle the
   array-shaped response.
3. `src/har.rs` ensures the new `Authorization` value is redacted in captures.
4. Tests and live probe verify the end-to-end behavior.

### Critical Pitfalls

1. **Global SAPISIDHASH** — only send it for the usage RPC.
2. **HAR leakage** — redact `Authorization` before writing HAR entries.
3. **Wrong SAPISID cookie source** — confirm the cookie used by the browser.
4. **Array unwrapping depth** — mirror the exact HAR shape in fixture tests.
5. **Breaking empty-data semantics** — preserve null → `{}` fallback.

## Implications for Roadmap

### Phase 1: Auth Header Parity for Usage Stats

**Rationale:** The most likely root cause of `{}` is missing auth headers.
Fixing auth first lets live captures validate that the server returns a
non-empty payload.

**Delivers:**
- SAPISIDHASH computed and sent on `jSf9Qc` requests.
- `x-goog-authuser: 0` header sent on `jSf9Qc` requests.
- HAR redaction for `Authorization`.
- Unit tests for header construction and redaction.

**Addresses:**
- FEATURES: non-empty response, HAR-matching request.
- PITFALLS: global SAPISIDHASH, HAR leakage, wrong cookie source.

### Phase 2: Payload and Parser Alignment

**Rationale:** Once auth is correct, the inner payload shape and response
parser must match the browser. This phase handles the `[2,[[...]],false]`
array shape and adds the typed API.

**Delivers:**
- Updated `build_get_usage_stats_payload` from HAR.
- Parser branch for array-shaped responses.
- `UsageStats` typed accessors for confirmed count fields.
- Wiremock fixture tests for the new response shape.

**Addresses:**
- FEATURES: typed accessors, backward-compatible parser.
- PITFALLS: array unwrapping depth, breaking empty-data semantics, over-typing.

### Phase 3: Live Verification and CLI Contract

**Rationale:** The milestone is only complete when a real account returns real
counts and the companion CLI surfaces them.

**Delivers:**
- Passing live-cookie integration test for `get_usage_stats`.
- Updated `examples/live_probe.rs` output.
- Confirmation that `gemini-cli usage` prints counts.
- All quality gates green.

**Addresses:**
- FEATURES: live probe reporting, CLI contract.
- PITFALLS: CLI drift.

### Phase Ordering Rationale

Auth must come first because without it the server will not return the real
payload, making payload/parser work speculative. Parser/API work comes second
because the exact field semantics depend on a non-empty live response. CLI and
live verification come last because they are acceptance gates.

### Research Flags

- **Phase 2:** Needs deeper research during planning — exact request payload
  and field semantics must be extracted from `/home/vitaly/mitm.har`.
- **Phase 1:** Standard pattern — SAPISIDHASH computation already exists and
  only needs to be wired in.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | No new dependencies; reuse existing auth helper. |
| Features | MEDIUM-HIGH | Scope is narrow; field semantics need HAR/live confirmation. |
| Architecture | HIGH | Additive, localized changes to one RPC path. |
| Pitfalls | HIGH | Standard reverse-engineering risks with clear mitigations. |

**Overall confidence:** MEDIUM — the plan is solid, but the exact request
payload and response field meanings remain unconfirmed until the HAR is
inspected during execution.

### Gaps to Address

- **Gap:** Exact `jSf9Qc` inner payload from the browser.  
  **How to handle:** Inspect `/home/vitaly/mitm.har` in Phase 2 planning.

- **Gap:** Which SAPISID cookie the browser used for the hash.  
  **How to handle:** Check the request headers in the same HAR entry.

- **Gap:** Companion CLI (`gemini-cli`) exact output expectation.  
  **How to handle:** Read `gemini-cli` source during Phase 3 planning.

## Sources

### Primary
- Captured HAR at `/home/vitaly/mitm.har` — observed `jSf9Qc` response shape
  `[2,[[999999,0,5,...]],false]` and browser `Authorization: SAPISIDHASH ...`
  plus `x-goog-authuser: 0`.

### Secondary
- Existing SDK source: `src/auth.rs`, `src/client.rs`, `src/settings.rs`,
  `src/har.rs`, `src/constants.rs`.

---
*Research completed: 2026-08-11*  
*Ready for roadmap: yes*
