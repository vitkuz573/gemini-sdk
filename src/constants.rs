//! Cross-cutting protocol constants for the Gemini web frontend.
//!
//! This module centralizes literals used across the transport, session, and
//! RPC layers so protocol drift is easier to detect and update.

/// Base URLs and URL paths used by the SDK.
pub mod urls {
    /// Gemini web frontend base URL.
    pub(crate) const GEMINI_BASE_URL: &str = "https://gemini.google.com";
    /// WAA (Web Application Authentication) service base URL.
    pub(crate) const WAA_BASE_URL: &str = "https://waa-pa.clients6.google.com";
    /// OGADS service base URL.
    pub(crate) const OGADS_BASE_URL: &str = "https://ogads-pa.clients6.google.com";
    /// Resumable upload service base URL.
    pub(crate) const PUSH_UPLOAD_BASE_URL: &str = "https://push.clients6.google.com";
    /// Path to the Gemini app entry point.
    pub(crate) const APP_PATH: &str = "/app";
    /// Templated app path including the `hl` language parameter.
    pub(crate) const APP_LANGUAGE_PATH_TEMPLATE: &str = "/app?hl={}";
    /// Path for batchexecute RPC calls.
    pub(crate) const BATCHEXECUTE_PATH: &str = "/_/BardChatUi/data/batchexecute";
    /// Source path prefix for conversation action RPCs (appended with the
    /// conversation id at runtime).
    pub(crate) const CONVERSATION_ACTION_SOURCE_PATH_PREFIX: &str = "/app/";
    /// Source path used by usage-stats RPCs.
    pub(crate) const USAGE_SOURCE_PATH: &str = "/usage";
    /// Source path used by scheduled prompts RPCs.
    pub(crate) const SCHEDULED_SOURCE_PATH: &str = "/scheduled";
    /// Default source path for batchexecute RPCs without a specific page.
    pub(crate) const DEFAULT_SOURCE_PATH: &str = "/";
}

/// Query keys used in batchexecute and app requests.
pub mod query_keys {
    /// RPC identifier list query key.
    pub(crate) const RPCIDS: &str = "rpcids";
    /// Source path query key.
    pub(crate) const SOURCE_PATH: &str = "source-path";
    /// Host language query key.
    pub(crate) const HL: &str = "hl";
    /// Per-page request counter query key.
    pub(crate) const REQID: &str = "_reqid";
    /// Response type query key.
    pub(crate) const RT: &str = "rt";
    /// Value used for the `rt` query key.
    pub(crate) const RT_VALUE: &str = "c";
    /// Build label query key.
    pub(crate) const BL: &str = "bl";
    /// Frame/session id query key.
    pub(crate) const F_SID: &str = "f.sid";
}

/// Transport-level markers and keys used by WIZ/batchexecute.
///
/// `ANTI_XSSI_PREFIX` and `RPC_FRAME_MARKER` are public because they are
/// re-exported by the `proto` module for use by response parsing helpers.
pub mod transport {
    /// WIZ anti-XSSI prefix used by `batchexecute` and `StreamGenerate` responses.
    pub const ANTI_XSSI_PREFIX: &str = ")] } ' \n\n";
    /// Key for the URL-encoded request payload in form bodies.
    pub(crate) const F_REQ_KEY: &str = "f.req";
    /// Discriminator used to identify batchexecute endpoints when building headers.
    pub(crate) const BATCHEXECUTE_ENDPOINT: &str = "batchexecute";
    /// Frame marker key in WIZ batchexecute response arrays.
    pub const RPC_FRAME_MARKER: &str = "wrb.fr";
}

/// Keys extracted from `window.WIZ_global_data` and related session objects.
pub mod wiz_keys {
    /// Gaia id key (legacy signed-in marker).
    pub(crate) const S06_GRB: &str = "S06Grb";
    /// Email address key (legacy signed-in marker).
    pub(crate) const OPEP_7C: &str = "oPEP7c";
    /// Session id key (maps to `f.sid`).
    pub(crate) const FDR_FJE: &str = "FdrFJe";
    /// Build label key.
    pub(crate) const CFB2H: &str = "cfb2h";
    /// Frame/session id key alias.
    pub(crate) const F_SID: &str = "f.sid";
    /// Session id fallback key.
    pub(crate) const SESSION_ID: &str = "session_id";
}

/// RPC identifiers used by batchexecute requests.
pub mod rpc_ids {
    /// `GetUserStatus` / model list RPC id.
    pub(crate) const OTAQ7B_RPC_ID: &str = "otAQ7b";
    /// `GetUserInfo` / signed-in diagnostics RPC id.
    pub(crate) const FD0QJE_RPC_ID: &str = "Fd0Qje";
    /// `K4WWud` locale/tools RPC id.
    pub(crate) const K4WWUD_RPC_ID: &str = "K4WWud";
}

