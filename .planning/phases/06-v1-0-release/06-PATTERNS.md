# Phase 6: v1.0 Release — Pattern Map

**Phase:** 6 — v1.0 Release
**Generated:** 2026-08-10

## Files to Create

| File | Role | Closest Existing Analog | Notes |
|------|------|------------------------|-------|
| `CHANGELOG.md` | Release notes / changelog | `README.md`, `.planning/phases/*/0*-SUMMARY.md` | New top-level file; keepachangelog format; draws from phase summaries and git history |
| `RELEASE.md` (optional) | Release checklist and publish command | `CONTRIBUTING.md` (if exists) | Can be folded into CHANGELOG.md per agent discretion |
| `docs/migration-v0-to-v1.md` (optional) | Migration guide | `docs/protocol.md` | Breaking changes documented: async config builders, attestation errors |

## Files to Modify

| File | Role | Data Flow | Notes |
|------|------|-----------|-------|
| `Cargo.toml` | Package manifest | Read/verify only | Confirm required fields; version stays 0.1.0; rust-version 1.80 |
| `README.md` | Public documentation | Read/update | Add feature list: hooks, tracing, injectable client, upload progress, audio/video, tools, metrics, session save/restore |
| `CONTRIBUTING.md` | Contributor docs | Read/update (if exists) or create | Add MSRV policy; note `cargo publish --dry-run` |
| `src/lib.rs` | Public API surface | Read/audit | Confirm `#![deny(missing_docs)]`; verify re-exports do not leak protocol internals |
| `src/*.rs` | Source files | Read/audit | Remove leftover `#[allow(unused)]`/dead code; ensure public items have docs |

## Analog Code Excerpts

### Public API re-export pattern in `src/lib.rs`
```rust
pub use auth::{Cookies, CookieHeaderProvider, Credentials, CredentialsError, CredentialsProvider};
pub use chat::{
    AudioSource, ChatMessage, ChatResponse, ContentPart, Conversation, GenerationConfig,
    ImageSource, ThinkingLevel, VideoSource,
};
```
Audit goal: every re-export is intentional and documented.

### Manifest pattern in `Cargo.toml`
```toml
[package]
name = "gemini-sdk"
version = "0.1.0"
authors = ["Vitaly Kuzyaev <vitkuz573@gmail.com>"]
edition = "2021"
license = "MIT"
description = "..."
repository = "https://github.com/vitkuz573/gemini-sdk"
readme = "README.md"
keywords = ["gemini", "bard", "google", "ai", "sdk"]
categories = ["api-bindings", "network-programming", "asynchronous"]
rust-version = "1.80"
```
Audit goal: all publish-required fields present.

### Phase summary pattern (source material for CHANGELOG)
Files like `.planning/phases/01-stabilize-v0-1-core/01-04-SUMMARY.md` contain bulleted outcomes that feed CHANGELOG.md sections.

## PATTERN MAPPING COMPLETE
