# Spike 006 — Detecting signed-in state

**Goal:** Determine how the real Gemini web frontend decides whether the user is
authenticated, and how the SDK should implement `is_signed_in`.

**Inputs:**

- `src/auth.rs`
- `src/client.rs`
- `src/session.rs`
- `/tmp/opencode/gemini_cookies.env`
- `/home/vitaly/mitm.har` (119 MB, not committed; the originally referenced
  `.planning/spikes/004-waa-token/data/mitm.har` is missing from the working
  tree because `*.har` is gitignored)

**Scope:** research and documentation only; no SDK code changes.

## 1. Current SDK behavior

`Credentials::is_signed_in` (`src/auth.rs:118`) and `Cookies::is_signed_in`
(`src/auth.rs:315`) both return `true` iff:

- `__Secure-1PSID` is present and non-empty, **and**
- `__Secure-1PSIDCC` is present and non-empty.

`Credentials::validate` enforces the same two cookies. This is the only
authentication gate in the SDK today:

- `GeminiClient::from_cookie_header` validates cookies at construction time.
- After that, `ensure_session` / `init_session` fetch `/app` and extract WIZ
  tokens, but it never checks whether the returned page is a sign-in page.
- If cookies are expired or invalid, the user gets a low-level HTTP/API error
  from a batchexecute call rather than a clear "not signed in" message.

### Flaws of the current logic

1. **Cookie-only check:** it never asks the server whether the session is
   actually valid. Cookies can be expired, revoked, or copied from an
   incognito session that has already been invalidated.
2. **Ignores `__Secure-1PSIDTS`:** the timestamp cookie is required for some
   endpoints (e.g. `StreamGenerate`) but `is_signed_in` does not require it.
3. **Ignores the `/app` page shape:** the frontend itself uses the presence of
   `window.WIZ_global_data.S06Grb` (the Gaia ID) and `oPEP7c` (the account
   email) to decide whether the user is signed in. The SDK does not look at
   these signals.
4. **No redirect detection:** when the user is not signed in, `GET /app` (or
   `GET /`) returns the public landing page or redirects to
   `accounts.google.com`. The SDK does not detect this case.

## 2. What the real frontend uses

The signed-in HTML page (`entry 264`, `GET https://gemini.google.com/`) is
served from `boq_assistant-bard-web-server_*` and contains:

```json
{
  "S06Grb": "111628289675248526498",
  "FdrFJe": "-1594710263937718439",
  "SNlM0e": "ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132",
  "oPEP7c": "vitkuz573@gmail.com",
  "W3Yyqf": "111628289675248526498",
  "WZsZ1e": "kNbcET7BNuwSZz18/A3K8K4P2NBAONhTNU",
  "qDCSke": "111628289675248526498",
  "cfb2h": "boq_assistant-bard-web-server_20260806.17_p0"
}
```

The unsigned landing page (`entry 52`, `GET https://gemini.google.com/` with
only `__Secure-ENID` cookie) returns the **same server build** but with the
user fields empty/missing:

```json
{
  "S06Grb": "",
  "FdrFJe": "5075253380415205645",
  "SNlM0e": null,
  "oPEP7c": null,
  "cfb2h": "boq_assistant-bard-web-server_20260806.17_p0"
}
```

The sign-in page (`entry 155`, `accounts.google.com/v3/signin/identifier`)
contains `ServiceLogin` and no Gemini app markup.

### Frontend auth signals

| Signal | Signed-in (`entry 264`) | Not signed-in (`entry 52`) | Meaning |
|--------|--------------------------|----------------------------|---------|
| `S06Grb` | non-empty Gaia ID | `""` | Primary signed-in user ID |
| `oPEP7c` | email address | missing | Signed-in account email |
| `SNlM0e` | valid token | missing | `at` token for batchexecute |
| `WZsZ1e` | obfuscated string | missing | Session secret / nonce source |
| `W3Yyqf` | same as `S06Grb` | missing | Secondary user id |
| `qDCSke` | same as `S06Grb` | missing | Another user id copy |
| `FdrFJe` | present | present | Frontend session id (`f.sid`) |
| `cfb2h` | bard server build | bard server build | Build label |

