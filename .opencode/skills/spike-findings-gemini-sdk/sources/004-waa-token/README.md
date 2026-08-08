---
spike: 004
name: waa-token
validates: "Reverse-engineer the BotGuard WAA token that goes into StreamGenerate slot 3 using only captured artifacts, without browser automation."
verdict: IN PROGRESS
related: [003-gemini-protocol]
tags: [gemini, waa, botguard, reverse-engineering, attestation, slot-3]
---

# Spike 004: Reverse WAA / BotGuard token for StreamGenerate slot 3

## Data artifacts in this directory

| File | Description |
|------|-------------|
| `mitm.har` | Full 119 MB capture with response bodies (550 entries). |
| `gemini_cookies.env` | Cookies and push id used during capture. |
| `botguard.js` | BotGuard VM loaded from `//www.google.com/js/bg/<id>.js` (66 KB). |
| `pairs.json` | Extracted WAA/Create payloads and StreamGenerate slot 3 values. |
| `stream_slots.json` | Full slot dumps for entries 470 and 504. |
| `waa_token_decoded.bin` | Base64url-decoded WAA/Create challenge token (23 KB). |
| `slot3_470_decoded.bin` | Base64url-decoded slot 3 for text StreamGenerate (1.9 KB). |
| `slot3_504_decoded.bin` | Base64url-decoded slot 3 for image StreamGenerate (1.9 KB). |

## What we already know

- `Waa/Create` returns a JSON+protobuf array:
  - `[0]` = `"bfkj"`
  - `[2][3]` = BotGuard JS URL, e.g. `//www.google.com/js/bg/kyf1VDjbHWvMTnIvog5EF0ApHEYdQKtekJaC4TVIw1c.js`
  - `[3]` = token id (same id as in the JS URL)
  - `[4]` = challenge token (~31 KB, base64url alphabet)
  - `[5]` = `"botguard"`
- The challenge token base64url-decodes to ~23 KB of binary data.
- `StreamGenerate` slot 3 is a `!`-prefixed base64url string:
  - text chat (entry 470): 2607 chars → 1954 bytes decoded
  - image chat (entry 504): 2653 chars → 1989 bytes decoded
- Slot 3 decoded payload is **binary**, not text, and is **not** a substring of the WAA challenge token.
- The two slot-3 payloads share a very long common suffix; only the first few bytes differ.
- BotGuard JS is an obfuscated VM. Entry point `botguard.bg(M, callback)` accepts the WAA challenge string `M` and invokes `callback(token)`. Inside it creates iframes, measures timing, and uses `atob` / `eval`.

## Why pure-HTTP reverse is hard

The VM is self-modifying / uses closure-based dispatch (`T`, `E`, `H`, `u`) and appears to:
1. Parse the challenge string into an internal program / bytecode.
2. Create hidden iframes and measure browser/environment timing.
3. Collect signals from `document`, `navigator`, `performance`, `localStorage`, etc.
4. Produce a short proof-of-work / attestation token.

A minimal Node.js emulation fails because the VM expects a real DOM with iframe `load`/`error` events, `trustedTypes`, `requestIdleCallback`, etc.

## What is still needed

To complete this spike without a browser we need **more ground-truth data** that links inputs to outputs. Specifically:

1. **Multiple independent sessions / WAA challenges + their resulting slot-3 tokens**
   - At least 5–10 fresh `Waa/Create` responses and the exact slot-3 token that followed each one.
   - This lets us look for deterministic transforms, XOR patterns, or length/prefix correlations.

2. **Same WAA challenge reused at different times / with different requests**
   - If we can replay a challenge token and get the same slot-3, the algorithm is deterministic in time.
   - If slot-3 changes, it includes a time/random component.

3. **Full browser DevTools instrumentation capture**
   - Console log of the exact call to `botguard.bg(...)` / `bg(...)` with all arguments.
   - Call stack at the moment of invocation.
   - Any global variables set just before/after (e.g., `window.bg`, `window.botguard`, `window.___jsl`).

4. **The exact DOM / JS environment state during challenge execution**
   - Snapshot of `document.documentElement.innerHTML` right before `Waa/Create`.
   - Values of `localStorage` / `sessionStorage` keys starting with `_` or `google`.
   - All cookies at the moment of invocation.

5. **All JS files loaded between `Waa/Create` and the next `StreamGenerate`**
   - Current HAR records URLs but not bodies for most gstatic bundles.
   - Need a new HAR/mitm dump with **Save response bodies enabled** for JS.
   - Alternatively, fetch each URL with the same `User-Agent`/`Cookie`/`x-client-data` and archive the responses.

6. **A dynamic trace of the BotGuard VM**
   - Hook `botguard.bg`, `eval`, `atob`, `document.createElement`, `iframe.contentWindow` in a real browser and log every call + argument.
   - This is the fastest way to discover which inputs produce slot 3.

7. **Server-side acceptance boundary**
   - Does the server accept a **reused** slot-3 token from an earlier session? (If yes, we can replay.)
   - Does the server accept slot-3 from text chat for an image request? (If yes, slot 3 is not image-specific.)

## How to capture the missing data

### Option A — mitmproxy with full JS body dump (preferred)

```bash
# 1. Start mitmproxy with body saving
mitmproxy --set hardump=/tmp/full.har --set save_stream_file=/tmp/flows.mitm

# 2. On the client open gemini.google.com, authenticate, upload an image, send one prompt.

# 3. Stop mitmproxy and copy /tmp/full.har here.
```

Make sure the client trusts the mitmproxy CA so TLS decryption works.

### Option B — Chrome DevTools + HAR with response bodies

1. Open `chrome://net-export/`, start logging to a file.
2. Reproduce image upload on gemini.google.com.
3. Stop logging, convert to HAR with `https://netlog-viewer.appspot.com/` or `go/chrome-net-export` tools.

### Option C — instrument the browser

In DevTools Console, before uploading an image, paste:

```js
const origBg = window.botguard && window.botguard.bg;
if (origBg) {
  window.botguard.bg = function(...args) {
    console.log('[BOTGUARD BG]', JSON.stringify(args).slice(0, 5000));
    console.trace();
    return origBg.apply(this, args);
  };
}
```

Also log:

```js
console.log('bg keys:', Object.keys(window.botguard));
console.log('storage:', {...localStorage, ...sessionStorage});
```

Then perform an image upload and save the console output.

## Next steps once data arrives

1. Run differential analysis on multiple `(challenge, slot3)` pairs.
2. Try to locate the exact bytecode interpreter in `botguard.js` and trace input → output.
3. Build a minimal JS harness (not a browser, but a Node/QuickJS VM with enough DOM mocks) that executes the VM deterministically.
4. Port the discovered algorithm to Rust.
5. Verify by making `upload_image_works` pass against live cookies.

## Current verdict

**IN PROGRESS**. The token is produced by Google's BotGuard VM. A direct transform from the WAA challenge token is not visible. More instrumented captures are required to decide whether a pure-HTTP/algorithmic generator is feasible.
