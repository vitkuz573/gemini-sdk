# Technology Stack Additions for Browserless WAA Reverse Engineering

**Project:** gemini-sdk
**Milestone:** v0.5 Browserless WAA Reverse
**Researched:** 2026-08-12
**Confidence:** MEDIUM

## Summary

This document recommends the new Rust crates, JS runtimes, and analysis tools needed to reverse-engineer or emulate Google's BotGuard VM for producing `StreamGenerate` slot-3 tokens without launching headless Chrome. All recommendations are scoped to the **new browserless WAA feature only**; the existing core stack (`tokio`, `reqwest`, `serde`, `base64`, etc.) is unchanged.

## Recommended Stack Additions

### Core: Deterministic Token Generation in Rust

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `base64` | 0.23.1 | Decode base64url WAA challenge strings and encode `!`-prefixed slot-3 payloads | Already a dependency; the `base64::engine::general_purpose::URL_SAFE` engine decodes unpadded base64url and is sufficient for both challenge and slot-3 token decoding. No new dependency required. |
| `serde_json` | 1.0.151 | Parse the `Waa/Create` JSON+protobuf envelope and intermediate challenge metadata | Already a dependency. The WAA `Create` response is a JSON array; robust parsing can continue to use `serde_json::Value` to tolerate undocumented shape drift, consistent with the rest of the crate. |
| `bytes` | 1.12.1 | Zero-copy slicing of decoded binary challenge payloads | Already a dependency via `reqwest`. Useful if reverse engineering reveals a structured binary blob inside the WAA challenge that should be parsed without copying. |

### For Algorithmic Port: Required New Crates

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `hex` | 0.4.3 | Hex-encode intermediate hashes, fingerprints, and raw payload dumps during reverse engineering | Lightweight, widely used. Likely needed if the token algorithm involves SHA-1/SHA-256 hex strings or fingerprints derived from the challenge. |
| `sha1` | 0.10.6 | Replicate any HMAC/SHA-1 or raw SHA-1 transforms the BotGuard VM performs client-side | Already a dependency (used for SAPISIDHASH). Reuse before adding new hashing crates; if the algorithm uses HMAC-SHA1, add `hmac` + `sha1` only after confirming the transform. |
| `nom` | 8.0.0 | Parse structured binary WAA challenge blobs into fields, opcodes, or VM bytecode segments | The challenge decodes to ~23 KB of binary data. If it contains length-prefixed fields or a custom bytecode header, `nom` is the standard Rust parser-combinator choice. |

### For VM Emulation Path: Embedded JS Engine

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `rquickjs` | 0.12.2 | Execute the captured `botguard.js` VM inside a lightweight JS engine with mocked DOM/browser globals | QuickJS is small (~210 KB x86 code), fast to start, and designed for embedding. `rquickjs` provides safe high-level Rust bindings, async runtime support, and the ability to inject host objects to mock `document`, `navigator`, `performance`, `localStorage`, `trustedTypes`, and `requestIdleCallback`. |

### Analysis Tools (Development / Reverse Engineering Only)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `insta` | 1.48.0 | Snapshot testing of `(WAA challenge, slot-3 token)` pairs and decoded binary payloads | Standard Rust snapshot library. Lets the team record ground-truth outputs and detect when a new algorithm/harness starts reproducing captured tokens. Should be a dev-dependency only. |
| Node.js + mitmproxy | 20.x / 11.x | Capture more `(challenge, slot-3)` pairs with full JS bodies | Not a crate dependency; used locally per `CAPTURE_FULL_JS.md` to build the training set. The SDK itself does not depend on Node.js at runtime. |

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| JS engine for VM emulation | `rquickjs` | `deno_core` | `deno_core` 0.410.0 is current, but Deno Core merged back into the Deno monorepo in April 2026 and the standalone `deno_core` repo is archived. It also pulls in a large V8 dependency, slower startup, and more complex extension/ops setup. Avoid unless QuickJS fails on a specific JS feature BotGuard requires. |
| JS engine for VM emulation | `rquickjs` | `boa_engine` 0.21.1 | Boa is pure Rust and has a `boa_runtime` crate with basic WebAPIs, but conformance is still experimental (~90% ECMAScript) and the BotGuard VM uses obscure runtime behavior (`eval`, `atob`, iframe events, `trustedTypes`, `requestIdleCallback`). Replicating those mocks is risky; V8/QuickJS behavior is closer to what Google targets. |
| JS engine for VM emulation | `rquickjs` | `wasmtime`/`wasmi` running compiled JS | Adds a full WebAssembly layer without solving the DOM-mock problem. Heavier and no clear path to deterministic token reproduction. |
| Structured protobuf decoding | `serde_json::Value` | `prost` 0.14.4 | `prost` is excellent for known `.proto` schemas, but the WAA `Create` response is an undocumented JSON array. Adding `prost` requires a schema that does not exist and increases build complexity; the existing `Value`-based approach is safer against drift. Revisit only if a stable protobuf definition is discovered. |
| Binary parsing | `nom` | Hand-written byte slicing | Hand-written parsing is fine for one field, but if the challenge blob has multiple length-prefixed sections, `nom` is safer and more maintainable. |

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `deno_core` | Archived standalone repo; heavy V8 footprint; complex embedding API that conflicts with the SDK's goal of remaining lightweight. | `rquickjs` for VM emulation, or a pure Rust port once the algorithm is understood. |
| `boa_engine` as primary VM | Experimental conformance; likely missing the obscure browser globals and timing APIs BotGuard relies on. | `rquickjs`, or port to Rust after tracing. |
| `tokio-tungstenite` for browserless WAA | Already used by the existing `browser-attestation` feature for Chrome CDP. The new browserless path must not require a browser or WebSocket. | Keep CDP path as an optional fallback behind the existing feature; do not add new CDP code. |
| Headless Chrome / Chromium runtime | The entire goal of the milestone is to avoid this. The existing `browser-attestation` feature already covers users who accept the overhead. | `rquickjs` harness or Rust port. |
| `prost` / `pbjson` | No known stable protobuf schema for the BotGuard challenge or slot-3 token. Premature. | `serde_json::Value` and manual byte parsing. |
| `reqwest`-level changes | The network layer already supports `waa-pa.clients6.google.com` and `ogads-pa.clients6.google.com` via `run_waa_init_chain`. | Reuse existing RPC helpers; no new HTTP dependencies. |

