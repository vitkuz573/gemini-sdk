# Spike 005: SNlM0e extraction and usage in Gemini web frontend

**HAR source:** `.planning/spikes/004-waa-token/data/mitm.har`  
**Scope:** Research only. No SDK source code was modified.

## 1. Summary

`SNlM0e` is a session-scoped authorization token rendered by the Gemini web server into `window.WIZ_global_data` inside the initial `/app` HTML response. The frontend reads it via the standard Google WIZ accessor `_.Bd("SNlM0e")` and passes it to outgoing `batchexecute` and `StreamGenerate` requests as the URL query parameter `at`.

The current SDK code in `src/session.rs::extract_snlim0e` already parses `"SNlM0e":"..."` from the HTML body and successfully extracts the token observed in this HAR. No logic change is strictly required for the current Google HTML shape, but the implementation is fragile: it depends on a specific JSON serialization order and does not validate that the token is being read from `window.WIZ_global_data`.

`SNlM0e` is **not** sent as an HTTP `Authorization` header by the Gemini frontend. Instead, the browser's normal Google cookie-based SAPISID authorization is handled by the closure library (`_.jp` / `LNa` / `KNa` hashes). `SNlM0e` is an additional per-session token that proves the page was fetched recently.

## 2. All `SNlM0e` occurrences in the HAR

| Entry | URL (truncated) | Content-Type | `SNlM0e` value | Context |
|-------|-----------------|--------------|----------------|---------|
| 57 | `https://gemini.gstatic.com/_/mss/boq-bard-web/_/js/...BardChatUi...` | `text/javascript` | *(used as key, not value)* | `TQb(a).configure(_.If(_.Bd("SNlM0e")),_.If(_.Bd("S06Grb")));` |
| 58 | same bundle (duplicate load) | `text/javascript` | *(key only)* | same as 57 |
| 155 | `https://accounts.google.com/v3/signin/identifier?continue=https://gemini.google.com/...` | `text/html` | `ALX_P8uawyPdkMdLdAnT4PUgXauZ:1786124428553` | inline `window.WIZ_global_data` during sign-in flow |
| 162 | `https://www.gstatic.com/_/mss/boq-identity/_/js/...AccountsSignInUi...` | `text/javascript` | *(key only)* | `b.configure(_.Xg("SNlM0e").string(null),_.Xg("S06Grb").string(null));` |
| 235 | `https://accounts.google.com/encryption/unlock/writeverifier?...` | `text/html` | `ANhjawXPjAkEcdxXRq6P2kPf2Da9:1786124502844` | inline `window.WIZ_global_data` during sign-in |
| 239 | `https://www.gstatic.com/_/mss/boq-identity/_/js/...AccountsKeychainDataRelayUi...` | `text/javascript` | *(key only)* | `b.configure(_.fk("SNlM0e").string(null),_.fk("S06Grb").string(null));` |
| **264** | **`https://gemini.google.com/`** | **`text/html`** | **`ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132`** | **canonical `window.WIZ_global_data` on the Gemini landing page** |
| 266 | `https://gemini.gstatic.com/_/mss/boq-bard-web/_/js/...BardChatUi...` | `text/javascript` | *(key only)* | same configure call as 57 |
| 272 | same bundle as 266 (duplicate load) | `text/javascript` | *(key only)* | same configure call |
| 379 | `https://ogs.google.com/u/0/widget/app?...origin=https%3A%2F%2Fgemini.google.com...` | `text/html` | `AC2YjkUsEtt-iytn0i_IvFy_V5ue:1786124586236` | inline `window.WIZ_global_data` in the OneGoogle bar widget |
| 389 | `https://www.gstatic.com/_/mss/boq-one-google/_/js/...OneGoogleWidgetUi...` | `text/javascript` | *(key only)* | `b.configure(_.ji("SNlM0e").string(null),_.ji("S06Grb").string(null));` |

The actual token consumed by Gemini API calls is the one from entry **264**.

## 3. How the browser extracts `SNlM0e`

The server emits the token inside a `<script>` block as part of `window.WIZ_global_data`:

