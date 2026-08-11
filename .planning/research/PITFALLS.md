# Research Pitfalls: Usage Stats Reliability

**Project:** Gemini SDK  
**Milestone:** v0.5 Usage Stats Reliability  
**Researched:** 2026-08-11  
**Confidence:** MEDIUM

## Critical Pitfalls

### 1. Sending SAPISIDHASH on all batchexecute RPCs

**Risk:** The browser only sends `Authorization: SAPISIDHASH` for specific
endpoints (ogads, settings). Adding it globally to `build_headers` could break
other RPCs or trigger extra scrutiny from Google's WAF.

**Prevention:** Pass the authorization hash only in `get_usage_stats` (and
only for the batchexecute request that needs it). Do not change the default
`build_headers` behavior for other methods.

**Addressed in:** Phase 1 (auth header fix).

### 2. Leaking the new Authorization header into HAR captures

**Risk:** `Authorization: SAPISIDHASH <ts>_<sha1>` is derived from session
cookies; writing it to a HAR file leaks credential-derived material.

**Prevention:** Before enabling HAR capture for this path, verify
`src/har.rs` redacts `Authorization`. Add a unit test that records a fake
usage-stats request and asserts the header value is replaced with a redaction
marker.

**Addressed in:** Phase 1 or Phase 3 (depending on HAR audit findings).

### 3. Hardcoding the wrong SAPISID cookie source

**Risk:** `Credentials::sapisid_value()` prefers `__Secure-1PAPISID`, then
`__Secure-3PAPISID`, then legacy `SAPISID`/`APISID`. If the usage RPC requires
a specific one, the fallback chain could compute a valid-looking hash that the
server rejects.

**Prevention:** Use the live HAR to confirm which cookie the browser used for
the `jSf9Qc` request. If it differs from the default, add a usage-specific
helper or document the required cookie set in the test header.

**Addressed in:** Phase 1.

### 4. Unwrapping the response array at the wrong depth

**Risk:** The browser payload `[2,[[999999,0,5,...]],false]` is an array. The
current parser expects a string payload at the `PAYLOAD` slot and falls back
to an empty object on null. If the parser is updated to unwrap one level too
shallow or too deep, callers still see empty or malformed data.

**Prevention:** Add a fixture test that mirrors the exact HAR payload and
asserts the parsed `UsageStats.value()` equals the expected inner value. Only
add typed accessors after the fixture test passes.

**Addressed in:** Phase 2.

### 5. Breaking the existing "no data" semantics

**Risk:** Some accounts legitimately have no usage data and the server returns
a null payload. The current code maps that to `{}`. A new parser must preserve
that behavior or callers will see errors instead of empty stats.

**Prevention:** Keep the existing null → empty object branch. Add a separate
branch for the array-shaped non-null payload.

**Addressed in:** Phase 2.

## Moderate Pitfalls

### 6. Companion CLI drift

**Risk:** `gemini-cli` has a `usage` subcommand that calls
`GeminiClient::get_usage_stats()`. If the return type or field names change,
the CLI may print `null` or panic.

**Prevention:** Include a CLI contract check in the milestone verification
step. Do not rename public accessors without updating the CLI.

**Addressed in:** Phase 3 or verification.

### 7. Over-typing the response

**Risk:** Turning the `[2,[[...]],false]` array into a strongly-typed struct
with named fields for every slot is brittle; Google can reorder or extend the
array.

**Prevention:** Expose only the confirmed scalar fields via accessors and keep
the raw `Value` accessor as the escape hatch. Document which slots are
confirmed by the HAR.

**Addressed in:** Phase 2.

## Confidence

- **Pitfall identification:** HIGH — all listed issues are standard for this
  kind of reverse-engineering task.
- **Mitigation specifics:** MEDIUM — exact redaction logic and cookie source
  depend on HAR/code inspection during implementation.
