---
spike: 001
name: har-api-coverage
validates: "Given the current 135 MB HAR capture at ~/mitm.har, compare the observed undocumented Gemini web frontend traffic with the current gemini-sdk implementation and report coverage, gaps, and protocol drift."
verdict: VALIDATED
related: [003-gemini-protocol]
tags: [gemini, har, api-coverage, reverse-engineering, batchexecute, stream-generate, waa, ogads]
---

# Spike 001: HAR API Coverage Audit (`~/mitm.har`)

## What This Validates

Whether the current Rust SDK (`gemini-sdk`) covers the undocumented API surface visible in a fresh 135 MB MITM capture of the Gemini web frontend, and what new or changed endpoints/RPCs appear.

## Research

- The HAR file at `~/mitm.har` contains **863 entries** (135 MB).
- The capture includes sign-in flow, app bootstrap, chat turns (text + image), file upload, and account navigation (`/usage`, `/scheduled`).
- Focus analysis excluded Google Ads, analytics, Chrome update, and static JS bundle requests.

## How to Run

```bash
cd /home/vitaly/projects/gemini-sdk
python3 - <<'PY'
import json
with open('/home/vitaly/mitm.har') as f:
    data = json.load(f)
print('entries:', len(data['log']['entries']))
PY
```

Reproducible artifacts are in this directory:

- `har_summary.json` — top-level counts and host distribution.
- `batchexecute_rpcids.json` — all observed Gemini `batchexecute` RPC ids with decoded inner payloads.
- `streamgenerate_slots.json` — non-null slot indexes for each captured `StreamGenerate`.

## What to Expect

A concrete mapping between every major request class in the HAR and the SDK module that handles it (or does not).

## Investigation Trail

1. Loaded HAR; counted 863 entries, 420 unique host+path endpoints.
2. Focused on Gemini-specific hosts (`gemini.google.com`, `push.clients6.google.com`, `waa-pa.clients6.google.com`, `ogads-pa.clients6.google.com`, `signaler-pa.clients6.google.com`, `myactivity.google.com`).
3. Normalized JS bundle URLs and counted 33 unique Gemini-relevant endpoints.
4. Extracted 23 unique `batchexecute` RPC ids on `gemini.google.com`.
5. Decoded 2 `StreamGenerate` request bodies (text fresh + image continuation) and compared slot layout with `src/proto/slots.rs`.
6. Compared WAA/ogads headers and payloads with `src/client.rs`.
7. Cross-referenced findings with prior spikes 001, 002, 003 in `.planning/spikes/`.

## Results

### Endpoint Coverage Map

| Host / Endpoint | Count in HAR | SDK Module | Coverage |
|-----------------|--------------|------------|----------|
| `GET gemini.google.com/` / `/app` | 2 | `client::fetch_app_page` | Full |
| `POST gemini.google.com/_/BardChatUi/data/batchexecute` | 98 | `client::batchexecute_rpc` | Partial |
| `POST gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate` | 2 | `client::stream_generate_raw` | Full (structure) |
| `POST gemini.google.com/_/BardChatUi/cspreport/fine-allowlist` | 2 | — | Not covered |
| `POST gemini.google.com/_/BardChatUi/jserror` | 1 | — | Not covered |
| `POST gemini.google.com/_/BardChatUi/web-reports` | 1 | — | Not covered |
| `POST push.clients6.google.com/upload/` | 4 | `upload` | Full |
| `POST waa-pa.clients6.google.com/$rpc/google.internal.waa.v1.Waa/Create` | 2 | `client::waa_create` | Full |
| `POST ogads-pa.clients6.google.com/$rpc/.../GetAsyncData` | 8 | `client::ogads_get_async_data` | Full |
| `GET/POST signaler-pa.clients6.google.com/punctual/...` | 48 | — | Not covered |
| `GET/POST myactivity.google.com/...` | 15 | — | Not covered |
| `GET gemini.gstatic.com/_/mss/boq-bard-web/_/js/<bundle>` | 109 | — | Not applicable (static JS) |

### batchexecute RPC Coverage

SDK currently implements **4** Gemini `batchexecute` RPCs:

- `otAQ7b` — model list / warm-up
- `sJBwce` — WAA prerequisite `[[1,2]]`
- `ESY5D` — feature flags
- `K4WWud` — locale/geo

The HAR contains **19 additional** `gemini.google.com` `batchexecute` RPCs that the SDK does not implement:

| RPC | Source-path | Inner payload (decoded) | Likely purpose |
|-----|-------------|-------------------------|----------------|
| `L5adhe` | `/`, `/app` | `[[null,...null, "cf41b0e0dd7d53e5"], [["last_selected_mode_id_on_web"]]]` | user prefs / last mode |
| `aPya6c` | `/app` | `[]` | heartbeat / telemetry |
| `cYRIkd` | `/app` | `["ru"]` | locale tools |
| `whPPme` | `/app` | `["ru", null, [4]]` | locale/model config |
| `maGuAc` | `/app` | `[1]` | telemetry flag |
| `GPRiHf` | `/app` | `[]` | telemetry |
| `Te6DCf` | `/app` | `[["ru"], [1,2]]` | locale/config |
| `o30O0e` | `/app` | `[["me"], [[["person.photo","person.name","person.email"]], null, [1,7]]]` | user info |
| `CNgdBe` | `/app` | `[1, ["ru"], 0]` | config |
| `ozz5Z` | `/app` | `[[[null,"1",447],null,1], ...]` | feature rollout list |
| `I4z33b` | `/app` | `[]` | telemetry |
| `ku4Jyf` | `/app` | `["ru",null,null,null,4,null,null,[1,3,7,17],null,[]]` | locale/tools |
| `VxUbXb` | `/app` | `[]` | telemetry |
| `qpEbW` | `/app` | `[[[1,11],[2,11],[6,11]]]` | telemetry / impressions |
| `Bsxleb` | `/app` | `[[76091940,null,26],null,35,null,null,null,[null,null,"<uuid>"]]` | config/state |
| `MyzX6c` | `/app` | `[]` | telemetry |
| `PCck7e` | `/app/<conversation>` | `["r_0958d664053635a6"]` | conversation action (e.g. regenerate/rating) |
| `jSf9Qc` | `/usage` | `[]` | usage page data |
| `XPSWpd` | `/scheduled` | `[]` | scheduled prompts data |

