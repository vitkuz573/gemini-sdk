# Browserless WAA Token Generation

## Purpose

This document describes the enterprise-grade, pure-Rust WAA (Web Abuse and
Attribution) token generator used to build the `StreamGenerate` slot-3 payload
without running a browser.

## Background

When the Gemini frontend sends a `StreamGenerate` request, the 97-slot `f.req`
inner array contains a WAA attestation token in **slot 3**. The token is not a
plain BotGuard response; it is a wrapper around a short raw token returned by
the BotGuard VM `h3d` callback.

Reverse-engineering showed that the wrapper has a stable layout, but only part
of it can be generated deterministically from the request data. The remaining
bytes depend on the live BotGuard VM and browser environment.

## Slot-3 wrapper layout

Decoded payload offsets:

| Offset | Length | Content |
|--------|--------|---------|
| 0 | 5 | Request-specific header (environment-derived). |
| 5 | 84 | `raw_token[5:89]` copied verbatim from the BotGuard VM token. |
| 89 | 4 | Placeholder `1a 02 00 00`. |
| 93 | 6 | Fixed prefix `01 21 52 00 00 00`. |
| 99 | 1 | Metadata submessage length `L`. |
| 100 | `L` | Metadata submessage (request-specific). |
| 100 + L | rest | VM-generated trailing payload. |

For the shipped generator, bytes `[89:]` are stored as a single captured
metadata block so that the output reproduces the live payload byte-for-byte.

## What is generative

- `qh = hex(sha256(textQuery + g))`, where `g` is the UUID placed in slot 59.
- Bytes `[5:89]` come directly from the raw BotGuard VM token.

## What is captured

- The 5-byte request header.
- The complete payload tail from offset 89 (metadata block + VM tail).

These fragments are keyed by the signature `(qh, cid, prqid, prsid)`.

## API

The module lives in `src/waa/mod.rs` and exposes:

- `WaaGenerator` — loads a cache of wrapper fragments and assembles slot-3
  tokens.
- `Signature` — typed `(qh, cid, prqid, prsid)` lookup key.
- `WrapperFragment` — captured `header` + `metadata_block` (hex strings).

```rust
use gemini_sdk::waa::WaaGenerator;

let generator = WaaGenerator::default()?;
let slot3 = generator.generate(
    raw_token_b64url,
    "prompt text",
    "g-uuid",
    "cid",
    "prqid",
    "prsid",
)?;
```

### Constructors

- `WaaGenerator::new(path)` — load from a JSON cache file.
- `WaaGenerator::from_json(text)` — load from an in-memory JSON string.
- `WaaGenerator::default()` — load the bundled default cache.

### Methods

- `compute_qh(text_query, g_uuid)` — returns the 64-character lowercase hex
  prompt hash.
- `generate(raw_token_b64url, text_query, g_uuid, cid, prqid, prsid)` — returns
  the base64url slot-3 token with a leading `!`.
- `add_signature(qh, cid, prqid, prsid, header, metadata_block)` — registers a
  new captured wrapper fragment at runtime.
- `has_signature(...)` and `len()` — cache introspection helpers.

## Cache format

The cache is a JSON object mapping serialized signature arrays to fragment
objects:

```json
{
  "[\"qh_hex\", \"cid\", \"prqid\", \"prsid\"]": {
    "header": "c3c0a5c0a4",
    "metadata_block": "1a0200000121520000001a..."
  }
}
```

- `header` — 5-byte header as lowercase hex.
- `metadata_block` — payload tail from offset 89 as lowercase hex.

The crate ships with `src/waa/data/default_wrappers.json`, which contains the
two validated signatures from the v0.5 spike.

## Error handling

- Missing or malformed cache files return `Error::Config`.
- Invalid base64url input or a raw token shorter than 89 bytes returns
  `Error::AttestationFailed`.
- An unknown `(qh, cid, prqid, prsid)` signature returns
  `Error::AttestationFailed` and includes the computed `qh` so the caller can
capture the missing wrapper from live traffic.

## Limitations and operational notes

- The generator does **not** execute the BotGuard VM. It cannot synthesize the
  5-byte header, metadata submessage, or VM tail for unseen signatures.
- When the signature is unknown, capture the full decoded slot-3 payload from a
  real browser session and register it with `add_signature` or append it to the
  JSON cache.
- The bundled default cache is intentionally small. Production deployments
  should maintain their own cache keyed by signature.

## Verification

Unit tests in `src/waa/mod.rs` and integration tests in
`tests/waa_integration_tests.rs` reproduce both captured slot-3 payloads
byte-for-byte and validate `qh` computation and unknown-signature behavior.

Run the tests with:

```bash
cargo test waa
```
