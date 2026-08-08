# Technology Stack

**Analysis Date:** 2026-08-08

## Languages

**Primary:**
- Rust (edition 2021) — the entire SDK, tests, examples, and benchmarks.
- Minimum supported Rust version (MSRV): 1.80, declared in `Cargo.toml` at `rust-version = "1.80"`.

**Secondary:**
- Markdown — crate-level and module-level documentation (`README.md`, `docs/protocol.md`).
- TOML — package manifest and formatting/linting configuration (`Cargo.toml`, `rustfmt.toml`, `clippy.toml`).

## Runtime

**Environment:**
- Tokio 1.40 with the `full` feature flag, providing the asynchronous runtime for all network and I/O operations.

**Package Manager:**
- Cargo (bundled with Rust 1.95 toolchain in this environment).
- Lockfile: `Cargo.lock` is committed.

## Frameworks

**Core:**
- `reqwest` 0.12 — HTTP client for all Gemini web frontend, WAA, ogads, and upload calls.
- `serde` / `serde_json` 1.0 — serialization and deserialization of WIZ protocol payloads and API responses.
- `tokio` 1.40 — async runtime and synchronization primitives (`tokio::sync::Mutex`, `tokio::process`).

**Testing:**
- Built-in `#[test]` and `#[tokio::test]` via `tokio-test` 0.4.
- `wiremock` 0.6 is listed in dev-dependencies but not used in the current test suite.
- `criterion` 0.5 with `async_tokio` for benchmarks.

**Build/Dev:**
- `cargo fmt` driven by `rustfmt.toml`.
- `cargo clippy --all-targets -- -D warnings` driven by `clippy.toml`.
- `cargo doc --no-deps` for documentation.

## Key Dependencies

**Critical:**
- `reqwest` 0.12 (features `cookies`, `json`, `multipart`, `stream`) — all outbound HTTP.
- `tokio` 1.40 (`full`) — runtime, sync, process spawning for attestation.
- `serde` / `serde_json` — WIZ protocol JSON parsing and request body construction.
- `thiserror` 1.0 — derive-based error enum (`src/errors.rs`).
- `tracing` / `tracing-subscriber` 0.3 — structured logging.

**Infrastructure:**
- `uuid` 1.11 (`v4`) — request UUID generation (`src/proto/mod.rs`).
- `base64` 0.22 — inline image encoding and WAA token encoding.
- `urlencoding` 2.1 — URL-encoded `f.req` bodies.
- `rand` 0.8 — nonce generation.
- `sha1` 0.10 — SAPISIDHASH authorization (`src/auth.rs`).
- `backoff` 0.4 (`tokio`) — exponential backoff retry wrapper (`src/retry.rs`).
- `regex` 1.11 (optional, feature `capture-fixtures`) — fixture redaction.
- `tokio-tungstenite` 0.24 (optional, feature `browser-attestation`) — Chrome DevTools Protocol WebSocket.
- `serde_urlencoded` 0.7 (optional, feature `browser-attestation`) — not currently referenced in source.
- `futures` 0.3 / `async-stream` 0.3 — stream helpers used in attestation and examples.

## Configuration

**Environment:**
- Cookie string supplied by caller via `GEMINI_COOKIES` env var in examples/tests.
- Optional `GEMINI_PUSH_ID` overrides the default push ID in `src/session.rs`.
- Live-cookie tests read `/tmp/opencode/gemini_cookies.env` via `dotenvy` (`tests/real_cookies.rs`).
- `.env` files are present in the repo only for the temporary test path; no committed secrets.

**Build:**
- `Cargo.toml` defines:
  - `default = []`
  - `browser-attestation = ["dep:tokio-tungstenite", "dep:serde_urlencoded", "dep:tracing-subscriber"]`
  - `capture-fixtures = ["dep:regex"]`
- `rustfmt.toml` enforces 100-column width, Unix newlines, import/module reordering, and field-init shorthand.
- `clippy.toml` sets `avoid-breaking-exported-api = false`.
- Release profile enables `lto = true` and `codegen-units = 1`.

## Platform Requirements

**Development:**
- Rust 1.80+.
- Linux/macOS/Windows with a C linker (`gcc` / `lld`).
- For `browser-attestation` feature: a Chrome/Chromium executable accessible via `CHROME_PATH`.

**Production:**
- No server deployment; this is a client library crate published (intended) to crates.io.
- Consumers embed it in async Tokio applications.

---

*Stack analysis: 2026-08-08*
