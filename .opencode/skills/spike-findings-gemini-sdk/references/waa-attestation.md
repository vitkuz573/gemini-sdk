# WAA / BotGuard Attestation

## Requirements

- Session initialization must obtain WAA/attestation context (`x-goog-ext-525001261-jspb`) without Chrome automation.
- The WAA token must feed into `StreamGenerate` slot 3 when available.

## How to Build It

### WAA initialization chain

Call in order after `/app` extraction and before the first `StreamGenerate`:

1. `batchexecute?rpcids=sJBwce` with body `[[["sJBwce","[[1,2]]",null,"generic"]]]`.
2. `POST https://waa-pa.clients6.google.com/$rpc/google.internal.waa.v1.Waa/Create`
   ```http
   Content-Type: application/json+protobuf
   x-goog-api-key: AIzaSyBGb5fGAyC-pRcRU6MUHb__b_vKha71HRE
   x-user-agent: grpc-web-javascript/0.1
   x-client-data: CNeOywE=
   Origin: https://gemini.google.com
   Referer: https://gemini.google.com/
   Body: ["br1aemAN9owlYRs9NnsA"]
   ```
3. `POST https://ogads-pa.clients6.google.com/$rpc/google.internal.onegoogle.asyncdata.v1.AsyncDataService/GetAsyncData`
   ```http
   Content-Type: application/json+protobuf
   x-goog-api-key: AIzaSyCbsbvGCe7C9mCtdaTycZB2eUFuzsYKG_E
   Authorization: SAPISIDHASH <ts>_<sha1> SAPISID1PHASH ... SAPISID3PHASH ...
   x-client-data: CNeOywE=
   Origin: https://gemini.google.com
   Referer: https://gemini.google.com/
   Body: [658,"https://gemini.google.com/",658,"ru","ch",1,null,0,0,"","",1,0,null,103135050,[[1,9,13],0,1,1],[1],null,1,0,"<base64>",{"1001":0}]
   ```
4. `batchexecute?rpcids=ESY5D` with body `[[["ESY5D","[null,[5]]",null,"generic"]]]`.

### Building `x-goog-ext-525001261-jspb`

Use this fixed template, substituting the request UUID:

```json
[1,null,null,null,"e6fa609c3fa255c0",null,null,0,[4,5,6,8],null,null,2,null,null,3,1,"<request_uuid>"]
```

The fingerprint `e6fa609c3fa255c0` is the Pro model id found in the `otAQ7b` response and in `ESY5D` feature flags.

### Computing `Authorization: SAPISIDHASH`

```rust
let timestamp = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs();
let input = format!("{} {} {}", origin, timestamp, sapisid);
let hash = hex::encode(sha1::Sha1::digest(input.as_bytes()));
let auth = format!("SAPISIDHASH {}_{}", timestamp, hash);
```

Origin must be `https://gemini.google.com` (no trailing slash for the hash).

## What to Avoid

- Do not treat the `Waa/Create` response token as slot 3 directly; the captured slot 3 is a different client-derived blob.
- Do not omit `Authorization` on `ogads GetAsyncData`; it is required.
- Do not send the wrong `x-goog-api-key` for each service; WAA and ogads use different keys.
- Do not assume the WAA chain always succeeds; failures should be non-fatal and fall back to no attestation.

## Constraints

- The `br1aemAN9owlYRs9NnsA` body parameter for `Waa/Create` is hardcoded from captured traffic and may rotate with JS builds.
- `ogads GetAsyncData` response is often an empty placeholder `[null,...,0]`; the useful context is assembled client-side.
- Without browser automation, generating a valid slot 3 may fail if Google enforces a BotGuard challenge. In that case, the `browser-attestation` feature (Chrome CDP) remains the fallback.

## Origin

Synthesized from spikes: 003-gemini-protocol, 004-waa-token.
Source files available in: `sources/003-gemini-protocol/`, `sources/004-waa-token/`.
