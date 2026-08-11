# Phase 14: Model, Chat & Upload Constants — Summary

**Phase:** 14-model-chat-upload-constants  
**Plan:** 01  
**Status:** Complete  
**Completed:** 2026-08-11

## What Changed

- Extended `src/constants.rs` with:
  - `mime` module — centralized MIME types for images, audio, video, PDF, JSON, plain text, form-urlencoded, and JSON+protobuf, plus helper functions for supported image/audio/video MIME lists.
  - `roles` module — `user` and `model` role strings.
  - `model_keywords` module — category derivation keywords (`lite`, `thinking`, `deep`, `pro`, `auto`, `flash`) and display titles.
  - `upload` module — upload command/header strings, protocol values, tenant id, and upload path.
- Refactored `src/models.rs` `derive_category` to use model keyword constants.
- Refactored `src/chat.rs` `ChatMessage::user` / `ChatMessage::model` to use role constants.
- Refactored `src/proto/slots.rs` `derive_attachment_filename` to use MIME constants.
- Refactored `src/upload.rs` to use upload URL, header, and MIME constants.

## Verification

- `cargo test --all-targets` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo doc --no-deps` passed.

## Notes

- Public API unchanged.
- Remaining inline MIME literals in tests and doc comments are intentional or in `#[cfg(test)]` blocks.
