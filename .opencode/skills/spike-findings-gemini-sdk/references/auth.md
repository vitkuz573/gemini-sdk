# Authentication and Signed-In Detection

## Requirements

- The SDK must detect when cookies are not actually accepted by Google as a signed-in session.
- `SNlM0e` must be extracted from `window.WIZ_global_data` and passed as the `at` query parameter.

## How to Build It

### Required cookies

The browser sends all of these on a signed-in request:

- `SID`
- `HSID`
- `SSID`
- `APISID`
- `SAPISID`
- `__Secure-1PSID`
- `__Secure-1PAPISID`
- `__Secure-1PSIDCC`
- `__Secure-1PSIDTS`
- `__Secure-ENID`
- `SIDCC`

The SDK currently requires `__Secure-1PSID` and `__Secure-1PSIDCC` at construction time, but for live calls the full set matters.

### Signed-in verification

After fetching `/app`, inspect `window.WIZ_global_data`:

```rust
let s06grb = extract_quoted_value(block, "S06Grb");
let opep7c = extract_quoted_value(block, "oPEP7c");
let is_signed_in = !s06grb.is_empty()
    && s06grb.chars().all(|c| c.is_ascii_digit())
    && looks_like_email(&opep7c);
```

- Empty `S06Grb` or missing `oPEP7c` means the page is the public landing page, not a signed-in session.
- The page may still contain a `ServiceLogin` link in the account menu; do not reject based on that substring alone.

### SNlM0e extraction and validation

Token shape: `<base64url-ish prefix>:<13-digit timestamp>`.

```rust
fn is_valid_snlim0e(token: &str) -> bool {
    let bytes = token.as_bytes();
    let colon = bytes.iter().position(|&b| b == b':').unwrap_or(0);
    let prefix = &bytes[..colon];
    let suffix = &bytes[colon + 1..];
    prefix.iter().all(|&b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        && suffix.len() == 13
        && suffix.iter().all(|&b| b.is_ascii_digit())
}
```

Search first inside the `window.WIZ_global_data` block; fall back to an unanchored search in the full body if necessary.

### Cookie header gotchas

- The SDK calls `/app?hl={language}`; the browser calls `GET /` with no `hl` and a full browser header set (including `sec-fetch-*`, `accept-language`, `referer`).
- If the SDK receives unsigned HTML while the browser is signed in, the usual cause is missing legacy cookies (`SID`, `HSID`, `SSID`, `APISID`, `SAPISID`) or stale `__Secure-1PSIDTS`.
- `APISID`/`SAPISID` are derived from `SID` server-side; include them explicitly when copying cookies.

## What to Avoid

- Do not rely solely on the presence of `__Secure-1PSID`/`__Secure-1PSIDCC` to decide signed-in state.
- Do not extract `SNlM0e` from just anywhere in the HTML without validating the token shape; sign-in/interstitial pages also contain `SNlM0e`.
- Do not send `/app?hl=en` without also sending the full cookie header; Google may serve the landing page.

## Constraints

- Google can revoke or rotate cookies at any time; the only reliable signal is the signed-in `/app` HTML shape.
- `SNlM0e` is time-scoped; stale tokens eventually cause 4xx on `batchexecute`/`StreamGenerate`.

## Origin

Synthesized from spikes: 005-snlM0e, 006-signed-in-detection, 008-cookie-auth.
Source files available in: `sources/005-snlM0e/`, `sources/006-signed-in-detection/`, `sources/008-cookie-auth/`.
