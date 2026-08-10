# Phase 12 Research: Transient 400 Patterns, HAR Format, and Retry Policy

## Transient 400 Pattern

### Observed Failure

During live testing the Google backend occasionally returns HTTP 400 with a body that contains WIZ error frames instead of a normal batchexecute response. The frames include the keys/fields:

- `er` — the top-level error frame.
- `di` — an auxiliary diagnostic frame.
- `af.httprm` — an HTTP parameter frame describing the failed request.

These fields appear inside the line-delimited JSON returned by `/_/BardChatUi/data/batchexecute`. The same request, when retried a few seconds later with identical parameters and cookies, often succeeds. This strongly suggests a transient backend-side rejection (load shedding, token validation race, or request-routing inconsistency) rather than an actual malformed request or expired cookie.

### Distinguishing Transient 400 from Permanent 400

A permanent 400 from Google is typically one of:

- Plain HTML redirect to a sign-in page (cookie rejection).
- A short JSON object with `{"error": "..."}` and no WIZ frame wrapper.
- A missing/unknown `rpcids` response.

The transient pattern is identified by ALL of the following:

1. HTTP status is 400.
2. Response body contains at least one WIZ line whose first string field is `"er"`.
3. The same response body also contains `"di"` and `"af.httprm"` string fields.
4. The `batchexecute` outer wrapper (`wrb.fr`) is absent or contains no matching RPC entry.

When all four conditions hold, the request should be retried with exponential backoff. If after the configured retry budget the pattern still occurs, the SDK must return `Error::Transient("Google rejected batchexecute with WIZ error frames after N retries")` so callers can still distinguish it from a permanent 400.

### Cookie Rejection → NotSignedIn

When `/app?hl={lang}` returns a 400, or returns HTML that does not contain the signed-in markers (`S06Grb` numeric Gaia id + `oPEP7c` email), the SDK currently returns a generic `Error::Api 400` or `Error::Parse`. The desired behavior is to inspect the `/app` response defensively and, if signed-in markers are absent, return `Error::NotSignedIn("cookies rejected by Gemini /app")`. This applies to both:

- `init_session` before any batchexecute call, and
- Any later batchexecute call whose response HTML indicates a sign-in redirect.

### Sources

- Spike findings skill: `references/auth.md` (signed-in detection).
- Live HAR captures from Phase 11 integration testing (not committed; cookies present).
- Internal test runs of `tests/real_cookies.rs` against `gemini.google.com` on 2026-08-10.

## HAR Capture Format

The SDK will support optional capture of every HTTP request/response into a W3C-style HAR file. Design choices:

### File Structure

```json
{
  "log": {
    "version": "1.2",
    "creator": {
      "name": "gemini-sdk",
      "version": "0.1.0"
    },
    "entries": [
      {
        "startedDateTime": "2026-08-10T10:00:00.000Z",
        "time": 123,
        "request": {
          "method": "POST",
          "url": "https://gemini.google.com/_/BardChatUi/data/batchexecute",
          "httpVersion": "HTTP/2.0",
          "headers": [...],
          "cookies": [...],
          "queryString": [...],
          "postData": {
            "mimeType": "application/x-www-form-urlencoded;charset=UTF-8",
            "text": "f.req=..."
          },
          "headersSize": -1,
          "bodySize": -1
        },
        "response": {
          "status": 400,
          "statusText": "",
          "httpVersion": "HTTP/2.0",
          "headers": [...],
          "cookies": [...],
          "content": {
            "size": 1234,
            "mimeType": "text/plain",
            "text": "..."
          },
          "redirectURL": "",
          "headersSize": -1,
          "bodySize": 1234
        },
        "cache": {},
        "timings": {
          "send": 0,
          "wait": 123,
          "receive": 0
        }
      }
    ]
  }
}
```

### Redaction Rules

Before writing to the HAR file, the following fields must be sanitized:

- Cookie names are preserved; values are replaced with `<redacted>`.
- `Authorization` header values are replaced with `<redacted>`.
- `x-goog-ext-*` headers containing session tokens are preserved in shape but their content replaced with `<redacted>`.
- POST body text is scanned for cookie-like substrings and any matches are replaced with `<redacted>`.

Redaction must not modify the in-memory request actually sent to Google; only the HAR snapshot is sanitized.

### Activation

HAR capture is opt-in via a new builder method:

```rust
let client = GeminiClient::from_cookie_header(&cookies)?
    .with_har_capture("/tmp/gemini_probe.har")
    .await;
```

If the path cannot be opened for writing, client construction returns `Error::Config`. The HAR writer is held behind a mutex and flushed after every entry to limit data loss on panic/crash.

### Dependencies

No new crates required. HAR uses `serde_json::Value` for the document and writes atomically with `tokio::fs`.

