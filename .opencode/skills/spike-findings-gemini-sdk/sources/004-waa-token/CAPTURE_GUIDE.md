# Capture guide for WAA / BotGuard spikes

## Goal

Collect a single HAR/mitm capture that contains **full bodies for every JS, JSON, and HTML request** between `Waa/Create` and the next `StreamGenerate` with an image attachment. This is required to reverse the BotGuard challenge without a browser.

## Prerequisites

- mitmproxy installed (`pip install mitmproxy` or system package).
- Client machine trusts mitmproxy CA.
- Valid Google account with Gemini access.

## Step 1 — install CA on client

```bash
# On the machine running mitmproxy
mitmproxy --version
cat ~/.mitmproxy/mitmproxy-ca-cert.pem
```

Install that cert on the client (browser / OS trust store).

## Step 2 — start mitmproxy with dumpers

```bash
cd /home/vitaly/projects/gemini-sdk/.planning/spikes/004-waa-token
mitmproxy \
  -s capture_js_dump.py \
  --set hardump=/tmp/gemini_full_$(date +%Y%m%d_%H%M%S).har \
  --set save_stream_file=/tmp/gemini_full_$(date +%Y%m%d_%H%M%S).mitm
```

This saves:
- `/tmp/gemini_full_*.har` — full HAR with bodies.
- `/tmp/gemini_full_*.mitm` — raw mitm flows.
- `js_dump/` — every JS/HTML/JSON body on disk, named by URL.

## Step 3 — reproduce image upload

On the client browser configured to use mitmproxy:

1. Open `https://gemini.google.com/app`.
2. Authenticate if needed.
3. Start DevTools → Console.
4. Paste the instrumentation hook (see below) and press Enter.
5. Upload any image and send a prompt, e.g. "Describe this image".
6. Wait for the answer.
7. Save DevTools console output to a file (`console.log`).

## Step 4 — DevTools instrumentation hook

Before uploading, run in Console:

```js
(function() {
  function log(label, data) {
    const s = JSON.stringify(data);
    console.log(label + ' ' + (s.length > 4000 ? s.slice(0, 4000) + '... (' + s.length + ' chars)' : s));
  }

  // Hook botguard.bg
  const check = () => {
    if (window.botguard && window.botguard.bg && !window.botguard._hooked) {
      const orig = window.botguard.bg;
      window.botguard.bg = function(...args) {
        log('[BOTGUARD_BG_ARGS]', args);
        console.trace('[BOTGUARD_BG_TRACE]');
        const cb = typeof args[args.length - 1] === 'function' ? args[args.length - 1] : null;
        if (cb) {
          args[args.length - 1] = function(result) {
            log('[BOTGUARD_BG_RESULT]', result);
            return cb.apply(this, arguments);
          };
        }
        return orig.apply(this, args);
      };
      window.botguard._hooked = true;
      log('[BOTGUARD_HOOKED]', Object.keys(window.botguard));
    }
    if (window.bg && !window.bg._hooked) {
      const orig = window.bg;
      window.bg = function(...args) {
        log('[WINDOW_BG_ARGS]', args);
        console.trace('[WINDOW_BG_TRACE]');
        return orig.apply(this, args);
      };
      window.bg._hooked = true;
    }
  };
  setInterval(check, 500);

  log('[BOTGUARD_KEYS]', window.botguard ? Object.keys(window.botguard) : null);
  log('[STORAGE]', {...localStorage, ...sessionStorage});
})();
```

Save the resulting console output as `console.log` in the spike directory.

## Step 5 — copy artifacts

```bash
DATE=$(date +%Y%m%d_%H%M%S)
mkdir -p /home/vitaly/projects/gemini-sdk/.planning/spikes/005-waa-token-$DATE
cp /tmp/gemini_full_*.har /home/vitaly/projects/gemini-sdk/.planning/spikes/005-waa-token-$DATE/
cp /tmp/gemini_full_*.mitm /home/vitaly/projects/gemini-sdk/.planning/spikes/005-waa-token-$DATE/
cp -r /home/vitaly/projects/gemini-sdk/.planning/spikes/004-waa-token/js_dump /home/vitaly/projects/gemini-sdk/.planning/spikes/005-waa-token-$DATE/
cp console.log /home/vitaly/projects/gemini-sdk/.planning/spikes/005-waa-token-$DATE/ 2>/dev/null || true
```

## What the next spike will do

With these artifacts we can:

1. Inspect the full Bard bundle that calls `botguard.bg`.
2. See exact arguments passed to the VM.
3. Compare multiple `(Waa challenge, slot 3)` pairs.
4. Attempt to build a minimal DOM VM harness in Node.js/QuickJS.
5. Port the token algorithm to Rust.

## Notes

- Do NOT record while logged into sensitive accounts if you are not comfortable with proxy TLS interception.
- The capture may contain cookies; keep it local / private.