/// MIME types used by the SDK for attachments and request bodies.
pub mod mime {
    /// PNG image MIME type.
    pub(crate) const PNG: &str = "image/png";
    /// JPEG image MIME type.
    pub(crate) const JPEG: &str = "image/jpeg";
    /// WebP image MIME type.
    pub(crate) const WEBP: &str = "image/webp";
    /// GIF image MIME type.
    pub(crate) const GIF: &str = "image/gif";
    /// PDF document MIME type.
    pub(crate) const PDF: &str = "application/pdf";
    /// MP3 audio MIME type.
    pub(crate) const MP3: &str = "audio/mp3";
    /// MPEG audio MIME type.
    pub(crate) const MPEG_AUDIO: &str = "audio/mpeg";
    /// WAV audio MIME type.
    pub(crate) const WAV: &str = "audio/wav";
    /// Ogg audio MIME type.
    pub(crate) const OGG_AUDIO: &str = "audio/ogg";
    /// MP4 video MIME type.
    pub(crate) const MP4_VIDEO: &str = "video/mp4";
    /// WebM video MIME type.
    pub(crate) const WEBM_VIDEO: &str = "video/webm";
    /// QuickTime video MIME type.
    pub(crate) const QUICKTIME: &str = "video/quicktime";
    /// JSON MIME type.
    pub(crate) const JSON: &str = "application/json";
    /// Plain text MIME type.
    pub(crate) const PLAIN_TEXT: &str = "text/plain";
    /// Form-urlencoded request body MIME type.
    pub(crate) const FORM_URLENCODED: &str = "application/x-www-form-urlencoded;charset=UTF-8";
    /// JSON+protobuf request body MIME type.
    pub(crate) const JSON_PROTOBUF: &str = "application/json+protobuf";

    /// Returns the MIME types supported for inline images.
    pub(crate) fn supported_image_mime_types() -> &'static [&'static str] {
        &[PNG, JPEG, WEBP, GIF]
    }

    /// Returns the MIME types supported for inline audio.
    pub(crate) fn supported_audio_mime_types() -> &'static [&'static str] {
        &[MP3, MPEG_AUDIO, WAV, OGG_AUDIO]
    }

    /// Returns the MIME types supported for inline video.
    pub(crate) fn supported_video_mime_types() -> &'static [&'static str] {
        &[MP4_VIDEO, WEBM_VIDEO, QUICKTIME]
    }
}

/// Chat message role strings.
pub mod roles {
    /// User role.
    pub(crate) const USER: &str = "user";
    /// Model role.
    pub(crate) const MODEL: &str = "model";
}

/// Model category derivation keywords and display titles.
pub mod model_keywords {
    /// Keyword for Flash-Lite models.
    pub(crate) const LITE: &str = "lite";
    /// Keyword for thinking / reasoning models.
    pub(crate) const THINKING: &str = "thinking";
    /// Keyword for deep-reasoning models.
    pub(crate) const DEEP: &str = "deep";
    /// Keyword for Pro models.
    pub(crate) const PRO: &str = "pro";
    /// Keyword for Auto / fallback models.
    pub(crate) const AUTO: &str = "auto";
    /// Keyword for Flash models.
    pub(crate) const FLASH: &str = "flash";
    /// Display title for Flash models.
    pub(crate) const TITLE_FLASH: &str = "Flash";
    /// Display title for Pro models.
    pub(crate) const TITLE_PRO: &str = "Pro";
}

/// Upload endpoint and header constants.
pub mod upload {
    /// Upload command header value for starting a resumable upload.
    pub(crate) const UPLOAD_COMMAND_START: &str = "start";
    /// Upload command header value for finalizing a resumable upload.
    pub(crate) const UPLOAD_COMMAND_FINALIZE: &str = "upload, finalize";
    /// Header name for the upload command directive.
    pub(crate) const X_GOOG_UPLOAD_COMMAND: &str = "x-goog-upload-command";
    /// Header name for the total content length hint.
    pub(crate) const X_GOOG_UPLOAD_HEADER_CONTENT_LENGTH: &str = "x-goog-upload-header-content-length";
    /// Header name for the upload protocol selection.
    pub(crate) const X_GOOG_UPLOAD_PROTOCOL: &str = "x-goog-upload-protocol";
    /// Header name returned with the resumable upload URL.
    pub(crate) const X_GOOG_UPLOAD_URL: &str = "x-goog-upload-url";
    /// Header name for the tenant identifier.
    pub(crate) const X_TENANT_ID: &str = "x-tenant-id";
    /// Header name for the push service identifier.
    pub(crate) const PUSH_ID_HEADER: &str = "push-id";
    /// Value for the resumable upload protocol.
    pub(crate) const RESUMABLE_PROTOCOL: &str = "resumable";
    /// Upload path appended to the push upload base URL.
    pub(crate) const UPLOAD_PATH: &str = "/upload/";
    /// Tenant identifier used for Bard uploads.
    pub(crate) const BARD_STORAGE_TENANT: &str = "bard-storage";
}
