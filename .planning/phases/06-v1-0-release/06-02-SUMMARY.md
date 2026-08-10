---
phase: 06-v1-0-release
plan: 02
subsystem: docs
status: complete
completed: 2026-08-10
requirements: [TOOL-05]
---

# Phase 6 Plan 02 Summary: Documentation and Release Notes

## Objective

Polish all user-facing documentation for the v0.1.0 release.

## What Changed

- **CHANGELOG.md**: Created at repo root following keepachangelog.com format.
  - `Unreleased` section.
  - `v0.1.0` section summarizing Added, Changed, Removed, Fixed, and Security
    highlights from all prior phases.
  - Release checklist with exact `cargo publish --all-features` command.
  - Link to `docs/migration-v0-to-v1.md`.
- **README.md**: Updated feature list to include hooks, tracing, injectable
  client, upload progress, audio/video, tools/function calling, metrics, and
  session save/restore. Fixed the quick-start example to match the current sync
  `from_cookie_header` and `ChatBuilder` API. Added MSRV note and expanded the
  development command list.
- **CONTRIBUTING.md**: Documented MSRV policy (`rust-version = "1.80"`) and
  required pre-submit checks, including `cargo publish --dry-run --all-features`.
- **docs/migration-v0-to-v1.md**: Created migration guide covering:
  - Async config builder methods (`with_language`, `with_max_retries`,
    `with_timeout`, etc.).
  - `Error::AttestationFailed` replacing silent synthetic fallback.
  - Before/after code snippets for each breaking change.
  - Note that v1.0.0 bump is deferred.

## Verification

```bash
cargo test --doc --all-features               # passed
cargo test --all-features --all-targets       # passed
cargo clippy --all-targets --all-features -- -D warnings  # passed
cargo doc --no-deps --all-features            # passed, 0 warnings
cargo publish --dry-run --all-features --allow-dirty      # passed
```

## Files Created/Modified

- Created: `CHANGELOG.md`, `docs/migration-v0-to-v1.md`
- Modified: `README.md`, `CONTRIBUTING.md`
