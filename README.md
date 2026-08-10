# Gemini SDK

A clean, well-structured, production-ready Rust SDK for interacting with the
Google Gemini / Bard web frontend (`gemini.google.com`).

> **Note:** This SDK targets the undocumented web frontend protocol. Google may
> change it without notice; this library is intended for advanced use cases and
> reverse-engineering-friendly integrations.

## Features

- Cookie-based authentication using browser cookies.
- Pluggable async `CredentialsProvider` for env/file/keyring auth sources.
- Text-only and image/audio/video (inline data / URL) chat completions.
- Streaming and non-streaming response handling.
- Model reasoning / thinking content extraction (`ChatResponse::thinking()`).
- Multi-turn conversation state.
- Model listing via `batchexecute` (`GetUserStatus` / `Fd0Qje`).
- File upload to `push.clients6.google.com` with progress events
  (`UploadEvent`).
- Optional browser attestation using headless Chrome CDP (`browser-attestation`
  feature).
- Consent / `SOCS` cookie auto-acquisition.
- Request/response `HttpHook` for custom observability.
- `tracing` spans on public operations (secrets are never logged).
- Injectable `reqwest::Client` for custom timeouts, middleware, or connection
  pooling.
- Function calling / tools via the `Tool` trait and `generate_with_tools`.
- Feature-gated metrics facade (`metrics` feature) with OpenTelemetry support.
- Session save/restore (`Snapshot`) and conversation save/restore.
- Proper error types, retry logic with exponential backoff, and rate-limit
  handling.
- Comprehensive unit and integration tests.

## Requirements

- Rust 1.80 or newer.
- `tokio` runtime.
- Valid signed-in Google cookies. The SDK requires `__Secure-1PSID` and
  `__Secure-1PSIDCC` to build a client, but live calls to `/app` and the
  backend RPCs need the full browser cookie set:
  `SID`, `HSID`, `SSID`, `APISID`, `SAPISID`, `SIDCC`, `__Secure-ENID`, `NID`,
  `__Secure-1PSIDTS`, `__Secure-1PAPISID` (or `__Secure-3PAPISID`), and `SOCS`.

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
gemini-sdk = "0.1"
```

Enable browser attestation if you need image uploads or true multi-turn state:

```toml
[dependencies]
gemini-sdk = { version = "0.1", features = ["browser-attestation"] }
```

## Quick start

```rust
use gemini_sdk::{GeminiClient, ModelCategory};

#[tokio::main]
async fn main() -> gemini_sdk::Result<()> {
    let cookies = "__Secure-1PSID=...; __Secure-1PSIDCC=...";
    let client = GeminiClient::from_cookie_header(cookies)?;

    let response = client
        .chat()
        .with_category(ModelCategory::Auto)
        .send_message("What is Rust?")
        .await?;

    println!("{}", response.text());

    Ok(())
}
```

## Examples

Run examples with live cookies:

```bash
# Copy the full Cookie header from a signed-in browser request to gemini.google.com.
export GEMINI_COOKIES="__Secure-1PSID=...; __Secure-1PSIDCC=...; SID=...; HSID=...; SSID=...; APISID=...; SAPISID=...; SIDCC=...; __Secure-ENID=...; NID=...; __Secure-1PSIDTS=...; __Secure-1PAPISID=...; SOCS=..."

cargo run --example text_chat -- "What is Rust?"
cargo run --example image_chat -- /path/to/image.png "Describe this image."
cargo run --example stream_chat -- "Tell me a story"
```

## Protocol documentation

For a detailed description of the undocumented Gemini web frontend endpoints,
WIZ slot layout, response frames, and attestation flow, see
[`docs/protocol.md`](docs/protocol.md).

## Architecture

- `auth` — Cookie parsing and header formatting.
- `client` — Main `GeminiClient` and high-level builders.
- `chat` — Chat messages, content parts, conversations, response types.
- `models` — Model discovery metadata and categories.
- `proto` — WIZ protocol helpers: slot builder and response parser.
- `upload` — Resumable upload to `push.clients6.google.com`.
- `errors` — Strongly-typed error enum with transient detection.
- `attestation` *(feature)* — Headless Chrome CDP payload capture.

## Semver policy

This crate follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

- **Before v1.0 (0.x.y):** minor version bumps (`0.1.0` → `0.2.0`) may include
  breaking changes. Patch version bumps (`0.1.0` → `0.1.1`) are reserved for
  backwards-compatible bug fixes, documentation improvements, and internal
  refactorings that do not change the public API.
- **After v1.0:** breaking changes are only introduced in major version bumps
  (`1.x.y` → `2.0.0`). Minor versions add functionality in a backwards-compatible
  manner, and patch versions contain only bug fixes.

Public types that are expected to grow over time are marked with
`#[non_exhaustive]` and their fields are kept private; use the provided
constructors and accessor methods to remain compatible with future releases.

See [`docs/migration-v0-to-v1.md`](docs/migration-v0-to-v1.md) for the current
breaking changes on the path to v1.0.

## Development

```bash
cargo fmt --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps --all-features
cargo publish --dry-run --all-features
```

Integration tests that require live cookies are marked with `#[ignore]`.

### MSRV policy

The Minimum Supported Rust Version is **1.80**, declared in `Cargo.toml` as
`rust-version`. MSRV is only raised in minor 0.x or major releases, never in
patch releases.

## License

This project is licensed under the [MIT License](LICENSE).

## Author

Vitaly Kuzyaev <vitkuz573@gmail.com>