```html
<script nonce="...">
window.WIZ_global_data = {
  ...
  "FdrFJe":"-1594710263937718439",
  ...
  "S06Grb":"111628289675248526498",
  "S6lZl":103135050,
  "SNlM0e":"ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132",
  ...
};
</script>
```

The base JS module defines:

```javascript
fea=function(a=window){return a.WIZ_global_data};
_.gea=function(a,b=window){return(b=fea(b))&&a in b?b[a]:null};
_.Bd=function(a,b=window){return new _.Ad(a,_.gea(a,b))};
_.If=function(a){var b=a.ka;if(b==null)return null;if(typeof b==="string")return b;throw Gja(a,"string");};
```

So reading `SNlM0e` is equivalent to:

```javascript
window.WIZ_global_data["SNlM0e"]
```

The value is a **string**, not a base64 blob, not a cookie, and not computed from cookies.

## 4. How the frontend consumes `SNlM0e`

### 4.1 App initialization

In the BardChatUi bundle (entry 266/272):

```javascript
TQb(a).configure(_.If(_.Bd("SNlM0e")), _.If(_.Bd("S06Grb")));
```

`TQb` creates a `DefaultDataAppContext`. The configure call stores the token and the session/user id (`S06Grb`) in the app context so that downstream RPC builders can attach them.

### 4.2 Request authorization

The closure authorization helper builds the `Authorization` header from cookies, **not** from `SNlM0e`:

```javascript
_.jp=function(a){
  var b=JNa(_.ha==null?void 0:_.ha.location.href), c=[], d;
  (d=_.ha.__SAPISID||_.ha.__APISID||_.ha.__3PSAPISID||_.ha.__1PSAPISID||_.ha.__OVERRIDE_SID)?d=!0
    :(typeof document!=="undefined"&&(d=new _.FNa(document),d=d.get("SAPISID")||d.get("APISID")||d.get("__Secure-3PAPISID")||d.get("__Secure-1PAPISID")),d=!!d);
  if(d){
    var e=(d=b=b.indexOf("https:")==0||...)?_.ha.__SAPISID:_.ha.__APISID;
    e||typeof document==="undefined"||(e=new _.FNa(document),e=e.get(d?"SAPISID":"APISID")||e.get("__Secure-3PAPISID"));
    (d=e?LNa(e,d?"SAPISIDHASH":"APISIDHASH",a):null)&&c.push(d);
    ...
  }
  return c.length==0?null:c.join(" ");
};
```

`LNa` / `KNa` compute SAPISIDHASH from the cookie value and origin using a SHA-1-like digest.

For `StreamGenerate`, the frontend builds:

```javascript
v.append("Content-Type","application/json+protobuf");
v.append("Accept","text/event-stream");
p&&v.append("X-Goog-Api-Key",p);
(p=_.jp([]))&&v.append("Authorization",p);
v.append("X-Goog-AuthUser",a.Sa.Oz());
```

So `SNlM0e` does **not** appear in headers. It is sent as the `at=` query parameter (see section 5).

### 4.3 What `S06Grb` is used for

`S06Grb` is the Google account obfuscated Gaia ID (`111628289675248526498`). It is read alongside `SNlM0e` and consumed by the OneGoogle bar widget as well as the Gemini app context. It is not sent directly in API URLs.

## 5. `SNlM0e` on the wire

Both `batchexecute` and `StreamGenerate` send the token as `at` in the request body (form-encoded) or URL query.

Examples from the HAR:

| Entry | Endpoint | `at` value |
|-------|----------|------------|
| 284 | `/_/BardChatUi/data/batchexecute?rpcids=otAQ7b...` | `ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132` |
| 470 | `.../StreamGenerate?bl=...&f.sid=...` | `ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132` |
| 504 | `.../StreamGenerate?bl=...&f.sid=...` | `ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132` |

Decoded `f.req` from entry 284 shows that `at` is a separate form field:

```
f.req=[[["otAQ7b","[]",null,"generic"]]]
&at=ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132
&
```

## 6. Related tokens in the same HTML response (entry 264)

