# Spike 008: Cookie Auth Mismatch

## Problem

Cookies copied from a real browser session fail when used in the Rust SDK. The SDK calls `GET https://gemini.google.com/app?hl=en` and receives HTTP 200, but the HTML is unsigned:

- `window.WIZ_global_data.S06Grb = ""`
- `oPEP7c` is missing
- `SNlM0e` is missing

The same cookie set is signed-in in the original browser.

## Inputs

- `/home/vitaly/projects/gemini-sdk/.planning/spikes/004-waa-token/data/mitm.har` — referenced path; actual capture used is `/home/vitaly/mitm.har` (119 MB, read-only, do not commit).
- `/tmp/opencode/gemini_cookies.env` — cookie string passed to the SDK.
- `/home/vitaly/projects/gemini-sdk/src/client.rs`, `src/session.rs`, `src/auth.rs` — SDK source files.

## HAR entry 264: browser request after sign-in

The browser request that successfully loads the signed-in `/app` page is entry 264:

```
GET https://gemini.google.com/  (no hl parameter)
HTTP/2.0
```

Request headers (signed-in, S06Grb present):

| Header | Value |
|--------|-------|
| `cache-control` | `max-age=0` |
| `upgrade-insecure-requests` | `1` |
| `user-agent` | `Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36` |
| `accept` | `text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7` |
| `x-client-data` | `CI7yygE=` |
| `sec-fetch-site` | `cross-site` |
| `sec-fetch-mode` | `navigate` |
| `sec-fetch-dest` | `document` |
| `sec-ch-ua` | `"Not-A.Brand";v="24", "Chromium";v="146"` |
| `sec-ch-ua-mobile` | `?0` |
| `sec-ch-ua-platform` | `"Linux"` |
| `sec-ch-ua-*` | several empty/low-entropy client-hint headers |
| `referer` | `https://accounts.google.com/` |
| `accept-encoding` | `gzip, deflate, br, zstd` |
| `accept-language` | `ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7` |
| `priority` | `u=0, i` |
| `Cookie` | 13 individual `Cookie` pseudo-headers (see below) |

Cookie pseudo-headers sent by the browser (entry 264):

```
__Secure-ENID=35.SE=OrWF7eZeKvTdXQJ...
SID=g.a000BAkf9oOgftLIXQpYtOqy81Hl...
__Secure-1PSID=g.a000BAkf9oOgftLIXQpYt...
__Secure-3PSID=g.a000BAkf9oOgftLIXQpYt...
HSID=AU8f8CWzzFzhJAg-D
SSID=AtDdtdLWunbVLTuWy
APISID=jgcLjniv1Vu1kEeL/...
SAPISID=kNbcET7BNuwSZz18/A3K8K4P2NBAONhTNU
__Secure-1PAPISID=kNbcET7BNuwSZz18/A3K8K4P2NBAONhTNU
__Secure-3PAPISID=kNbcET7BNuwSZz18/A3K8K4P2NBAONhTNU
SIDCC=AKEyXzVjwTRTOzJRXII...
__Secure-1PSIDCC=AKEyXzV5x55bYEv4...
__Secure-3PSIDCC=AKEyXzVd-JOerpPjg5...
```

Entry 52 (initial unsigned landing request, before sign-in) sends only `__Secure-ENID`. Its response is also unsigned (`S06Grb = ""`, `oPEP7c` and `SNlM0e` missing), same HTML shape as the current SDK failure.

## SDK request for `/app`

`GeminiClient::fetch_app_page` in `src/client.rs:704-728` sends:

```
GET https://gemini.google.com/app?hl={language}
HTTP/1.1 (reqwest default; HTTP/2 available if enabled)
```

Headers:

| Header | Value |
|--------|-------|
| `Cookie` | reconstructed from `Credentials` / `Cookies` |
| `User-Agent` | `Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36` |
| `Accept` | `text/html` |

Missing compared to the browser:

- `cache-control`, `upgrade-insecure-requests`
- `sec-fetch-site`, `sec-fetch-mode`, `sec-fetch-dest`, `sec-fetch-user`
- `sec-ch-ua`, `sec-ch-ua-mobile`, `sec-ch-ua-platform` and other client-hint headers
- `x-client-data`
- `referer`
- `accept-language`
- `priority`
- The broader `accept` media-type list

## Cookie comparison

Cookie string from `/tmp/opencode/gemini_cookies.env` contains 8 cookies:

```
__Secure-1PAPISID
__Secure-1PSID
__Secure-1PSIDCC
__Secure-1PSIDTS
__Secure-3PAPISID
__Secure-3PSID
__Secure-3PSIDCC
__Secure-3PSIDTS
```

HAR entry 264 contains 13 cookies. Missing from the SDK env:

```
__Secure-ENID
SID
HSID
SSID
APISID
SAPISID
SIDCC
```

All overlapping secure-cookie values are different between the SDK env and the HAR, which means the env was captured from a different session or a different moment in time than the HAR.

`__Secure-1PSID` value parsing was checked:

- The SDK parser (`Credentials::from_header` / `Cookies::from_header`) preserves the exact value, including `.`, `_`, `-`, and the trailing `0076`.
- No mangling of special characters was observed.

## Experiments

### curl with exact HAR headers and cookies

Using curl with the exact headers and all 13 HAR cookies (including the reconstructed `Cookie` header) returns signed-in HTML:

- `S06Grb = 111628289675248526498`
- `oPEP7c = vitkuz573@gmail.com`
- `SNlM0e = ADR5zap...`

### curl with SDK cookies only

Using the exact HAR headers but only the 8 SDK cookies returns unsigned HTML:

