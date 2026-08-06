# Gemini SDK

A clean, well-structured, production-ready Rust SDK for interacting with the
Google Gemini / Bard web frontend (`gemini.google.com`).

> **Note:** This SDK targets the undocumented web frontend protocol. Google may
> change it without notice; this library is intended for advanced use cases and
> reverse-engineering-friendly integrations.

## Features

- Cookie-based authentication using browser cookies.
- Text-only and image (inline data) chat completions.
- Streaming and non-streaming response handling.
- Multi-turn conversation state.
- Model listing via `batchexecute` (`GetUserStatus` / `Fd0Qje`).
- File upload to `push.clients6.google.com`.
- Optional browser attestation using headless Chrome CDP (`browser-attestation`
  feature).
- Consent / `SOCS` cookie auto-acquisition.
- Proper error types, retry logic with exponential backoff, and rate-limit
  handling.
- Comprehensive unit and integration tests.

## Requirements

- Rust 1.80 or newer.
- `tokio` runtime.
- Valid signed-in Google cookies (`__Secure-1PSID` and `__Secure-1PSIDCC`).

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
    let client = GeminiClient::from_cookie_header(cookies)
        .await?
        .with_category(ModelCategory::Auto);

    let response = client.chat().send_message("What is Rust?").await?;
    println!("{}", response.text());

    Ok(())
}
```

## Examples

Run examples with live cookies:

```bash
export GEMINI_COOKIES="__Secure-1PSID=...; __Secure-1PSIDCC=..."

cargo run --example text_chat -- "What is Rust?"
cargo run --example image_chat -- /path/to/image.png "Describe this image."
cargo run --example stream_chat -- "Tell me a story"
```

## Architecture

- `auth` — Cookie parsing and header formatting.
- `client` — Main `GeminiClient` and high-level builders.
- `chat` — Chat messages, content parts, conversations, response types.
- `models` — Model discovery metadata and categories.
- `proto` — WIZ protocol helpers: slot builder and response parser.
- `upload` — Resumable upload to `push.clients6.google.com`.
- `errors` — Strongly-typed error enum with transient detection.
- `attestation` *(feature)* — Headless Chrome CDP payload capture.

## Development

```bash
cargo check
cargo test
cargo clippy --all-targets
cargo doc --no-deps
```

Integration tests that require live cookies are marked with `#[ignore]`.

## License

This project is licensed under the [MIT License](LICENSE).

## Author

Vitaly Kuzyaev <vitkuz573@gmail.com>