| Key | Value in HAR | Likely purpose |
|-----|--------------|----------------|
| `FdrFJe` | `-1594710263937718439` | Frontend session id; sent as `f.sid` query param on every batchexecute/StreamGenerate call. |
| `qKIAYe` | `feeds/mcudyrk2a4khkz` | Push channel feed id; used for server-push / streaming registration. |
| `KnDnFf` | `feeds/nrij2vo2gajxiu` | Alternate push feed id. SDK prefers `qKIAYe`, falls back to this. |
| `S06Grb` | `111628289675248526498` | Obfuscated Google account Gaia id; read with `SNlM0e` by app context. |
| `S6lZl` | `103135050` | Numeric user id / dpi-like identifier used by OneGoogle widget. |
| `TSDtV` | `%.@.[[null,[[45780889,...` | Feature-flag/experiment vector (WIZ array encoding). Not used directly in API calls. |
| `cfb2h` | `boq_assistant-bard-web-server_20260806.17_p0` | Build label (`bl` query param). |
| `eptZe` | `/_/BardChatUi/` | Base path for data endpoints. |

`bl` (the bare `"bl"` key the SDK regex looks for) does **not** appear as a top-level key in `window.WIZ_global_data` in this capture; the build label lives under `cfb2h`.

## 7. Comparison with `src/session.rs`

Current `extract_snlim0e`:

```rust
fn extract_snlim0e(body: &str) -> Option<String> {
    if let Some(idx) = body.find("\"SNlM0e\":\"") {
        let start = idx + "\"SNlM0e\":\"".len();
        if let Some(end) = body[start..].find('"') {
            let token = &body[start..start + end];
            if token.len() > 10 {
                return Some(token.to_string());
            }
        }
    }
    // fallback branch ...
}
```

- **Works on the current HAR:** it finds `"SNlM0e":"ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132"` and returns the correct token.
- **Matches the browser extraction pattern:** the browser reads `window.WIZ_global_data["SNlM0e"]`, which is exactly the JSON literal the regex targets.
- **Mismatches / fragility:**
  1. It does not anchor the search to `window.WIZ_global_data`. A later unrelated occurrence of the string could produce a false positive.
  2. It assumes the value is double-quoted. If Google switches to single quotes or escapes, it breaks.
  3. The fallback branch `body.find("SNlM0e")` is too greedy and could grab arbitrary text.
  4. It does not validate the token shape beyond `len() > 10`.

## 8. Recommendations

1. **Scope the extraction** to the `window.WIZ_global_data = { ... }` block, or at least verify that the match is inside that block.
2. **Use a single robust regex** such as:
   ```rust
   static ref SNLIM0E_RE: Regex = Regex::new(
       r#"window\.WIZ_global_data\s*=\s*\{[^}]*"SNlM0e"\s*:\s*"([^"]+)""#
   ).unwrap();
   ```
   (with a fallback to a non-anchored `"SNlM0e":"([^"]+)"` if the block start is ever stripped).
3. **Validate the extracted token** against a pattern like `^[A-Za-z0-9_-]+:\d{13}$` (observed format: base64-url-ish prefix + colon + 13-digit timestamp). This rejects HTML fragments or escaped values.
4. **Do not treat `SNlM0e` as an Authorization header.** Continue sending it as the `at` form/query parameter.
5. **Update tests/fixtures** to include a real `window.WIZ_global_data` snippet instead of a bare token string.
6. **Investigate token expiry:** the suffix is a Unix timestamp in milliseconds (`1786124577132` ≈ 2026-08-06). The SDK may need to refresh the `/app` page when requests fail with an expired-token error.

## 9. What else is needed for robust extraction

- A reliable way to obtain the initial `/app` HTML with the user's authenticated cookies.
- Cookie handling for `APISID` / `SAPISID` / `__Secure-1PAPISID` so that the requests can actually be authorized (the header auth is cookie-derived, `SNlM0e` alone is not enough).
- Handling of consent/redirect pages where `window.WIZ_global_data` may be absent or the token may be empty.
- Detection of the build label via `cfb2h` as a primary source, because bare `"bl"` is not present in this capture.
