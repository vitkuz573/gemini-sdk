# Phase 4: Advanced Media & Sessions - Research

**Researched:** 2026-08-10
**Domain:** Rust SDK; Google Gemini web frontend media upload protocol; snapshot serialization
**Confidence:** MEDIUM

## Summary

Phase 4 extends the existing image upload path to audio and video, and adds session save/restore helpers. Both areas can be built almost entirely on established codebase patterns.

The media work mirrors `ImageSource`: add `AudioSource` and `VideoSource` enums, extend `ContentPart`, teach `prepare_request` to collect audio/video inline data, reuse the resumable `push.clients6.google.com/upload` flow, and emit the same slot-0 attachment shape used for images. The only uncertainty is the exact audio/video MIME allowlist accepted by the live Gemini frontend; the CONTEXT.md allowlist is a reasonable starting set and can be validated against protocol tests.

The session persistence work is straightforward serde. `Credentials` already stores secrets in plain fields, so snapshot serialization must not derive `Debug` and must use a custom redaction if `Debug` is added later. A versioned snapshot envelope makes future migrations possible without breaking restore.

**Primary recommendation:** Mirror the `ImageSource` pattern for audio/video; use a versioned JSON envelope for session snapshots; keep snapshots opaque to callers.

## User Constraints

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

### Deferred Ideas
- Tools / function calling (ADV-01) deferred to Phase 5.
- Auto cookie refresh (ADV-03) deferred to Phase 5.
- Publish to crates.io (TOOL-05) deferred to Phase 6.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde` | 1.x | Derive `Serialize`/`Deserialize` for snapshot types | Ecosystem standard already used in SDK |
| `serde_json` | 1.x | Snapshot serialization format | Already a dependency; human readable and versionable |
| `base64` | 0.22.x | Inline media encoding | Already used for images in `chat.rs` |
| `reqwest` | 0.12.x | HTTP client for uploads | Already used by upload flow |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `mime` (optional) | N/A | MIME type parsing/validation | Not required; keep inline string matching to mirror existing code |

## Architecture Patterns

1. **Media enum mirror.** `AudioSource` and `VideoSource` should be independent public enums identical in shape to `ImageSource`. A shared `MediaSource` trait is optional; the phase scope allows either.
2. **Prepared request collector.** Extend `PreparedRequest` with `inline_audio` and `inline_video` vectors of `(mime_type, base64_data)` tuples, then collect them in `prepare_request` alongside `inline_images`.
3. **Unified attachment upload.** Reuse `upload_attachments` by iterating over all inline media collections and producing `WebAttachment` values. No separate upload endpoint is needed: the resumable upload endpoint is media-agnostic.
4. **Slot 0 attachment shape.** The current `build_slot0` emits `[[reference, 1, null, mime_type], filename, null, null, null, null, null, null, [0]]` per attachment. Audio and video attachments use the same shape; only `mime_type` differs.
5. **Versioned snapshot envelope.** Define a `Snapshot` struct with `format_version: u32` and nested fields for credentials, session state, and optional conversation. This lets future phases migrate snapshots without breaking restore.
6. **Debug redaction for credentials.** `Credentials` must not derive `Debug`. Keep the existing hand-written `Debug` implementation that redacts secrets.

## Don't Hand-Roll

- MIME type parsing: use simple string allowlist matching (existing pattern) rather than pulling in a MIME crate.
- Base64 encoding/decoding: reuse the `base64` crate already in the dependency tree.
- Upload resumption/chunking: reuse the existing two-step `start_upload`/`finalize_upload` flow; do not implement a new upload protocol.
- JSON serialization: use `serde_json`, not ad-hoc string building.

## Common Pitfalls

- Changing `ImageSource` shape or breaking the existing image path.
- Deriving `Debug` on `Credentials` and leaking secrets in snapshots.
- Forgetting to include audio/video MIME types in `derive_attachment_filename` so filenames get a reasonable extension.
- Not handling `Url` variants for audio/video (they exist in the enum but do not require upload; they still need to be represented in slot 0 if used).
- Serializing `SessionState` fields that should stay internal (e.g., WAA tokens) without versioning; the versioned envelope mitigates this.

## Code Examples

### Existing pattern: `ImageSource`

```rust
#[derive(Debug, Clone)]
pub enum ImageSource {
    InlineData { mime_type: String, data: String },
    Url { url: String },
}
```

### Existing pattern: `ContentPart`

```rust
#[derive(Debug, Clone)]
pub enum ContentPart {
    Text(String),
    Thinking(String),
    Image(ImageSource),
}
```

### Existing pattern: `PreparedRequest`

```rust
pub struct PreparedRequest {
    pub prompt: String,
    pub inline_images: Vec<(String, String)>,
    pub config: Option<GenerationConfig>,
    pub category: ModelCategory,
}
```

### Existing pattern: `build_slot0` attachment shape

```rust
json!([
    [att.reference.clone(), 1, null, att.mime_type.clone()],
    att.filename.clone(),
    null, null, null, null, null, null,
    [0]
])
```

## Package Legitimacy Audit

No new external packages are required for this phase. All functionality uses existing dependencies (`serde`, `serde_json`, `base64`, `reqwest`).

## Threat Model Notes

- Snapshot strings contain credentials in recoverable form; callers must store them securely. The plan should document this risk and avoid adding convenience methods that write snapshots to disk without explicit caller action.
- Do not derive `Debug` for `Credentials`; keep redaction.
- The upload endpoint is unchanged; continue to validate the upload URL origin (`*.google.com`) as the existing code does.

## RESEARCH COMPLETE