## Retry Policy

### Existing Policy

`src/retry.rs` uses `backoff` crate with:

- initial interval: 500 ms
- max interval: 8 s
- max elapsed time: 30 s

`Error::is_transient` currently treats server errors (5xx) and 429 as transient; all other 4xx are permanent.

### Required Change

1. Introduce a new internal classification step after a batchexecute HTTP 400 is received: if the body matches the WIZ transient pattern, convert the error to `Error::Transient` before passing it to the retry loop.
2. Keep the existing `send_with_retry` wrapper but allow it to receive an optional response-body inspector closure that reclassifies 400s.
3. Update `Error::is_transient` so `Error::Transient` is retried (already true) and so a 400 carrying the WIZ pattern is retried at the retry layer.

Because `send_with_retry` operates on `reqwest::Response`, the cleanest approach is:

- Read the response body text eagerly inside the operation closure.
- If status is 400 and body matches transient WIZ pattern, return a synthetic `reqwest::Error` or a new internal error marker that the retry loop treats as transient.
- On success, reconstruct a `reqwest::Response` from status/headers/body for downstream parsing.

To avoid breaking the existing response flow, add a thin internal helper `send_batchexecute_with_retry` that:

1. Calls `send_with_retry`.
2. Inside the closure, sends the request, checks status, reads body.
3. If transient WIZ 400, returns a transient error.
4. Otherwise returns an `Ok(ResponseWithBody)` struct holding status, headers, and body text.

All batchexecute methods (`conversation_action`, `get_user_info`, `get_last_selected_mode`, `set_last_selected_mode`, `get_locale_tools`, `get_model_config`, `get_locale_config`, `get_tools_config`, `get_usage_stats`, `get_scheduled_prompts`, `list_models`) should route through this helper.

`StreamGenerate` retry behavior is out of scope for this phase; it continues to use the existing `send_with_retry` path.

## Live Probe Design

### Binary: `examples/live_probe.rs`

A standalone example that:

1. Reads `GEMINI_COOKIES` and optional `GEMINI_BASE_URL`, `GEMINI_HAR_PATH`, and `GEMINI_REPORT_PATH` env vars.
2. Builds a `GeminiClient` with HAR capture enabled if `GEMINI_HAR_PATH` is set.
3. Exercises the following calls in order (each is independent except conversation actions which need a created turn):
   - `verify_signed_in`
   - `list_models`
   - base chat `send_message`
   - streaming `generate_stream` (consume first chunk)
   - `get_user_info`
   - `get_last_selected_mode`
   - `set_last_selected_mode` (set to the current value or a safe known mode id, then read back)
   - `get_locale_tools`
   - `get_model_config`
   - `get_locale_config`
   - `get_tools_config`
   - `get_usage_stats`
   - `get_scheduled_prompts`
   - create a turn, then `regenerate_turn`, `rate_turn(Good)`, `delete_turn` on it
4. Records per-call telemetry: operation, duration_ms, success bool, error string (empty on success), retry_count, http_status, transient_400_detected bool.
5. Writes a JSON report to `GEMINI_REPORT_PATH` (default `/tmp/gemini_live_probe_report.json`).
6. Exits with code 0 only if all non-optional calls succeed; exits with code 1 and prints a summary of failures.

The probe intentionally mutates a throwaway conversation so it does not interfere with user history.

### Report Schema

```json
{
  "sdk_version": "0.1.0",
  "started_at": "2026-08-10T10:00:00Z",
  "finished_at": "2026-08-10T10:01:00Z",
  "base_url": "https://gemini.google.com",
  "summary": {
    "total": 14,
    "passed": 13,
    "failed": 1
  },
  "calls": [
    {
      "operation": "list_models",
      "duration_ms": 1234,
      "success": true,
      "error": "",
      "retry_count": 0,
      "http_status": 200,
      "transient_400_detected": false
    }
  ]
}
```

## Test Expansion

`tests/real_cookies.rs` should gain tests for each v0.2 API:

- `get_user_info_works`
- `get_last_selected_mode_works`
- `set_last_selected_mode_round_trips`
- `get_locale_tools_works`
- `get_model_config_works`
- `get_locale_config_works`
- `get_tools_config_works`
- `get_usage_stats_works`
- `get_scheduled_prompts_works`
- `conversation_actions_works` (create a turn, regenerate, rate, delete)

Each test must skip gracefully when `GEMINI_COOKIES` is missing.

## Risks

- The WIZ transient 400 pattern may evolve; detection must be conservative and fall back to the current generic behavior when uncertain.
- HAR files can grow large; the probe writes one file per run, and users are responsible for cleanup.
- Live tests depend on cookies and Google backend state; they remain `#[ignore]` or env-gated and cannot run in CI.

## Open Questions

None. This phase is fully specified.
