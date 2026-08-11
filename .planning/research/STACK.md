# Research Stack: Usage Stats Reliability

**Project:** Gemini SDK  
**Milestone:** v0.5 Usage Stats Reliability  
**Researched:** 2026-08-11  
**Confidence:** MEDIUM

## Summary

The SDK already has most of the stack pieces needed to fix `get_usage_stats`.
No new external dependencies are required: the existing `sha1` transitive
dependency (via `Credentials::sapisid_hash`) can produce the
`Authorization: SAPISIDHASH <ts>_<sha1>` value, and `reqwest` can carry the
new `x-goog-authuser` header. The only additions are internal constants and a
small amount of request-building logic scoped to the `jSf9Qc` RPC path.

## Recommended Stack / Additions

### 1. Internal `Authorization: SAPISIDHASH` helper (no new dependency)

`src/auth.rs` already exposes `Credentials::sapisid_hash(origin)`.
`src/client.rs` already has a helper `credentials_to_sapisid_hash(cookies,
origin)`. The fix is to **call it for the usage-stats request** and thread the
result into `build_headers`.

- **Why here:** The hash depends only on the cookies held by `GeminiClient`
  and the request origin, both available in `get_usage_stats`.
- **Rationale:** The live browser sends this header for the ogads / settings
  path; the SDK currently only uses it for WAA/ogads init, not for
  `batchexecute` calls.

### 2. `x-goog-authuser: 0` header for the usage-stats request

Add a new header constant for `x-goog-authuser` and send it on the
`get_usage_stats` request. Browser captures show this header present on the
relevant RPC.

- **Why `0`:** The SDK targets the default / primary Google account in the
  cookie session; `0` is the standard authuser index for the first signed-in
  account.
- **Where:** inside `get_usage_stats`, after `build_headers` returns, push the
  authuser header onto the request. Alternatively extend `build_headers` with
  an optional authuser parameter so only the usage RPC opts in.

### 3. Constants in `src/constants.rs`

Add:

- `headers::GOOG_AUTHUSER` = `"x-goog-authuser"`
- `headers::GOOG_AUTHUSER_DEFAULT` = `"0"` (or keep it inline with a comment)

No new public API surface is needed; these can remain `pub(crate)`.

### 4. Minimal modern cookie set

The browser request relies on `__Secure-1PSID`, `__Secure-1PSIDCC`,
`__Secure-1PSIDTS`, `__Secure-1PAPISID`, and `__Secure-3PAPISID`. The SDK
already parses all of these in `Credentials`/`Cookies`. The only gap is that
`get_usage_stats` does not currently compute or send SAPISIDHASH.

## Stack Changes NOT Needed

- No new crates or dependencies.
- No change to the transport layer (`send_batchexecute_with_retry`).
- No change to retry/backoff policy.
- No browser automation / CDP changes.

## Integration Points

| Component | Change |
|-----------|--------|
| `src/constants.rs` | Add `x-goog-authuser` constant(s). |
| `src/client.rs` | In `get_usage_stats`, compute SAPISIDHASH from cookies, pass it and authuser to `build_headers`, send on request. |
| `src/settings.rs` | Keep `UsageStats` wrapper; optionally add typed accessors once the response shape is confirmed. |
| `src/har.rs` | Redact `Authorization` values if not already redacted. |

## Confidence

- **Stack:** HIGH — only internal wiring, no new dependencies.
- **SAPISIDHASH correctness:** MEDIUM — existing `sapisid_hash` implementation
  is tested, but the live frontend may require exact origin formatting or a
  specific SAPISID source cookie; needs live validation.
