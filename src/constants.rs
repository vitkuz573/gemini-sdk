//! Cross-cutting protocol constants for the Gemini web frontend.
//!
//! This module centralizes literals used across the transport, session, and
//! RPC layers so protocol drift is easier to detect and update.

/// Base URLs and URL paths used by the SDK.
pub mod urls {
    /// Gemini web frontend base URL.
    pub const GEMINI_BASE_URL: &str = "https://gemini.google.com";
    /// WAA (Web Application Authentication) service base URL.
    pub(crate) const WAA_BASE_URL: &str = "https://waa-pa.clients6.google.com";
    /// OGADS service base URL.
    pub(crate) const OGADS_BASE_URL: &str = "https://ogads-pa.clients6.google.com";
    /// Resumable upload service base URL.
    pub(crate) const PUSH_UPLOAD_BASE_URL: &str = "https://push.clients6.google.com";
    /// Path to the Gemini app entry point.
    pub const APP_PATH: &str = "/app";
    /// Templated app path including the `hl` language parameter.
    pub(crate) const APP_LANGUAGE_PATH_TEMPLATE: &str = "/app?hl={}";
    /// Path for batchexecute RPC calls.
    pub const BATCHEXECUTE_PATH: &str = "/_/BardChatUi/data/batchexecute";
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
    pub const HL: &str = "hl";
    /// Per-page request counter query key.
    pub const REQID: &str = "_reqid";
    /// Response type query key.
    pub const RT: &str = "rt";
    /// Value used for the `rt` query key.
    pub const RT_VALUE: &str = "c";
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
    pub const PNG: &str = "image/png";
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
    pub const FORM_URLENCODED: &str = "application/x-www-form-urlencoded;charset=UTF-8";
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

/// HTTP method names used for HAR recording and request dispatch.
pub mod http_methods {
    /// HTTP GET method.
    pub(crate) const GET: &str = "GET";
    /// HTTP POST method.
    pub(crate) const POST: &str = "POST";
}

/// Static HTTP header names and values.
pub mod headers {
    /// `Content-Type` header name.
    pub(crate) const CONTENT_TYPE: &str = "Content-Type";
    /// `User-Agent` header name.
    pub(crate) const USER_AGENT: &str = "User-Agent";
    /// `Origin` header name.
    pub(crate) const ORIGIN: &str = "Origin";
    /// `Referer` header name.
    pub(crate) const REFERER: &str = "Referer";
    /// `X-Same-Domain` header name.
    pub(crate) const X_SAME_DOMAIN: &str = "X-Same-Domain";
    /// `Cache-Control` header name.
    pub(crate) const CACHE_CONTROL: &str = "Cache-Control";
    /// `Pragma` header name.
    pub(crate) const PRAGMA: &str = "Pragma";
    /// `x-client-data` header name.
    pub(crate) const X_CLIENT_DATA: &str = "x-client-data";
    /// `Cookie` header name.
    pub const COOKIE: &str = "Cookie";
    /// `Authorization` header name.
    pub(crate) const AUTHORIZATION: &str = "Authorization";
    /// `Set-Cookie` header name.
    pub(crate) const SET_COOKIE: &str = "Set-Cookie";
    /// `Accept` header name.
    pub(crate) const ACCEPT: &str = "Accept";
    /// `Content-Length` header name.
    pub(crate) const CONTENT_LENGTH: &str = "Content-Length";
    /// `sec-fetch-site` header name.
    pub(crate) const SEC_FETCH_SITE: &str = "sec-fetch-site";
    /// `x-goog-api-key` header name.
    pub(crate) const X_GOOG_API_KEY: &str = "x-goog-api-key";
    /// `x-user-agent` header name.
    pub(crate) const X_USER_AGENT: &str = "x-user-agent";
    /// `x-user-agent` header value.
    pub(crate) const X_USER_AGENT_VALUE: &str = "grpc-web-javascript/0.1";
    /// `x-client-data` value used for WAA/OGADS requests.
    pub(crate) const X_CLIENT_DATA_VALUE: &str = "CNeOywE=";
    /// Common prefix for `x-goog-ext-*` extension headers.
    pub(crate) const X_GOOG_EXT_PREFIX: &str = "x-goog-ext-";
    /// `x-goog-authuser` header name.
    ///
    /// Observed on browser `jSf9Qc` (usage stats) batchexecute requests.
    pub(crate) const X_GOOG_AUTHUSER: &str = "x-goog-authuser";
    /// `x-goog-authuser` header value.
    ///
    /// Browser usage-stats requests consistently send `0` for the default
    /// signed-in account.
    pub(crate) const X_GOOG_AUTHUSER_VALUE: &str = "0";

