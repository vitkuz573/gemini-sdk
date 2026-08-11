# Research Features: Usage Stats Reliability

**Project:** Gemini SDK  
**Milestone:** v0.5 Usage Stats Reliability  
**Researched:** 2026-08-11  
**Confidence:** MEDIUM

## Summary

The corrected `get_usage_stats` must behave like the live Gemini frontend:
return a non-empty payload with account-level usage counts. The HAR-observed
shape `[2,[[999999,0,5,...]],false]` suggests an array wrapping the real data,
so the parser needs to unwrap one level deeper than the current
`serde_json::Value` wrapper.

## Table Stakes (must have)

1. **Non-empty response when the account has usage data**
   - After sending the correct auth headers and payload, `get_usage_stats`
     must not return `{}` for a live signed-in account.
   - Acceptance: live-cookie test yields a `UsageStats` value whose inner
     object/array is non-empty.

2. **Typed accessors for the most common fields**
   - At minimum expose:
     - `requests_today` or equivalent daily count
     - `requests_total` or equivalent total count
   - Keep a `value()` accessor returning raw `serde_json::Value` so protocol
     drift does not break callers.

3. **Backward-compatible parser**
   - The existing parser returns `Value::Object` for null payloads. Preserve
     that behavior for accounts with no data.
   - When the payload is the array shape `[2,[[...]],false]`, expose the inner
     array/object in a predictable way.

4. **Correct request shape**
   - The inner `f.req` payload for `jSf9Qc` must match the browser capture.
   - Current SDK sends `[]`; the browser likely sends a richer array or object.

## Differentiators (should have)

1. **HAR-backed request payload constant**
   - Add the payload as a named constant with a HAR citation, following the
     v0.3/v0.4 convention of documented, named protocol literals.

2. **Live probe reports actual counts**
   - Update `examples/live_probe.rs` so the `get_usage_stats` probe prints the
     parsed counts when available, not just success/failure.

## Anti-features / Defer

1. **Full schema modeling of every array slot**
   - Do not hardcode brittle structs for the undocumented `[2,[[...]],false]`
     shape. Use typed accessors only for fields confirmed in the HAR / live
     testing.

2. **Applying SAPISIDHASH to every batchexecute RPC**
   - Scope it to the usage path first; broadening it is a separate decision
     that could affect other RPCs.

3. **Companion CLI rewrite**
   - Only verify / lightly adjust the CLI contract; do not expand CLI scope.

## Feature Mapping Notes

| Feature | File | Notes |
|---------|------|-------|
| Auth header fix | `src/client.rs` | Compute and send SAPISIDHASH + x-goog-authuser. |
| Payload fix | `src/settings.rs` | Update `build_get_usage_stats_payload`. |
| Parser fix | `src/settings.rs` | Unwrap array-shaped response; keep null fallback. |
| Typed API | `src/settings.rs` | Add `UsageStats` accessors; keep raw `value()` escape hatch. |
| Live acceptance | `tests/real_cookies.rs`, `examples/live_probe.rs` | Verify non-empty stats. |
| CLI contract | `gemini-cli` | Confirm `usage` subcommand prints counts. |

## Confidence

- **Feature set:** HIGH — scope is narrow and well understood.
- **Field semantics:** MEDIUM — exact meaning of each slot in `[2,[[...]],false]`
  is not yet confirmed; needs HAR inspection or live test.
