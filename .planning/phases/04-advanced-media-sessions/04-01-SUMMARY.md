# Phase 4, Plan 01 Summary: Audio/Video Upload Support

## Objective
Add audio and video upload support that mirrors the existing image path, delivering requirement MEDIA-03.

## Changes Made

### `src/chat.rs`
- Added public `AudioSource` and `VideoSource` enums with `InlineData { mime_type, data }` and `Url { url }` variants.
- Added `ChatMessage::with_audio` and `ChatMessage::with_video` constructors.
- Extended `ContentPart` with `Audio(AudioSource)` and `Video(VideoSource)` variants.
- Extended `PreparedRequest` with `inline_audio` and `inline_video` vectors.
- Updated `prepare_request` to extract inline audio/video data, rejecting `Url` variants.
- Updated `extract_prompt` to ignore the new part variants.
- Added unit tests for extraction and URL rejection.

### `src/upload.rs`
- Added `is_allowed_media_type` helper with an allowlist covering `image/*`, audio types (`audio/mp3`, `audio/mpeg`, `audio/wav`, `audio/ogg`), and video types (`video/mp4`, `video/webm`, `video/quicktime`).
- Updated `upload_attachments` to process images, audio, and video inline data, rejecting unsupported MIME types with `Error::bad_request`.
- Added unit tests for the MIME allowlist.

### `src/proto/slots.rs`
- Extended `derive_attachment_filename` to map audio/video MIME types to file extensions (`mp3`, `wav`, `ogg`, `mp4`, `webm`, `mov`).
- Added unit tests for the new extension mappings.

### `src/lib.rs`
- Re-exported `AudioSource` and `VideoSource`.

### Test/Bench Updates
- Updated `PreparedRequest` struct literals in `tests/proto_tests.rs`, `tests/integration_tests.rs`, and `benches/slot_building.rs` to include the new fields.

### `src/client.rs` and `src/proto/parser.rs`
- Updated match arms on `ContentPart` to cover the new audio and video variants.

## Verification
- `cargo check --all-targets`: passed
- `cargo test --all-targets`: passed (174 tests across all targets)
- `cargo clippy --all-targets -- -D warnings`: passed

## Threat Register
- T-04-01: Unsupported MIME types are rejected before any network call.
- T-04-02: Existing upload response handling does not leak secrets.