    /// `X-Same-Domain` header value.
    pub(crate) const X_SAME_DOMAIN_VALUE: &str = "1";
    /// `Cache-Control: no-cache` value.
    pub(crate) const CACHE_CONTROL_NO_CACHE: &str = "no-cache";
    /// `Pragma: no-cache` value.
    pub(crate) const PRAGMA_NO_CACHE: &str = "no-cache";
    /// `sec-ch-ua` full header value for Chrome 146.
    pub(crate) const SEC_CH_UA: &str =
        "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"146\", \"Chromium\";v=\"146\"";
    /// `sec-ch-ua-mobile` value.
    pub(crate) const SEC_CH_UA_MOBILE: &str = "?0";
    /// `sec-ch-ua-platform` value.
    pub(crate) const SEC_CH_UA_PLATFORM: &str = "\"Windows\"";
    /// `sec-fetch-dest` value.
    pub(crate) const SEC_FETCH_DEST: &str = "empty";
    /// `sec-fetch-mode` value.
    pub(crate) const SEC_FETCH_MODE: &str = "cors";
    /// `sec-fetch-site` same-origin value.
    pub(crate) const SEC_FETCH_SITE_SAME_ORIGIN: &str = "same-origin";
    /// `sec-fetch-site` cross-site value.
    pub(crate) const SEC_FETCH_SITE_CROSS_SITE: &str = "cross-site";
}

/// Browser-like user agent strings.
pub mod user_agents {
    /// User agent for Gemini web requests (Chrome 146).
    pub(crate) const BROWSER_LIKE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36";
    /// User agent for push upload requests (Chrome 133).
    pub const UPLOAD_BROWSER_LIKE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";
}

/// HAR/redaction constants.
pub mod har {
    /// W3C HAR version.
    pub(crate) const VERSION: &str = "1.2";
    /// HAR creator name.
    pub(crate) const CREATOR_NAME: &str = "gemini-sdk";
    /// Request MIME type for form-encoded batchexecute bodies.
    pub(crate) const REQUEST_MIME_TYPE: &str = "application/x-www-form-urlencoded;charset=UTF-8";
    /// Default response MIME type.
    pub(crate) const RESPONSE_MIME_TYPE: &str = "application/json";
    /// Replacement value for redacted fields.
    pub(crate) const REDACTED_VALUE: &str = "<redacted>";