None of these additional RPCs are required for the core chat flow observed in the HAR (`/app` init → `otAQ7b` → `sJBwce` → `Waa/Create` → `ogads` → `ESY5D` → optional `cYRIkd`/`Te6DCf`/`K4WWud` → `StreamGenerate`). They are UI-support calls (user info, telemetry, settings pages, history).

### StreamGenerate Slot Comparison

Captured `StreamGenerate` bodies match the SDK's 97-slot structure well:

| Slot | HAR (text fresh) | HAR (image continuation) | SDK (`src/proto/slots.rs`) | Status |
|------|------------------|--------------------------|----------------------------|--------|
| 0 | prompt + attachments | prompt + attachments | matches | OK |
| 1 | `["ru"]` | `["ru"]` | uses `session.language` | OK |
| 2 | single array | single array with continuation | `ConversationState::to_slot2` | OK |
| 3 | `!...` WAA token | `!...` WAA token | from `browser-attestation` or default | OK (feature) |
| 4 | 32-hex nonce | 32-hex nonce | `fresh_request_nonce` | OK |
| 6 | `[1]` | `[1]` | `[1]` | OK |
| 7 | `1` | `1` | `1` | OK |
| 10 | `1` | `1` | `1` | OK |
| 11 | `0` | `0` | `0` | OK |
| 17 | `[[0]]` | `[[1]]` | `[[0]]` fresh / `[[1]]` cont | OK |
| 18 | `0` | `0` | `0` | OK |
| 27 | `1` | `1` | `1` | OK |
| 30 | `[4]` | `[4]` | `[category.as_enum_value()]` | OK |
| 41 | `[1]` | `[1]` | `[1]` | OK |
| 53 | `0` | `0` | `0` | OK |
| 59 | UUID | UUID | `fresh_request_uuid` | OK |
| 61 | `[]` | `[]` | `[]` | OK |
| 66 | `null` | `null` | `null` | OK |
| 68 | `2` | `2` | `2` | OK |
| 79 | `3` | `3` | `3` | OK |
| 80 | `1` | `1` | default `Standard` | OK |
| 91 | `0` | `0` | `0` | OK |
| 96 | `1` fresh | `0` continuation | `1` fresh / `0` cont | OK |

### Header Differences

| Header | HAR | SDK | Status |
|--------|-----|-----|--------|
| `x-client-data` | `CNeOywE=` | `CI7yygE=` | Drift; SDK constant is older |
| `x-goog-ext-525001261-jspb` | `[1,null,null,null,"e6fa609c3fa255c0",null,null,0,[4,5,6,8],null,null,2,null,null,3,1,"<uuid>"]` | builds same shape | OK |
| `x-goog-ext-525005358-jspb` | `["<uuid>",1]` | `["<uuid>",1]` | OK |
| `x-goog-ext-73010989-jspb` | `[0]` | `[0]` | OK |
| `x-goog-ext-73010990-jspb` | `[0,0,0]` | `[0,0,0]` | OK |

### WAA / ogads Observations

- `Waa/Create` body in HAR: `["br1aemAN9owlYRs9NnsA"]` — matches SDK.
- `ogads GetAsyncData` body in HAR varies by origin:
  - For `gemini.google.com`: `[658,"https://gemini.google.com/",658,"ru","ch",1,null,0,0,"","",1,0,null,103135050,[[1,9,13],0,1,1],[1],null,1,0,"<base64>",{"1001":0}]`
  - SDK body matches the Gemini origin variant.
- `Authorization: SAPISIDHASH ...` is present on ogads calls in the HAR; SDK adds it when credentials include SAPISID.

### Gaps Identified

1. **Telemetry / reporting endpoints are not implemented** — intentionally out of scope for a library SDK:
   - `/_/BardChatUi/cspreport/fine-allowlist`
   - `/_/BardChatUi/jserror`
   - `/_/BardChatUi/web-reports`
   - `signaler-pa` channels
   - `myactivity.google.com` history endpoints
2. **Settings / history page RPCs are not implemented** — also out of scope:
   - `/usage`, `/scheduled`, `/personalization-settings` page data RPCs.
3. **`x-client-data` constant drifted** — SDK uses `CI7yygE=`, HAR shows `CNeOywE=`. This is a low-risk, easy-update constant.
4. **`PCck7e` conversation actions** — SDK does not expose rating, regenerate, or delete-turn actions.

## Verdict

**VALIDATED** — for the core chat flow (session init, model listing, text/image chat, file upload, WAA/ogads attestation), the SDK covers the undocumented API surface observed in `~/mitm.har`. The remaining uncovered endpoints are UI telemetry, settings/history pages, and reporting, which are intentionally outside the SDK's scope. The only concrete drift is the `x-client-data` header constant.
