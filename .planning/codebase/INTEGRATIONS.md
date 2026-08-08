# External Integrations

**Analysis Date:** 2026-08-08

## APIs & External Services

**Google Gemini / Bard web frontend:**
- Base host: `https://gemini.google.com`.
- Endpoints consumed:
  - `GET /app?hl={language}` — session initialization and signed-in check (`src/client.rs:710`).
  - `POST /_/BardChatUi/data/batchexecute` — model listing (`otAQ7b`), warm-up (`sJBwce`), feature flags (`ESY5D`), and other batchexecute RPCs (`src/client.rs:199`, `581`).
  - `POST /_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate` — chat generation and streaming (`src/client.rs:329`).
- Static API keys embedded in `src/client.rs`:
  - `WAA_API_KEY` (`AIzaSyBGb5fGAyC-pRcRU6MUHb__b_vKha71HRE`) for WAA Create.
  - `OGADS_API_KEY` (`AIzaSyCbsbvGCe7C9mCtdaTycZB2eUFuzsYKG_E`) for ogads GetAsyncData.

**WAA (Web Authenticated Actions) service:**
- Host: `https://waa-pa.clients6.google.com`.
- Endpoint: `POST /$rpc/google.internal.waa.v1.Waa/Create` (`src/client.rs:625`).
- Purpose: obtain WAA token for slot 3 of the StreamGenerate request list.

**OneGoogle AsyncData service:**
- Host: `https://ogads-pa.clients6.google.com`.
- Endpoint: `POST /$rpc/google.internal.onegoogle.asyncdata.v1.AsyncDataService/GetAsyncData` (`src/client.rs:666`).
- Purpose: obtain the `x-goog-ext-525001261-jspb` WAA context header value.

**Google resumable upload (push):**
- Host: `https://push.clients6.google.com/upload/`.
- Used for inline image uploads before chat generation (`src/upload.rs:11`).

**Chrome DevTools Protocol (optional):**
- Local WebSocket connection spawned when the `browser-attestation` feature is enabled.
- Used by `BrowserAttestationClient` in `src/attestation.rs` to capture real browser StreamGenerate payloads.

## Data Storage

**Databases:**
- Not applicable — stateless client SDK.

**File Storage:**
- Local filesystem only:
  - Examples read image files from disk (`examples/image_chat.rs`).
  - `capture_fixtures` example writes captured fixtures to `tests/fixtures/`.
  - Attestation writes a temporary Chrome profile to `/tmp/gemini-sdk-chrome-profile`.

**Caching:**
- In-memory session state cached in `GeminiClient` (`src/session.rs` + `src/client.rs`).
- `reqwest` connection pooling configured with `pool_max_idle_per_host(20)`.

## Authentication & Identity

**Auth Provider:**
- Custom cookie-based authentication using browser cookies copied from a signed-in Google session.
- Required cookies: `__Secure-1PSID` and `__Secure-1PSIDCC` (`src/auth.rs`).
- Optional cookies used for streaming / SAPISIDHASH: `__Secure-1PSIDTS`, `__Secure-1PAPISID`, `SAPISID`, `APISID`, `SOCS`, and legacy `SID`/`HSID`/`SSID`.
- `Credentials::sapisid_hash` builds the `Authorization: SAPISIDHASH <ts>_<sha1>` header for grpc-web endpoints.

## Monitoring & Observability

**Error Tracking:**
- None integrated.

**Logs:**
- `tracing` is used for debug-level internal logging (e.g., consent banner detection, WAA init failures).
- `tracing_subscriber` is only an optional/dev dependency; library consumers must initialize their own subscriber.

## CI/CD & Deployment

**Hosting:**
- crates.io-targeted library (not deployed as a service).

**CI Pipeline:**
- Not detected — no `.github/workflows`, `.gitlab-ci.yml`, or equivalent.

## Environment Configuration

**Required env vars (for examples and live tests):**
- `GEMINI_COOKIES` — raw `Cookie` header string with signed-in Google cookies.
- `GEMINI_PUSH_ID` (optional) — overrides the default push ID for uploads.
- `CHROME_PATH` (when using `browser-attestation`) — path to Chrome/Chromium executable.

**Secrets location:**
- Secrets are supplied at runtime via environment variables only.
- No committed `.env` or credentials files; `.gitignore` ignores `/target`.

## Webhooks & Callbacks

**Incoming:**
- None.

**Outgoing:**
- None — the SDK only initiates requests to Google services.

---

*Integration audit: 2026-08-08*
