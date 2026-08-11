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
