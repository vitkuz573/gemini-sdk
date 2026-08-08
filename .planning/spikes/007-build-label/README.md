---
spike: 007
name: build-label
validates: "Given the Gemini /app HTML response, automatically extract the current build label (bl) from window.WIZ_global_data.cfb2h instead of using a hardcoded fallback."
verdict: VALIDATED
related: [005-snlM0e, 006-signed-in-detection]
tags: [gemini, protocol, build-label, bl, wiz, reverse-engineering]
---

# Spike 007: Automatically select the Gemini web frontend `bl` parameter

## What This Validates

Given a fresh `GET https://gemini.google.com/` HTML response, when the SDK parses `window.WIZ_global_data`, then it should use the value of the `cfb2h` key as the `bl` query parameter for every `batchexecute` and `StreamGenerate` request, without hardcoding a build label.

## Research

### Current SDK behavior

`src/session.rs` already extracts `build_label` from the `/app` HTML body:

```rust
fn extract_build_label(body: &str) -> Option<String> {
    // Primary: Google stores the build label under the key `cfb2h` inside
    // window.WIZ_global_data in the current HTML shape.
    if let Some(block) = extract_wiz_global_data_block(body) {
        if let Some(label) = extract_quoted_value(block, "cfb2h") {
            if label.starts_with("boq_assistant-bard-web-") && label.len() > 10 {
                return Some(label);
            }
        }
    }

    // Fallback: bare substring search for older or stripped responses.
    for pattern in ["boq_assistant-bard-web-server_", "boq_assistant-bard-web-frontend_"] {
        ...
    }
    None
}
```

`src/client.rs` already consumes `session.build_label` and appends it as the `bl` query parameter in three places:

- `list_models` (`src/client.rs:211`)
- `stream_generate_raw` (`src/client.rs:340`)
- `batchexecute_rpc` (`src/client.rs:590`)

So the extraction and wiring are already implemented. The question addressed by this spike is whether the value being extracted is the *correct* live value and whether a hardcoded fallback is still lurking anywhere.

### Where the frontend gets `bl`

The real Gemini frontend reads `window.WIZ_global_data["cfb2h"]` and stores it as `buildLabel` in the app context. The BardChatUi JS bundle (entry 57 / 266) contains:

```javascript
buildLabel:(b=_.If(UF("cfb2h")))!=null?b:void 0
```

and later:

```javascript
var RQb=class extends _.gl{constructor(a){super();this.buildLabel=a}
ha(a){this.buildLabel&&_.oh(a.ha,"bl",this.buildLabel)}};
```

This confirms that `bl` is literally the value of `cfb2h` from the landing-page `window.WIZ_global_data`.

### Evidence from the HAR

Using the 119 MB capture at `/home/vitaly/mitm.har` (550 entries):

| Metric | Value |
|--------|-------|
| Unique `bl` values on `gemini.google.com` | 1 |
| Total `bl` occurrences on `gemini.google.com` | 92 |
| Value | `boq_assistant-bard-web-server_20260806.17_p0` |
| `cfb2h` in signed-in `/` HTML (entry 264) | `boq_assistant-bard-web-server_20260806.17_p0` |
| `cfb2h` in unsigned `/` HTML (entry 52) | `boq_assistant-bard-web-server_20260806.17_p0` |

`bl` appears on every `batchexecute` and both `StreamGenerate` requests. No `gemini.google.com/_/BardChatUi` request that should carry `bl` is missing it.

The same value is also present in the OneGoogle widget HTML (entry 379), but under a different server label (`boq_onegooglehttpserver_20260802.01_p1`). The Gemini app ignores that label and uses the one from the Gemini landing page.

### No hardcoded fallback in source

A search of `src/` shows `build_label` is only ever sourced from `session.build_label`, which comes from `extract_from_app_html`. There is no `const DEFAULT_BUILD_LABEL` or hardcoded string being used as a fallback when extraction fails.

## How to Run

The spike is research-only; no executable artifact is required. The evidence can be reproduced with:

```bash
python3 - <<'PY'
import json, re
from collections import Counter

with open('/home/vitaly/mitm.har', 'r', encoding='utf-8', errors='replace') as f:
    har = json.load(f)

entries = har['log']['entries']
bl_values = []
for e in entries:
    url = e['request']['url']
    for part in url.split('?', 1)[-1].split('&'):
        if part.startswith('bl='):
            bl_values.append(part[3:])

print(Counter(bl_values))

for i, e in enumerate(entries):
    ctype = next((h['value'] for h in e['response']['headers']
                  if h['name'].lower() == 'content-type'), '')
    if 'text/html' in ctype:
        text = e['response']['content'].get('text', '')
        m = re.search(r'"cfb2h"\s*:\s*"([^"]+)"', text)
        if m:
            print(i, e['request']['url'][:60], '->', m.group(1))
PY
```

## What to Expect

- All signed-in Gemini API calls carry the same `bl` value.
- That value matches `window.WIZ_global_data.cfb2h` from the landing page.
- No other build label is used for `batchexecute` or `StreamGenerate`.

## Investigation Trail

1. **Read prior spikes.** Spike 005 already noted that `cfb2h` is the source of `bl` and that the bare `"bl"` key is not present in `window.WIZ_global_data`. Spike 006 confirmed the same `cfb2h` value appears in both signed-in and unsigned landing pages.
2. **Inspected current SDK code.** `extract_build_label` already targets `cfb2h` and validates the `boq_assistant-bard-web-` prefix. `list_models`, `stream_generate_raw`, and `batchexecute_rpc` all append `bl` from `session.build_label`.
3. **Verified against HAR.** Parsed all 550 entries: 92 `bl` occurrences, one unique value, identical to `cfb2h` in entries 52 and 264.
4. **Confirmed frontend wiring.** JS bundle shows `UF("cfb2h")` → `buildLabel` → `_.oh(a.ha,"bl",this.buildLabel)`, matching the SDK behavior.
5. **Checked for hardcoded fallbacks.** No constant build label exists in `src/`. The only fallback path is a substring search for `boq_assistant-bard-web-server_` / `boq_assistant-bard-web-frontend_`, which is only used if `cfb2h` extraction fails.

## Results

**Verdict: VALIDATED ✓**

The SDK already automatically selects the `bl` parameter from the live `/app` HTML via `window.WIZ_global_data.cfb2h`. There is no hardcoded build label in the request path.

### Surprises / caveats

- The build label is stable for the whole session in this capture (all 92 calls use the same value). We have not observed a mid-session server-side build rotation, but if Google rotates the build label, the SDK will pick up the new value on the next `/app` refresh.
- The fallback substring scan in `extract_build_label` could match a JS bundle URL (`boq-bard-web`) instead of the server build label if the `cfb2h` key is ever renamed. The fallback should probably require the value to start with `boq_assistant-bard-web-` (which it already does for the primary path but not for the fallback prefix scan).
- The unsigned landing page contains the same `cfb2h` value as the signed-in page, so extracting `bl` does not require authentication.

### Recommended implementation (minimal)

No source change is required for the core behavior. Two small hardening steps are recommended:

1. **Strengthen the fallback** so it cannot accidentally grab a JS bundle build name:

   ```rust
   for pattern in ["boq_assistant-bard-web-server_", "boq_assistant-bard-web-frontend_"] {
       if let Some(idx) = body.find(pattern) {
           let area = &body[idx..];
           for end_char in ['"', '\\', '\'', '`'] {
               if let Some(end) = area.find(end_char) {
                   let label = &area[..end];
                   if label.starts_with("boq_assistant-bard-web-") && label.len() > 10 {
                       return Some(label.to_string());
                   }
               }
           }
       }
   }
   ```

2. **Add unit-test fixtures** covering:
   - `cfb2h` inside a full `window.WIZ_global_data` block.
   - `cfb2h` on the unsigned landing page (empty `S06Grb`).
   - A fallback HTML shape where the key is absent but the server label string is present.

### Impact on remaining work

- Spike 004 (WAA token) remains the blocking item for fully automated image uploads; `bl` selection is not a blocker.
- Spike 006 (signed-in detection) can rely on the same `/app` HTML parse path that already extracts `cfb2h`.
