# Research Architecture: Usage Stats Reliability

**Project:** Gemini SDK  
**Milestone:** v0.5 Usage Stats Reliability  
**Researched:** 2026-08-11  
**Confidence:** MEDIUM

## Summary

The fix is localized to the `get_usage_stats` call path. It requires:

1. Computing SAPISIDHASH from the client's cookies.
2. Passing that hash (and an authuser index) into the request headers.
3. Updating the inner payload and parser for `jSf9Qc`.

All other `batchexecute` RPCs should keep their existing behavior to avoid
regressions.

## Component Changes

### `src/client.rs`

`get_usage_stats` currently builds headers with:

```rust
let headers = self
    .build_headers(None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
    .await;
```

Change it to compute the SAPISID hash and authuser and pass them in:

```rust
let origin = self.inner.config.read().await.base_url.clone();
let authorization = credentials_to_sapisid_hash(&cookies, &origin);
let headers = self
    .build_headers(None, None, authorization.as_deref(), Some(transport::BATCHEXECUTE_ENDPOINT))
    .await;
// then add x-goog-authuser: 0
```

The existing `build_headers` already has an `authorization: Option<&str>`
parameter, so this reuses existing plumbing. The authuser header can be added
explicitly at the request-building site, since it should only affect this RPC.

### `src/constants.rs`

Add `pub(crate) const GOOG_AUTHUSER: &str = "x-goog-authuser";` in the
headers module. Keep the value `"0"` inline or as a named constant depending
on style consistency with the rest of the file.

### `src/settings.rs`

- Update `build_get_usage_stats_payload()` to match the browser HAR.
- Update `parse_usage_stats_response()` / `extract_inner_value()` to handle the
  array-shaped payload `[2,[[...]],false]`.
- Add typed `UsageStats` accessors for confirmed fields.

### `src/har.rs`

Verify the HAR writer already redacts `Authorization` header values. If not,
add redaction for `Authorization` and `x-goog-authuser` (the authuser index is
low sensitivity, but redacting it keeps captures clean).

## Data Flow

```
GeminiClient::get_usage_stats
  ├─ ensure_session()
  ├─ compute SAPISIDHASH from cookies + origin
  ├─ build batchexecute body (updated inner payload)
  ├─ build headers + Authorization + x-goog-authuser: 0
  ├─ send_batchexecute_with_retry
  └─ parse_usage_stats_response
       ├─ null payload → empty object (existing behavior)
       └─ array payload → unwrap inner value, return UsageStats
```

## Suggested Build Order

1. **Auth headers first** — smallest change, can be validated with a live
   capture showing `Authorization` and `x-goog-authuser` in the outgoing
   request.
2. **Payload/parser second** — once HAR inspection shows the exact browser
   request body, align `build_get_usage_stats_payload` and the parser.
3. **Typed API third** — add accessors after the live response shape is known.
4. **Tests + CLI verification last** — Wiremock fixtures for the array shape,
   then live-cookie acceptance, then CLI check.

## Risk: Scope Creep

The auth fix may also apply to `get_scheduled_prompts` (`XPSWpd`). Resist the
urge to change it in the same milestone unless the HAR or live tests show the
same empty-response symptom. Keep the milestone focused on usage stats.

## Confidence

- **Architecture fit:** HIGH — changes are additive and scoped.
- **Request ordering:** MEDIUM — SAPISIDHASH may need to be sent on the ogads
  init call as well as the batchexecute call; live testing will confirm.