    /// `__Secure-1PSID` cookie name.
    pub(crate) const SECURE_1_PSID: &str = "__Secure-1PSID";
    /// `__Secure-1PSIDCC` cookie name.
    pub(crate) const SECURE_1_PSIDCC: &str = "__Secure-1PSIDCC";
    /// `__Secure-3PSID` cookie name.
    pub(crate) const SECURE_3_PSID: &str = "__Secure-3PSID";
    /// `__Secure-3PSIDCC` cookie name.
    pub(crate) const SECURE_3_PSIDCC: &str = "__Secure-3PSIDCC";
    /// `SAPISID` cookie name.
    pub(crate) const SAPISID: &str = "SAPISID";
    /// `APISID` cookie name.
    pub(crate) const APISID: &str = "APISID";
    /// `SSID` cookie name.
    pub(crate) const SSID: &str = "SSID";
    /// `HSID` cookie name.
    pub(crate) const HSID: &str = "HSID";
    /// `SID` cookie name.
    pub(crate) const SID: &str = "SID";
    /// `SOCS` cookie name.
    pub(crate) const SOCS: &str = "SOCS";
    /// Authorization bearer prefix in form data.
    pub(crate) const AUTHORIZATION_BEARER_PREFIX: &str = "authorization=Bearer ";
    /// Access token prefix in form data.
    pub(crate) const ACCESS_TOKEN_PREFIX: &str = "access_token=";
}

/// Transient WIZ 400 response markers.
pub mod transient {
    /// `"er"` marker.
    pub(crate) const ER_MARKER: &str = "\"er\"";
    /// `"di"` marker.
    pub(crate) const DI_MARKER: &str = "\"di\"";
    /// `"af.httprm"` marker.
    pub(crate) const HTTPRM_MARKER: &str = "\"af.httprm\"";
    /// Title substring used in Google sign-in redirect pages.
    pub(crate) const SIGN_IN_REDIRECT_TITLE_SUBSTRING: &str = "Sign in - Google Accounts";
}

/// Tracing span names, metric names, and attribute keys.
#[allow(clippy::module_name_repetitions)]
pub mod tracing_names {
    /// Prefix shared by all SDK tracing span names.
    pub(crate) const SPAN_PREFIX: &str = "gemini.";

    /// Span/operation name for `regenerate_turn`.
    pub(crate) const REGENERATE_TURN: &str = "gemini.regenerate_turn";
    /// Span/operation name for `rate_turn`.
    pub(crate) const RATE_TURN: &str = "gemini.rate_turn";
    /// Span/operation name for `delete_turn`.
    pub(crate) const DELETE_TURN: &str = "gemini.delete_turn";
    /// Span/operation name for `get_user_info`.
    pub(crate) const GET_USER_INFO: &str = "gemini.get_user_info";
    /// Span/operation name for `get_last_selected_mode`.
    pub(crate) const GET_LAST_SELECTED_MODE: &str = "gemini.get_last_selected_mode";
    /// Span/operation name for `set_last_selected_mode`.
    pub(crate) const SET_LAST_SELECTED_MODE: &str = "gemini.set_last_selected_mode";
    /// Span/operation name for `get_locale_tools`.
    pub(crate) const GET_LOCALE_TOOLS: &str = "gemini.get_locale_tools";
    /// Span/operation name for `get_model_config`.
    pub(crate) const GET_MODEL_CONFIG: &str = "gemini.get_model_config";
    /// Span/operation name for `get_locale_config`.
    pub(crate) const GET_LOCALE_CONFIG: &str = "gemini.get_locale_config";
    /// Span/operation name for `get_tools_config`.
    pub(crate) const GET_TOOLS_CONFIG: &str = "gemini.get_tools_config";
    /// Span/operation name for `get_usage_stats`.
    pub(crate) const GET_USAGE_STATS: &str = "gemini.get_usage_stats";
    /// Span/operation name for `get_scheduled_prompts`.
    pub(crate) const GET_SCHEDULED_PROMPTS: &str = "gemini.get_scheduled_prompts";
    /// Span/operation name for `list_models`.
    pub(crate) const LIST_MODELS: &str = "gemini.list_models";
    /// Span/operation name for `generate`.
    pub(crate) const GENERATE: &str = "gemini.generate";
    /// Span/operation name for `generate_with_tools`.
    pub(crate) const GENERATE_WITH_TOOLS: &str = "gemini.generate_with_tools";
    /// Span/operation name for `generate_stream`.
    pub(crate) const GENERATE_STREAM: &str = "gemini.generate_stream";
    /// Span/operation name for `generate_with_conversation`.
    pub(crate) const GENERATE_WITH_CONVERSATION: &str = "gemini.generate_with_conversation";
    /// Span/operation name for `upload_with_progress`.
    pub(crate) const UPLOAD_WITH_PROGRESS: &str = "gemini.upload_with_progress";
    /// Span/operation name for `verify_signed_in`.
    pub(crate) const VERIFY_SIGNED_IN: &str = "gemini.verify_signed_in";
    /// Span/operation name for `diagnose_signed_in`.
    pub(crate) const DIAGNOSE_SIGNED_IN: &str = "gemini.diagnose_signed_in";
    /// Span/operation name for the WAA init chain.
    pub(crate) const WAA_INIT_CHAIN: &str = "gemini.waa_init_chain";
    /// Span/operation name for response parsing.
    pub(crate) const PARSE_RESPONSE: &str = "gemini.parse_response";
    /// Span/operation name for `generate_raw`.
    pub(crate) const GENERATE_RAW: &str = "gemini.generate_raw";
    /// Span/operation name for conversation state ingestion.
    pub(crate) const INGEST_CONVERSATION_STATE: &str = "gemini.ingest_conversation_state";