## Integration with Existing Crate

The browserless WAA generator should live in a new source module (e.g., `src/waa/`) and be exposed through an **optional Cargo feature**, similar to how `browser-attestation` is gated today.

### Proposed Cargo additions

```toml
[dependencies]
# Existing dependencies already cover base64/serde_json/bytes/sha1/thiserror.
# Add only if the VM-emulation path is selected:
rquickjs = { version = "0.12.2", features = ["futures", "macro"], optional = true }
hex = { version = "0.4.3", optional = true }
nom = { version = "8.0.0", optional = true }

[dev-dependencies]
insta = { version = "1.48.0", features = ["json"] }

[features]
browserless-waa = ["dep:rquickjs", "dep:hex", "dep:nom"]
```

Keep `rquickjs` and `hex`/`nom` optional so the default SDK remains lightweight.

### Integration points

1. **`src/attestation.rs`** — Keep the existing `BrowserAttestationClient` (CDP-based) unchanged. The new browserless generator is a parallel implementation, not a replacement.
2. **`GeminiClient::run_waa_init_chain` in `src/client.rs`** — After `waa_create()` returns the challenge token, call the new `BrowserlessWaa::generate_token(challenge, session)` to produce slot-3. If generation succeeds, store it in `SessionState::waa_token` and include it in `build_inner_req_list` slot 3.
3. **`src/proto/slots.rs`** — Slot 3 already accepts `waa_token: Option<&str>`. The browserless path just supplies a non-`None` token; no slot builder changes are required.
4. **Fallback behavior** — If browserless generation fails, the warm-up chain should fall back to the synthetic/no-attestation path (current behavior) or, if `browser-attestation` is enabled, to the CDP path. This preserves the "WAA failures are non-fatal" design decision.

## Phase-Scoped Stack Recommendations

| Phase | Primary stack | Notes |
|-------|---------------|-------|
| Spike / analysis | `base64`, `serde_json`, `hex`, `insta` | Decode pairs, snapshot payloads, look for deterministic patterns. |
| VM harness | `rquickjs` + custom DOM mocks | Only if algorithmic port fails; provides the fastest path to a real token without a browser. |
| Ported algorithm | `base64`, `bytes`, `nom`, `sha1` | If the transform is identified and can be expressed in pure Rust. |
| Production feature | Optional `browserless-waa` feature gating the above | Default-off to keep SDK light. |

## Heavy / Browser-like Dependencies — Flagged

The following are explicitly flagged as heavy and should remain optional or avoided:

- **`rquickjs`** — Medium weight (~210 KB code, small C library). Acceptable as an optional feature, but not in default dependencies.
- **`deno_core`** — Heavy (V8), archived standalone repo; avoid.
- **`boa_engine`** — Pure Rust but large compile-time footprint and incomplete WebAPI conformance; avoid as primary VM.
- **Chrome / Chromium / CDP** — Already gated by `browser-attestation`; do not expand.

## Version Compatibility

| Package | MSRV / Compatibility | Notes |
|---------|---------------------|-------|
| `rquickjs` 0.12.2 | Rust 1.70+ per README | Compatible with project MSRV 1.80. |
| `hex` 0.4.3 | Rust 1.31+ | Compatible. |
| `nom` 8.0.0 | Rust 1.56+ | Compatible. |
| `insta` 1.48.0 | Rust 1.60+ (dev only) | Compatible. |
| `base64` 0.23.1 | Rust 1.57+ | Already used; latest as of 2026-08-04. |
| `serde_json` 1.0.151 | Rust 1.56+ | Already used; latest as of 2026-07-20. |

## Sources

- `docs.rs` pages for `base64` 0.23.1, `serde_json` 1.0.151, `bytes` 1.12.1, `hex` 0.4.3, `nom` 8.0.0, `sha1`, `thiserror` 2.0.20, `insta` 1.48.0 — verified current versions and MSRV compatibility (confidence LOW for web-fetched pages; version numbers are authoritative from docs.rs metadata).
- `docs.rs`/`crates.io` for `rquickjs` 0.12.2, `deno_core` 0.410.0, `boa_engine` 0.21.1, `boa_runtime` 0.21.1, `wasmtime` 47.0.3, `wasmi` 1.1.0 — comparison basis for JS engine selection (confidence MEDIUM for docs.rs; deno_core repo archival confirmed via GitHub fetch, confidence LOW).
- Project source files: `src/attestation.rs`, `src/client.rs` (lines 2306–2394), `src/session.rs`, `src/proto/mod.rs`, `src/proto/slots.rs`, `Cargo.toml` — integration points and existing dependencies.
- Spike findings: `.planning/spikes/004-waa-token/README.md`, `CAPTURE_GUIDE.md`, `CAPTURE_FULL_JS.md`, `.opencode/skills/spike-findings-gemini-sdk/references/waa-attestation.md` — WAA protocol shape, missing data, and current CDP fallback design.
