# Gemini Web Frontend Protocol

## Requirements

- `StreamGenerate` must use a 97-slot `inner_req_list` that matches the live frontend.
- Model listing must use the correct `batchexecute` RPC id (`otAQ7b`).
- The `bl` (build label) query parameter must be extracted from the live `/app` HTML (`window.WIZ_global_data.cfb2h`), not hardcoded.
- Upload flow must remain compatible with `push.clients6.google.com/upload` resumable uploads.

## How to Build It

### 1. Session bootstrap

1. `GET https://gemini.google.com/app?hl={language}` with cookies.
2. Parse `window.WIZ_global_data` for:
   - `SNlM0e` → `at` token
   - `FdrFJe` → `f.sid`
   - `cfb2h` → `bl` (build label)
   - `qKIAYe`/`KnDnFf` → `push-id`
   - `S06Grb` and `oPEP7c` → signed-in verification
3. Accept consent banner if present by posting the `data-payload` save URL.

### 2. Warm-up / model list

```rust
self.batchexecute_rpc(
    "otAQ7b",
    build_batchexecute_body(at.as_deref()),
    language,
    build_label.as_deref(),
    session_id.as_deref(),
    cookie_header,
    Some("/"),
).await?;
```

Use `source-path=/` for the first call, `/app` afterwards. Extract the Pro model fingerprint (16-char hex, e.g. `e6fa609c3fa255c0`) from the response for the WAA context header.

### 3. StreamGenerate request

```rust
let url = format!(
    "{WEB_BASE_URL}/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate"
);
let params = vec![
    ("hl", session.language.clone()),
    ("_reqid", request_uuid.clone()),
    ("rt", "c".to_string()),
    // optional:
    ("bl", build_label.clone()),
    ("f.sid", session_id.clone()),
];
```

Body is `application/x-www-form-urlencoded` with:

- `f.req=[null,"<inner_req_list JSON>"]`
- `at=<SNlM0e token>`

### 4. 97-slot `inner_req_list`

Key slots (all others `null`):

| Slot | Meaning | Value |
|------|---------|-------|
| 0 | prompt + attachments | `[prompt, 0, null, attachments, null, null, 0]` |
| 1 | language | `["ru"]` |
| 2 | conversation state | fresh: `["","",null,...,null,""]`; continuation: `[c_id, r_id, rp_id, null,..., token]` |
| 3 | WAA token | long `!...` blob or `Value::Null` if unavailable |
| 4 | nonce | 32-char hex |
| 6 | new dialog flag | `[1]` |
| 7 | unknown constant | `1` |
| 10 | unknown constant | `1` |
| 11 | unknown constant | `0` |
| 17 | turn counter | fresh `[[0]]`, continuation `[[1]]` |
| 18 | unknown constant | `0` |
| 27 | unknown constant | `1` |
| 30 | model category | `[4]` for `Auto` |
| 41 | mode picker | `[1]` |
| 53 | unknown constant | `0` |
| 59 | request UUID | uppercase UUID |
| 61 | empty array | `[]` |
| 66 | unknown | `null` |
| 68 | unknown constant | `2` |
| 79 | unknown constant | `3` |
| 80 | thinking level | `1` Standard, `2` Extended, `3` DeepThink |
| 91 | unknown constant | `0` |
| 96 | fresh/continuation | `1` fresh, `0` continuation |

### 5. Required headers

```http
Content-Type: application/x-www-form-urlencoded;charset=UTF-8
Origin: https://gemini.google.com
Referer: https://gemini.google.com/
x-client-data: CNeOywE=
x-goog-ext-525001261-jspb: [1,null,null,null,"e6fa609c3fa255c0",null,null,0,[4,5,6,8],null,null,2,null,null,3,1,"<request_uuid>"]
x-goog-ext-525005358-jspb: ["<request_uuid>",1]
x-goog-ext-73010989-jspb: [0]
x-goog-ext-73010990-jspb: [0,0,0]
```

### 6. Response parsing

Response body is WIZ frames prefixed with `)] }' \n\n`. Each frame is a JSON line:

```json
["wrb.fr", null, "[null,[\"c_...\",\"r_...\"],{\"18\":\"r_...\",\"21\":[\"<continuation_token>\"],\"44\":true}]"]
```

- Meta frame: extract `conversation_id`, `response_id`, `continuation_token` (key `21` or `26`).
- Text frame: `response_part_id` + text strings.
- Thinking frame: `part[37][0]` list of markdown strings.

### 7. File upload

1. **Start**
   ```http
   POST https://push.clients6.google.com/upload/
   x-goog-upload-command: start
   x-goog-upload-header-content-length: <bytes>
   x-goog-upload-protocol: resumable
   x-tenant-id: bard-storage
   push-id: <push-id>
   Content-Type: application/x-www-form-urlencoded;charset=UTF-8
   Body: File name: <filename>
   ```
2. Read `x-goog-upload-url` from response.
3. **Finalize**
   ```http
   POST <x-goog-upload-url>
   x-goog-upload-command: upload, finalize
   x-goog-upload-offset: 0
   x-tenant-id: bard-storage
   push-id: <push-id>
   Content-Type: image/png
   Body: <raw bytes>
   ```
4. Response body is the `contrib_service` reference path used in slot 0.

## What to Avoid

- Do not hardcode `bl` or `f.sid`; always extract from `/app` HTML.
- Do not send `pageId=none` or `authuser=0` on `StreamGenerate`; the live frontend does not.
- Do not wrap slot 2 in an extra array for fresh conversations.
- Do not omit `x-goog-ext-525001261-jspb`; the server rejects requests without the WAA context.
- Do not use `source-path=/app` for the very first `otAQ7b`/`sJBwce` warm-up calls.

## Constraints

- `x-client-data` drifts with Chrome versions; captured value is `CNeOywE=` (was `CI7yygE=` in older spikes).
- Model category enum values: Auto=4, Pro=3, etc. — verify against `ModelCategory::as_enum_value`.
- The WAA token in slot 3 is ~2600–2700 chars and prefixed with `!`. It is not the raw `Waa/Create` response.
- `push-id` defaults to `feeds/mcudyrk2a4khkz` but should be extracted from `/app` when possible.

## Origin

Synthesized from spikes: 001-gemini-protocol, 002-gemini-protocol, 003-gemini-protocol, 007-build-label, 009-har-api-coverage.
Source files available in: `sources/001-gemini-protocol/`, `sources/002-gemini-protocol/`, `sources/003-gemini-protocol/`, `sources/007-build-label/`, `sources/009-har-api-coverage/`.