    /// Request counter metric name.
    pub(crate) const METRIC_REQUESTS: &str = "gemini_sdk.requests";
    /// Retry counter metric name.
    pub(crate) const METRIC_RETRIES: &str = "gemini_sdk.retries";
    /// Request latency histogram metric name.
    pub(crate) const METRIC_REQUEST_LATENCY: &str = "gemini_sdk.request_latency";

    /// Operation attribute key.
    pub(crate) const OPERATION: &str = "operation";
    /// Status attribute key.
    pub(crate) const STATUS: &str = "status";
    /// Category attribute key.
    pub(crate) const CATEGORY: &str = "category";
    /// Bytes attribute key.
    pub(crate) const BYTES: &str = "bytes";
}

/// Browser attestation CDP strings and defaults.
pub mod attestation {
    use std::time::Duration;

    // Regression note: the constants in this module are intentionally left as
    // `pub(crate)` because they are only used inside the crate. They are not
    // part of the public API and do not need to be exposed to examples or tests.

    /// CDP `Runtime.enable` method.
    pub(crate) const RUNTIME_ENABLE: &str = "Runtime.enable";
    /// CDP `Network.enable` method.
    pub(crate) const NETWORK_ENABLE: &str = "Network.enable";
    /// CDP `Page.enable` method.
    pub(crate) const PAGE_ENABLE: &str = "Page.enable";
    /// CDP `Network.setCookie` method.
    pub(crate) const NETWORK_SET_COOKIE: &str = "Network.setCookie";
    /// CDP `Page.navigate` method.
    pub(crate) const PAGE_NAVIGATE: &str = "Page.navigate";
    /// CDP `Runtime.evaluate` method.
    pub(crate) const RUNTIME_EVALUATE: &str = "Runtime.evaluate";

    /// Cookie domain injected into Chrome.
    pub(crate) const CHROME_DOMAIN: &str = ".google.com";
    /// Cookie path injected into Chrome.
    pub(crate) const CHROME_PATH: &str = "/";
    /// Navigate URL template for attestation, parameterized by language.
    pub(crate) const NAVIGATE_URL_TEMPLATE: &str = "https://gemini.google.com/app?hl={}";
    /// Default language used for attestation navigation.
    pub(crate) const LANGUAGE_FOR_ATTESTATION: &str = "en";

    /// Send button test id selector.
    pub(crate) const SEND_BUTTON_SELECTOR: &str = "[data-test-id=\"send-button\"]";
    /// Send button aria-label selector.
    pub(crate) const SEND_BUTTON_ARIA_LABEL: &str = "button[aria-label*=\"Send\"]";
    /// Prompt textarea selector.
    pub(crate) const TEXTAREA_SELECTOR: &str = "textarea";

    /// Chrome profile path used by attestation.
    pub(crate) const CHROME_PROFILE_PATH: &str = "/tmp/gemini-sdk-chrome-profile";
    /// Chrome flags passed when launching the browser.
    pub(crate) const CHROME_FLAGS: &[&str] = &[
        "--headless=new",
        "--no-sandbox",
        "--disable-gpu",
        "--disable-dev-shm-usage",
        "--remote-debugging-port=0",
    ];