- `S06Grb = ""`
- `oPEP7c` missing
- `SNlM0e` missing

### Adding missing cookies one at a time

Starting from SDK cookies and adding each missing HAR cookie individually did not produce a signed-in page.

### Adding `SID` + `SSID`

Adding **both** `SID` and `SSID` from the HAR to the SDK cookies made the request signed-in:

```
S06Grb = 111628289675248526498
oPEP7c = vitkuz573@gmail.com
SNlM0e = ADR5zap...
```

Other combinations tested:

- `SDK + SID` only → unsigned
- `SDK + SSID` only → unsigned
- `SDK + HSID` only → unsigned
- `SDK + SID + HSID` → unsigned
- `SDK + SSID + HSID` → unsigned
- `__Secure-1PSID + SID + SSID` (minimal) → signed-in
- `__Secure-1PSID + SID + SSID + HSID` → signed-in

This shows that the legacy `SID` and `SSID` cookies are both required (together) for Google to accept this cookie set as signed-in, even though the HAR's own secure cookies can tolerate removing `SID` or `SSID` individually.

### HTTP version

Tested with curl `--http1.1` and `--http2`. Both work when cookies are correct.

### `hl` parameter

Tested `hl=en` and `hl=ru` with SDK cookies only. Both return unsigned HTML; the parameter does not affect signed-in detection when cookies are wrong.

### Header-only variations

With SDK cookies only, adding any of the following headers individually did not produce a signed-in page:

- `Referer: https://accounts.google.com/`
- `X-Client-Data: CI7yygE=`
- `Sec-CH-UA: ...`
- `Accept-Language: ru-RU,ru;q=0.9,...`
- `Priority: u=0, i`
- `Cache-Control: max-age=0`

With correct cookies, removing `x-client-data` still returns signed-in HTML.

### Python requests

Reproduced the same behavior with Python `requests`:

- SDK cookies only → unsigned
- Confirms this is not a Rust/reqwest-specific issue.

## HTML shape comparison

Entry 52 (unsigned, only `__Secure-ENID`) and the SDK response (8 secure cookies) have the same unsigned HTML shape. Both differ from entry 264 only in the absence of the authenticated `window.WIZ_global_data` fields (`S06Grb`, `oPEP7c`, `SNlM0e`). The surrounding landing-page markup is otherwise identical.

## Additional auth tokens in the HAR

No `Authorization` header or URL token was found in the `/app` GET request or subsequent `batchexecute` / `StreamGenerate` requests. The only extra auth-related request header is:

- `x-client-data: CI7yygE=` (already hardcoded in the SDK as `X_CLIENT_DATA`).

## Hypotheses

1. **Missing legacy auth cookies** (confirmed): The SDK env only contains the secure `__Secure-1P*` cookies. Google's sign-in state check for the `/app` page additionally requires the legacy `SID` and `SSID` cookies (and arguably the full legacy set `HSID`, `APISID`, `SAPISID`, `SIDCC`, `__Secure-ENID`).
2. **HTTP/2 vs HTTP/1.1** (ruled out): Both protocols return signed-in HTML when cookies are correct.
3. **Header fingerprinting** (ruled out as primary cause): Exact browser headers with SDK cookies still return unsigned HTML. Adding individual headers to SDK cookies does not help.
4. **Cookie value mangling** (ruled out): `Credentials::from_header` preserves the exact value of `__Secure-1PSID`, including `.`, `_`, `-`, and the trailing `0076`.
5. **`hl` parameter** (ruled out): Both `en` and `ru` return unsigned HTML with bad cookies.
6. **Session mismatch between env and HAR**: The secure-cookie values in the env differ from the HAR. Even with all HAR legacy cookies, the SDK secure cookies are from a different Google session. The fact that `SDK + SID + SSID` works indicates the secure cookies in the env are valid for that account; they just need the accompanying legacy cookies.

## Recommended fixes

1. **Stop requiring only `__Secure-1PSID` + `__Secure-1PSIDCC`**: Update `Credentials::from_header` and `Cookies::from_header` to accept and preserve **all** cookies from the browser's `Cookie` header, especially the legacy auth cookies `SID`, `HSID`, `SSID`, `APISID`, `SAPISID`, `SIDCC`, and `__Secure-ENID`.
2. **Update `is_signed_in` checks**: `Credentials::is_signed_in` currently requires `SID`, `HSID`, `SSID` (good) but the `Cookies` jar and the env file do not supply them. The SDK should document that users must copy the **entire** `Cookie` header from the browser, not a filtered subset.
3. **Forward all cookies to `/app`**: `fetch_app_page` already uses the full cookie jar via `to_header_value`; once the jar contains the legacy cookies, the request will be signed-in.
4. **Consider adding more browser-like headers** (secondary): Although headers are not the blocker, adding `sec-fetch-*`, `sec-ch-ua-*`, `accept-language`, and the full `accept` list may reduce future fingerprint-based rejections.
5. **Handle cookie expiration / refresh**: If Google later rotates required legacy cookies, the SDK should surface a clear error telling the user to refresh the full cookie string from the browser.

## Verdict

**Root cause identified**: The cookie string in `gemini_cookies.env` is missing the legacy Google auth cookies (`SID`, `SSID`, etc.) that Google's `/app` endpoint requires to recognize a signed-in session. The SDK parser preserves values correctly; the request headers are not the primary cause. Once the full browser `Cookie` header is supplied, the SDK's existing `/app` fetch returns signed-in HTML.

## Tags

`gemini`, `auth`, `cookies`, `signed-in`, `har`, `reverse-engineering`, `SID`, `SSID`, `legacy-cookies`
