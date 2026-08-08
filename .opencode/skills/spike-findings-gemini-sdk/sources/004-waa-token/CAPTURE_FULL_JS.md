# Full-JS capture instructions for Gemini / BotGuard reverse engineering

## Why the previous HAR missed JS bodies

The previous capture recorded Gemini gstatic JS bundles only as URLs with empty bodies. Likely causes:

1. QUIC/HTTP3 bypassed the proxy (`--ignore-certificate-errors` does not force HTTP/2).
2. Chrome cached the JS from a previous non-proxied session.
3. `mitmproxy` hardump dropped 304/cached responses.

## Required fixes

- Disable QUIC explicitly.
- Force HTTP/1.1 or HTTP/2 (not QUIC).
- Use a fresh Chrome profile so no JS is cached.
- Install the mitmproxy CA into Chrome's trust store instead of `--ignore-certificate-errors`.
- Use the addon that dumps every response body to disk (`capture_js_dump.py`).

## One-time: install mitmproxy CA

```bash
# 1. Start mitmproxy once to generate certificates.
mitmdump --set hardump=/tmp/warmup.har
# Stop with Ctrl-C.

# 2. Chrome on Linux reads system NSS DB. Install cert:
certutil -d sql:$HOME/.pki/nssdb -A -t "C,," -n mitmproxy \
  -i ~/.mitmproxy/mitmproxy-ca-cert.pem

# Verify:
certutil -d sql:$HOME/.pki/nssdb -L | grep mitmproxy
```

If `certutil` is missing: `sudo apt install libnss3-tools`.

## Start the proxy

```bash
cd /home/vitaly/projects/gemini-sdk/.planning/spikes/004-waa-token
mitmdump \
  -s capture_js_dump.py \
  --set hardump=/tmp/gemini_full_$(date +%Y%m%d_%H%M%S).har \
  --mode regular@8082
```

## Start Chromium with a clean profile

```bash
# Kill old chromium instances first
pkill -f chromium || true

# Remove any previous mitm profile to avoid cached JS
rm -rf /tmp/chromium-mitm-fresh

chromium \
  --user-data-dir=/tmp/chromium-mitm-fresh \
  --proxy-server="http://127.0.0.1:8082" \
  --disable-quic \
  --enable-features="UseOzonePlatform" \
  --no-first-run \
  --no-default-browser-check \
  --disable-background-networking \
  --disable-component-update \
  --disk-cache-dir=/tmp/chromium-mitm-cache \
  --disk-cache-size=1 \
  --media-cache-size=1 \
  --enable-logging=stderr --v=1 \
  https://gemini.google.com/app
```

## Reproduce image upload

1. Sign in to Gemini (if not already signed in).
2. Open DevTools → Console.
3. Paste the hook from `CAPTURE_GUIDE.md` section "DevTools instrumentation hook".
4. Upload any image and send: `Describe this image in one sentence.`
5. Wait for the answer.
6. Save the console log: right-click in Console → `Save as...` → `console.log`.

## Stop and collect artifacts

```bash
# In the mitmdump terminal press Ctrl-C.

DATE=$(date +%Y%m%d_%H%M%S)
OUT=/home/vitaly/projects/gemini-sdk/.planning/spikes/005-waa-token-$DATE
mkdir -p $OUT
cp /tmp/gemini_full_*.har $OUT/
cp -r js_dump $OUT/
cp console.log $OUT/ 2>/dev/null || true
cp /home/vitaly/projects/gemini-sdk/.planning/spikes/004-waa-token/data/gemini_cookies.env $OUT/cookies.env 2>/dev/null || true

ls -lh $OUT
```

## Verify JS bodies were captured

```bash
python3 - <<'PY'
import json, sys
har = json.load(open(sys.argv[1], 'rb'), strict=False)
js_entries = [e for e in har['log']['entries'] if '.js' in e['request']['url']]
with_body = sum(1 for e in js_entries if e['response']['content'].get('text'))
print(f'JS entries: {len(js_entries)}, with body: {with_body}')
PY $OUT/*.har
```

Expected: dozens of JS entries with bodies, including `boq-bard-web` bundles.

## Troubleshooting

### Still no JS bodies

- Check that mitmproxy CA is trusted: open `https://mitm.it` in Chromium and verify green lock.
- Check for QUIC bypass: in Chrome DevTools Network tab, protocol column should show `h2` or `http/1.1`, not `h3`.
- Try forcing HTTP/1.1:
  ```bash
  chromium --disable-http2 --disable-quic ...
  ```

### `certutil` install fails

Use the manual Chrome certificate import:
- Settings → Privacy and security → Security → Manage certificates.
- Authorities → Import → `~/.mitmproxy/mitmproxy-ca-cert.pem`.
- Trust for identifying websites.

## What changes for spike 004

This directory now contains:
- `CAPTURE_FULL_JS.md` (this file).
- `capture_js_dump.py` addon.
- `data/mitm.har` — original capture.
- `data/botguard.js` — BotGuard VM.

Once a full-JS capture arrives, spike 005 should:
1. Locate the Bard bundle that calls `window.botguard.bg(...)`.
2. Extract exact arguments and the callback closure.
3. Compare multiple `(challenge, slot3)` pairs.
4. Build a headless VM harness or port the algorithm to Rust.