    /// Maximum time to wait for CDP navigation.
    pub(crate) const NAVIGATE_TIMEOUT_SECONDS: u64 = 60;
    /// Maximum time to wait for StreamGenerate post data capture.
    pub(crate) const CAPTURE_TIMEOUT_SECONDS: u64 = 60;

    /// Returns the default navigation timeout duration.
    pub(crate) fn navigate_timeout() -> Duration {
        Duration::from_secs(NAVIGATE_TIMEOUT_SECONDS)
    }

    /// Returns the default capture timeout duration.
    pub(crate) fn capture_timeout() -> Duration {
        Duration::from_secs(CAPTURE_TIMEOUT_SECONDS)
    }
}

/// JSON Schema keys used when declaring tool schemas.
pub mod tool_schema {
    /// `type` key.
    pub(crate) const TYPE: &str = "type";
    /// `object` type value.
    pub(crate) const OBJECT: &str = "object";
    /// `properties` key.
    pub(crate) const PROPERTIES: &str = "properties";
    /// `required` key.
    pub(crate) const REQUIRED: &str = "required";
    /// `name` key.
    pub(crate) const NAME: &str = "name";
    /// `parameters` key.
    pub(crate) const PARAMETERS: &str = "parameters";
}

/// Authentication diagnostic messages.
pub mod auth {
    /// Advice appended when legacy cookies are missing from the supplied header.
    pub(crate) const MISSING_LEGACY_COOKIES_ADVICE: &str = " Copy the full signed-in cookie header from the browser, including SID, HSID, SSID, APISID, SAPISID, SIDCC, __Secure-ENID, and NID.";
}

#[cfg(test)]
mod regression_tests {
    //! Regression gate: fail if eliminated magic strings reappear in `src/`.
    //!
    //! The deny-list contains protocol literals that have been centralized in
    //! this module. The test walks every `.rs` file under `src/` except this
    //! file and asserts that none of the literals appear as inline strings.

    use std::fs;
    use std::path::Path;

    /// Literals that must not appear inline outside `src/constants.rs`.
    ///
    /// The list is intentionally conservative: it avoids strings that appear
    /// legitimately in doc comments, test fixtures, or error messages (e.g.
    /// `/app` is referenced in many doc strings, `wrb.fr` appears in parser
    /// error messages and fixture data, `<redacted>` is the documented redaction
    /// token, and key names like `S06Grb`/`oPEP7c`/`FdrFJe` are used in
    /// diagnostics). The gate targets protocol values that are cheap to
    /// centralize and high-risk to duplicate.
    const DENY_LIST: &[&str] = &[
        "https://gemini.google.com/_/BardChatUi/data/batchexecute",
        "application/json+protobuf",
        "application/x-www-form-urlencoded;charset=UTF-8",
        "bard-storage",
        "x-goog-upload-command",
    ];

    #[test]
    fn no_deny_list_literals_in_source() {
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src_dir = crate_root.join("src");
        let constants_path = src_dir.join("constants.rs");

        let mut failures = Vec::new();
        visit_dir(&src_dir, &constants_path, &mut failures);

        if !failures.is_empty() {
            let mut message = String::from(
                "regression gate failed: eliminated magic strings found in src/:\n",
            );
            for (file, literal) in &failures {
                message.push_str(&format!("  {file:?}: {literal}\n"));
            }
            panic!("{message}");
        }
    }

    fn visit_dir(dir: &Path, skip: &Path, failures: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).expect("src/ should be readable") {
            let entry = entry.expect("directory entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                visit_dir(&path, skip, failures);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs")
                && path != skip
            {
                let contents = fs::read_to_string(&path).expect("source file should be readable");
                for literal in DENY_LIST {
                    if contents.contains(literal) {
                        failures.push((path.to_string_lossy().to_string(), (*literal).to_string()));
                    }
                }
            }
        }
    }
}
