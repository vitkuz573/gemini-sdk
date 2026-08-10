# Phase 4: Advanced Media & Sessions - Context

**Gathered:** 2026-08-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Support richer media types and persistent sessions. Scope covers MEDIA-03 and ADV-02.

Key outcomes:

- Audio and video upload paths.
- Session save/restore helpers for conversation and auth state.

</domain>

<decisions>
## Implementation Decisions

### Audio & Video Uploads (MEDIA-03)
- Add separate public `AudioSource` and `VideoSource` enums with `InlineData { mime_type, data }` and `Url` variants, mirroring the existing `ImageSource` pattern.
- Add `ContentPart::Audio(AudioSource)` and `ContentPart::Video(VideoSource)` variants.
- Extend `prepare_request` to extract audio/video inline data from `ChatMessage::parts` and pass it through `PreparedRequest`.
- Extend `upload_attachments` in `src/upload.rs` to accept audio/video bytes and MIME types, using the same resumable upload endpoint.
- Update `build_slot0` in `src/proto/slots.rs` to emit the correct attachment shape for audio and video references.
- Maintain a supported MIME type allowlist in upload.rs (e.g., audio/mp3, audio/mpeg, audio/wav, audio/ogg, video/mp4, video/webm, video/quicktime).
- Keep `ImageSource` unchanged for backward compatibility.

### Session Persistence (ADV-02)
- Add `GeminiClient::save_session() -> Result<String>` that serializes a snapshot containing:
  - `Credentials` (with secrets; caller is responsible for safe storage)
  - `SessionState`
  - Optional current `Conversation`
- Add `GeminiClient::restore_session(snapshot: &str) -> Result<(Self, Option<Conversation>)>` that reconstructs a client and optional conversation.
- Add `Conversation::save(&self) -> Result<String>` and `Conversation::restore(snapshot: &str) -> Result<Self>` for conversation-only persistence.
- Use `serde_json` for serialization; mark snapshot format with a version field for forward compatibility.
- Ensure `Credentials` implements `Serialize`/`Deserialize` securely (do not derive Debug with secrets).

### the agent's Discretion
- Agent may choose whether to introduce a shared `MediaSource` trait or keep the enums independent.
- Agent may adjust exact MIME allowlist and slot 0 array shape based on protocol tests.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/chat.rs` — `ImageSource`, `ContentPart`, `ChatMessage`, `Conversation`, `PreparedRequest`.
- `src/upload.rs` — `upload_file`, `start_upload`, `finalize_upload`, `UploadEvent`, `upload_progress_stream`.
- `src/proto/slots.rs` — `build_slot0`, `WebAttachment`, `derive_attachment_filename`.
- `src/auth.rs` — `Credentials`, `Cookies`.
- `src/session.rs` — `SessionState`.
- `src/client.rs` — `GeminiClient`, `Inner`, `ClientConfig`.

### Established Patterns
- Content parts are enums; images use `ImageSource::InlineData { mime_type, data }`.
- Inline data is base64-encoded and uploaded via `upload_attachments`.
- `WebAttachment` carries `reference`, `mime_type`, `filename`.
- Serialization uses `serde` derive macros.
- Public API additions are re-exported from `src/lib.rs`.

### Integration Points
- `ChatMessage::with_part` → `ContentPart::Audio` / `ContentPart::Video`.
- `prepare_request` → extracts media inline data → `PreparedRequest`.
- `upload_attachments` → uploads all media types → `Vec<WebAttachment>`.
- `build_slot0` → includes attachment list in slot 0.
- `GeminiClient::save_session` / `restore_session` → JSON snapshot.

</code_context>

<specifics>
## Specific Ideas

- Reuse `derive_attachment_filename` logic for audio/video extensions.
- Add unit tests for MIME type allowlist rejection and attachment filename derivation.
- Add integration tests for save/restore round-trip without live network.
- Consider adding a `Snapshot` struct with `format_version: u32` to ease future migrations.

</specifics>

<deferred>
## Deferred Ideas

- Tools / function calling (ADV-01) deferred to Phase 5.
- Auto cookie refresh (ADV-03) deferred to Phase 5.
- Publish to crates.io (TOOL-05) deferred to Phase 6.

</deferred>