The frontend therefore treats the page as signed-in when `S06Grb` is a
non-empty numeric string and `oPEP7c` is present. Only in that state does it
call `otAQ7b`, `sJBwce`, `Waa/Create`, `ogads` and the rest of the warm-up
chain.

### RPC call pattern in the capture

- **Not signed-in (`entry 52`):** the browser is immediately redirected to
  `accounts.google.com` (`entry 153` 302, `entry 155` sign-in page). No
  `otAQ7b` call is made from the signed-in path. The only batchexecute calls
  before authentication use other RPC ids on the public landing page.
- **Signed-in (`entry 264`):** immediately after the HTML loads, the frontend
  calls `otAQ7b` (`entry 284`), `sJBwce` (`entry 287`), `Waa/Create`
  (`entry 280`), `ogads GetAsyncData` (`entry 276`) and so on.

There is no single explicit JS boolean like `isSignedIn`; the decision is made
by checking `window.WIZ_global_data.S06Grb` and `oPEP7c`.

## 3. Required cookies for Gemini endpoints

The captured signed-in requests contain the following cookies:

| Cookie | `batchexecute` | `StreamGenerate` | `ogads` | Notes |
|--------|---------------|------------------|---------|-------|
| `SID` | yes | yes | yes | Legacy session id |
| `HSID` | yes | yes | yes | HTTP-only auth |
| `SSID` | yes | yes | yes | Secure auth |
| `APISID` | yes | yes | yes | Used for `SAPISIDHASH` fallback |
| `SAPISID` | yes | yes | yes | Used for `SAPISIDHASH` fallback |
| `__Secure-1PAPISID` | yes | yes | yes | Preferred `SAPISIDHASH` source |
| `__Secure-3PAPISID` | yes | yes | yes | 3P variant |
| `__Secure-1PSID` | yes | yes | yes | Primary signed-in session |
| `__Secure-3PSID` | yes | yes | yes | 3P variant |
| `__Secure-1PSIDCC` | yes | yes | yes | Secondary signed-in token |
| `__Secure-3PSIDCC` | yes | yes | yes | 3P variant |
| `SIDCC` | yes | yes | yes | Consent/anti-abuse |
| `__Secure-ENID` | yes | yes | yes | Experiment/consent id |
| `COMPASS` | yes | yes | no | Set by `/` after sign-in |
| `__Secure-1PSIDTS` | some | yes | no | Timestamp / anti-replay |
| `__Secure-3PSIDTS` | some | yes | no | 3P variant |
| `AEC` | no | no | no | Not observed in signed-in Gemini requests |

### Minimum required set

The SDK currently requires only `__Secure-1PSID` + `__Secure-1PSIDCC`. Based on
the capture, the **minimum cookies that must be sent to Gemini endpoints** are:

- `__Secure-1PSID`
- `__Secure-1PSIDCC`
- `__Secure-1PAPISID` (or the legacy `SAPISID`/`APISID` trio) — needed for
  `Authorization: SAPISIDHASH` on `ogads-pa.clients6.google.com`.
- `SID`, `HSID`, `SSID` — the classic Google auth triad; every signed-in
  Gemini request includes them.

Optional but normally present:

- `__Secure-1PSIDTS` / `__Secure-3PSIDTS` — required by `StreamGenerate` in
  the capture (entry 470). The SDK should warn or fail early if this is
  missing when streaming is requested.
- `COMPASS` — set by `gemini.google.com/` after login; present on all
  subsequent `batchexecute` and `StreamGenerate` calls.
- `SIDCC`, `__Secure-ENID`, `NID` — tracking/consent ids that travel with the
  session.

`AEC` is **not required** for Gemini endpoints; it is a Google Ads cookie and
was not observed in any signed-in Gemini request.

## 4. Recommended `is_signed_in` implementation

A robust check should be three-layered:

1. **Cookie presence** (fast local check):
   - `__Secure-1PSID` and `__Secure-1PSIDCC` are present and non-empty.
   - At least one SAPISID source is available:
     `__Secure-1PAPISID`, `SAPISID`, or `APISID`.
   - Legacy auth cookies `SID`, `HSID`, `SSID` are present.
   - Optional but recommended: `__Secure-1PSIDTS` is present for streaming.

2. **Server-side confirmation** (authoritative):
   - `GET https://gemini.google.com/app?hl={lang}` (or `/` if `/app` is not
     reachable) with the cookie header.
   - Parse `window.WIZ_global_data`.
   - Treat as signed-in only if `S06Grb` is a non-empty numeric string and
     `oPEP7c` is present and looks like an email address.
   - If the response is a redirect to `accounts.google.com` or contains
     `ServiceLogin`, treat as **not signed in**.
   - If `S06Grb` is empty and `oPEP7c` is missing, treat as **not signed in**.

3. **Warm-up health check** (optional, expensive):
   - A cheap `otAQ7b` batchexecute call can confirm that the session is still
     accepted by the backend. Failure with an auth-specific error code means
   "not signed in".

### Pseudo-code

```rust
pub fn is_signed_in(&self) -> bool {
    // Layer 1: local cookie sanity check.
    if self.psid.is_empty() || self.psidcc.is_empty() {
        return false;
    }
    if self.sapisid_value().is_none() {
        return false; // cannot build Authorization: SAPISIDHASH
    }
    // SID/HSID/SSID are not stored as typed fields but live in `extra`.
    for required in &["SID", "HSID", "SSID"] {
        if !self.extra.contains_key(*required) {
            return false;
        }
    }
    true
}

pub async fn check_signed_in_with_server(&self, http: &Client) -> Result<bool> {
    let cookie_header = self.to_header_value();
    let url = "https://gemini.google.com/app?hl=en";

    let response = http
        .get(url)
        .header("Cookie", &cookie_header)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "text/html")
        .send()
        .await?;

    // Detect redirect to accounts.google.com.
    if response.url().host_str() == Some("accounts.google.com") {
        return Ok(false);
    }
    let status = response.status();
    if !status.is_success() {
        return Ok(false);
    }

    let body = response.text().await?;

    // Detect sign-in page.
    if body.contains("ServiceLogin") || body.contains("accounts.google.com/signin") {
        return Ok(false);
    }

    // Extract window.WIZ_global_data and check the primary user id.
    let Some(wiz) = extract_wiz_global_data_block(&body) else {
        return Ok(false);
    };

    let s06grb = extract_quoted_value(wiz, "S06Grb").unwrap_or_default();
    let opep7c = extract_quoted_value(wiz, "oPEP7c");

    let numeric_id = !s06grb.is_empty()
        && s06grb.chars().all(|c| c.is_ascii_digit());

    Ok(numeric_id && opep7c.is_some())
}
```

### Suggested SDK integration

- Keep the existing `is_signed_in()` as a fast local check, but rename or
  extend it to `has_cookie_credentials()` if its semantics change.
- Add a new `GeminiClient::verify_signed_in() -> Result<bool>` that performs
  the server-side `/app` check.
- Call `verify_signed_in()` inside `ensure_session()` before extracting tokens.
  If it returns `false`, return a clear `Error::NotSignedIn` instead of
  letting the first batchexecute call fail.
- When exporting cookies from a browser, require `__Secure-1PSIDTS` to be
  present in the documentation; the SDK can warn if it is missing.

## 5. Verdict

The current `is_signed_in` is too weak: it only looks at two cookies and never
verifies them with the server. The real frontend relies on
`window.WIZ_global_data.S06Grb` and `oPEP7c` in the `/app` HTML response, and
it redirects anonymous users to `accounts.google.com`. The SDK should mirror
that behavior with a cookie pre-check plus a server-side `/app` verification.

**Next step:** implement `Credentials::has_cookie_credentials` and
`GeminiClient::verify_signed_in`, add an `Error::NotSignedIn` variant, and
cover both signed-in and signed-out `/app` shapes with unit tests.
