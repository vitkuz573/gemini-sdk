//! Main SDK client.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::Stream;
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, warn};

use crate::auth::{Cookies, Credentials, CredentialsProvider};
use crate::chat::{
    prepare_request, ChatMessage, ChatResponse, ContentPart, Conversation, GenerationConfig,
    ImageSource, PreparedRequest,
};
use crate::conversation_actions::{
    build_delete_payload, build_rate_payload, build_regenerate_payload,
    parse_conversation_action_response, ConversationAction, ConversationActionResult, TurnRating,
    PCCK7E_RPC_ID,
};
use crate::errors::{Error, Result};
use crate::har::HarWriter;
use crate::locale_model_config::{
    build_get_locale_config_payload, build_get_locale_tools_payload,
    build_get_model_config_payload, build_get_tools_config_payload, parse_locale_config_response,
    parse_locale_tools_response, parse_model_config_response, parse_tools_config_response,
    LocaleConfig, LocaleTools, ModelConfig, ToolsConfig, CYRIKD_RPC_ID, KU4JYF_RPC_ID,
    TE6DCF_RPC_ID, WHPPME_RPC_ID,
};
use crate::models::{ModelCategory, ModelInfo};
use crate::proto::parser::{
    extract_conversation_state, parse_chat_response, parse_model_list, parse_response_parts,
};
use crate::proto::slots::{build_inner_req_list, ConversationState as ProtoConversationState};
use crate::proto::{
    build_batchexecute_body, build_esy5d_body, build_ogads_body, build_sjbwce_body,
    build_stream_generate_body, build_waa_create_body, fresh_request_uuid,
};
use crate::session::{extract_consent_save_url, extract_from_app_html, SessionState};
use crate::settings::{
    build_get_scheduled_prompts_payload, build_get_usage_stats_payload,
    parse_scheduled_prompts_response, parse_usage_stats_response, ScheduledPrompts, UsageStats,
    JSF9QC_RPC_ID, XPSWPD_RPC_ID,
};
use crate::tool::{Tool, ToolError, ToolResult};
use crate::transient_400::is_wiz_transient_400;
use crate::upload::{self, UploadEvent};
use crate::user_profile::{
    build_get_last_selected_mode_payload, build_get_user_info_payload,
    build_set_last_selected_mode_payload, parse_last_selected_mode_response,
    parse_user_info_response, LastSelectedMode, UserInfo, L5ADHE_RPC_ID, O30O0E_RPC_ID,
};

/// Async hook that observes prepared requests and parsed responses.
///
/// Hooks receive only typed SDK types ([`PreparedRequest`] and [`ChatResponse`]),
/// never raw cookies, auth headers, or base64 image payloads. Implementers must
/// not rely on interior mutability to alter the request or response; the trait
/// takes immutable references for a reason. Any secret values observed inside
/// the hook must be redacted by the implementer before logging or exporting.
pub trait HttpHook: Send + Sync {
    /// Called after a request has been prepared but before it is sent.
    ///
    /// The supplied [`PreparedRequest`] contains the flattened prompt text and
    /// inline image metadata (count and MIME type), not the raw base64 bytes.
    fn on_request<'a>(
        &'a self,
        request: &'a PreparedRequest,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// Called after a response has been parsed and a [`ChatResponse`] produced.
    fn on_response<'a>(
        &'a self,
        response: &'a ChatResponse,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

/// Diagnostic result returned by [`GeminiClient::diagnose_signed_in`].
#[derive(Debug, Clone)]
pub struct AppDiagnostics {
    /// Whether `/app` contained the signed-in markers.
    pub signed_in: bool,
    /// The `S06Grb` Gaia id, if the page was signed in.
    pub gaia_id: Option<String>,
    /// The `oPEP7c` email address, if the page was signed in.
    pub email: Option<String>,
    /// Why the `/app` HTML was rejected as unsigned, if it was rejected.
    pub failure_reason: Option<String>,
    /// Legacy/account cookies that were missing from the supplied header.
    pub missing_legacy_cookies: Vec<&'static str>,
}

impl HttpHook for Arc<dyn HttpHook> {
    fn on_request<'a>(
        &'a self,
        request: &'a PreparedRequest,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        (**self).on_request(request)
    }

    fn on_response<'a>(
        &'a self,
        response: &'a ChatResponse,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        (**self).on_response(response)
    }
}

use crate::constants::auth::MISSING_LEGACY_COOKIES_ADVICE;
use crate::constants::har as har_constants;
use crate::constants::headers as header_constants;
use crate::constants::http_methods::{GET, POST};
use crate::constants::query_keys::{BL, F_SID, HL, REQID, RPCIDS, RT, RT_VALUE, SOURCE_PATH};
use crate::constants::tracing_names::{
    DELETE_TURN, DIAGNOSE_SIGNED_IN, GENERATE, GENERATE_RAW, GENERATE_STREAM,
    GENERATE_WITH_CONVERSATION, GENERATE_WITH_TOOLS, GET_LAST_SELECTED_MODE, GET_LOCALE_CONFIG,
    GET_LOCALE_TOOLS, GET_MODEL_CONFIG, GET_SCHEDULED_PROMPTS, GET_TOOLS_CONFIG, GET_USAGE_STATS,
    GET_USER_INFO, INGEST_CONVERSATION_STATE, LIST_MODELS, METRIC_RETRIES, OPERATION,
    PARSE_RESPONSE, RATE_TURN, REGENERATE_TURN, SET_LAST_SELECTED_MODE, UPLOAD_WITH_PROGRESS,
    VERIFY_SIGNED_IN, WAA_INIT_CHAIN,
};
use crate::constants::urls::{BATCHEXECUTE_PATH, GEMINI_BASE_URL, OGADS_BASE_URL, WAA_BASE_URL};
use crate::constants::urls::{
    CONVERSATION_ACTION_SOURCE_PATH_PREFIX, SCHEDULED_SOURCE_PATH, USAGE_SOURCE_PATH,
};
use crate::constants::user_agents::BROWSER_LIKE;
use crate::constants::{rpc_ids, transport};

/// Best-effort default fingerprint used when the live session does not yield
/// one (e.g., ogads init failed). The captured `Pro` model id is reused as a
/// stand-in but may not match the live model selection — see spike findings.
const WAA_FINGERPRINT_DEFAULT: &str = "e6fa609c3fa255c0";
const WAA_API_KEY: &str = "AIzaSyBGb5fGAyC-pRcRU6MUHb__b_vKha71HRE";
const OGADS_API_KEY: &str = "AIzaSyCbsbvGCe7C9mCtdaTycZB2eUFuzsYKG_E";

/// The main entry point for interacting with the Gemini web frontend.
///
/// A `GeminiClient` holds the HTTP client, cookies, and extracted session state.
/// It is cheaply cloneable; clones share the same underlying session.
#[derive(Clone)]
#[non_exhaustive]
pub struct GeminiClient {
    inner: Arc<Inner>,
}

struct Inner {
    http: Client,
    cookies: Mutex<Cookies>,
    session: Mutex<SessionState>,
    config: RwLock<ClientConfig>,
    provider: Mutex<Option<Arc<dyn CredentialsProvider>>>,
    har_writer: Mutex<Option<HarWriter>>,
}

/// Configuration values shared by every request made by a [`GeminiClient`].
#[derive(Clone)]
pub struct ClientConfig {
    language: String,
    max_retries: usize,
    timeout: Duration,
    system_instruction: Option<String>,
    http_hook: Option<Arc<dyn HttpHook>>,
    fatal_hook_errors: bool,
    metrics_recorder: Option<Arc<dyn crate::metrics::MetricsRecorder>>,
    base_url: String,
    har_path: Option<String>,
}

impl std::fmt::Debug for ClientConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientConfig")
            .field("language", &self.language)
            .field("max_retries", &self.max_retries)
            .field("timeout", &self.timeout)
            .field("system_instruction", &self.system_instruction)
            .field("http_hook", &self.http_hook.is_some())
            .field("fatal_hook_errors", &self.fatal_hook_errors)
            .field("base_url", &self.base_url)
            .field("har_path", &self.har_path)
            .finish()
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            max_retries: 3,
            timeout: Duration::from_secs(120),
            system_instruction: None,
            http_hook: None,
            fatal_hook_errors: false,
            metrics_recorder: None,
            base_url: GEMINI_BASE_URL.to_string(),
            har_path: None,
        }
    }
}

impl ClientConfig {
    /// Returns the configured Gemini frontend base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl GeminiClient {
    /// Creates a client from a cookie header string copied from a browser.
    ///
    /// # Errors
    ///
    /// Returns an error if the cookie header is missing required cookies or if
    /// the underlying HTTP client cannot be built.
    pub fn from_cookie_header(header: &str) -> Result<Self> {
        let credentials =
            Credentials::from_header(header).map_err(|e| Error::Config(e.to_string()))?;
        Self::from_credentials(credentials)
    }

    /// Creates a client from typed [`Credentials`].
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn from_credentials(credentials: Credentials) -> Result<Self> {
        Self::with_config(credentials.into(), ClientConfig::default())
    }

    /// Creates a client from a map of cookie names to values.
    ///
    /// # Errors
    ///
    /// Returns an error if the map is missing required cookies or if the
    /// underlying HTTP client cannot be built.
    pub fn from_cookies(cookies: impl Into<Cookies>) -> Result<Self> {
        Self::with_config(cookies.into(), ClientConfig::default())
    }

    /// Creates a client from an externally built [`reqwest::Client`].
    ///
    /// This allows callers to control connection pooling, timeouts, middleware,
    /// and other HTTP client configuration. The provided client is used as-is;
    /// the SDK will not rebuild or reconfigure it.
    ///
    /// # Errors
    ///
    /// Returns an error if the credentials are missing required cookies.
    pub fn from_http_client(
        client: reqwest::Client,
        credentials: impl Into<Cookies>,
    ) -> Result<Self> {
        Self::from_http_client_with_config(client, credentials, ClientConfig::default())
    }

    /// Creates a client from an externally built [`reqwest::Client`] and a
    /// fully populated client configuration.
    ///
    /// This is the most flexible constructor: it lets callers supply a custom
    /// HTTP client, a custom base URL, and every other SDK option in one call.
    /// It is primarily useful for testing against a mock server or proxy, and
    /// for advanced deployment scenarios.
    ///
    /// # Errors
    ///
    /// Returns an error if the credentials are missing required cookies.
    pub fn from_http_client_with_config(
        client: reqwest::Client,
        credentials: impl Into<Cookies>,
        config: ClientConfig,
    ) -> Result<Self> {
        let cookies: Cookies = credentials.into();
        cookies.to_credentials().map_err(|e| Error::Config(e.to_string()))?;
        Self::with_http_client(cookies, config, client)
    }

    /// Creates a client from a [`HashMap`] of cookies.
    ///
    /// # Errors
    ///
    /// Returns an error if the map is missing required cookies or if the
    /// underlying HTTP client cannot be built.
    pub fn from_hashmap(cookies: HashMap<String, String>) -> Result<Self> {
        Self::from_cookies(cookies)
    }

    /// Creates a client from any [`CredentialsProvider`].
    ///
    /// This is the extension point for custom auth sources: implement
    /// [`CredentialsProvider`] to read credentials from environment variables,
    /// files, keyrings, etc., then pass the provider to this constructor.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider cannot produce credentials or if the
    /// underlying HTTP client cannot be built.
    pub async fn from_provider<P>(provider: P) -> Result<Self>
    where
        P: CredentialsProvider + 'static,
    {
        let credentials = provider.credentials().await?;
        let client = Self::from_credentials(credentials)?;
        *client.inner.provider.lock().await = Some(Arc::new(provider));
        Ok(client)
    }

    /// Registers a credentials provider on an existing client.
    ///
    /// The provider is used by [`ChatBuilder::with_refresh_on_auth_error`] to
    /// refresh credentials automatically on `NotSignedIn` errors.
    pub async fn with_provider<P>(self, provider: P) -> Self
    where
        P: CredentialsProvider + 'static,
    {
        *self.inner.provider.lock().await = Some(Arc::new(provider));
        self
    }

    /// Sets the language code sent to the Gemini frontend.
    pub async fn with_language(self, language: impl Into<String>) -> Self {
        let language = language.into();
        let mut config = self.inner.config.write().await;
        config.language.clone_from(&language);
        drop(config);
        self
    }

    /// Sets the maximum number of retries for transient failures.
    pub async fn with_max_retries(self, max_retries: usize) -> Self {
        let mut config = self.inner.config.write().await;
        config.max_retries = max_retries;
        drop(config);
        self
    }

    /// Sets a custom Gemini frontend base URL.
    ///
    /// The default is `https://gemini.google.com`. Override this to point the
    /// client at a mock server, proxy, or regional endpoint.
    pub async fn with_base_url(self, base_url: impl Into<String>) -> Self {
        let base_url = base_url.into();
        let mut config = self.inner.config.write().await;
        config.base_url = base_url;
        drop(config);
        self
    }

    /// Sets the request timeout.
    pub async fn with_timeout(self, timeout: Duration) -> Self {
        let mut config = self.inner.config.write().await;
        config.timeout = timeout;
        drop(config);
        self
    }

    /// Sets a client-level default system instruction applied when no per-turn
    /// instruction is provided.
    pub async fn with_system_instruction(self, instruction: impl Into<String>) -> Self {
        let instruction = instruction.into();
        let mut config = self.inner.config.write().await;
        config.system_instruction = Some(instruction);
        drop(config);
        self
    }

    /// Sets a request/response hook for observing traffic.
    pub async fn with_http_hook(self, hook: impl HttpHook + 'static) -> Self {
        let mut config = self.inner.config.write().await;
        config.http_hook = Some(Arc::new(hook));
        drop(config);
        self
    }

    /// Makes hook errors abort the request instead of being logged and ignored.
    pub async fn with_fatal_hook_errors(self, fatal: bool) -> Self {
        let mut config = self.inner.config.write().await;
        config.fatal_hook_errors = fatal;
        drop(config);
        self
    }

    /// Sets a metrics recorder for observing request, retry, parse, and
    /// attestation boundaries.
    pub async fn with_metrics(
        self,
        recorder: impl crate::metrics::MetricsRecorder + 'static,
    ) -> Self {
        let mut config = self.inner.config.write().await;
        config.metrics_recorder = Some(Arc::new(recorder));
        drop(config);
        self
    }

    /// Enables optional W3C HAR 1.2 capture for every HTTP transaction.
    ///
    /// The file at `path` is created or truncated. Cookies, authorization
    /// headers, and `x-goog-ext-*` values are redacted before writing.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened for writing.
    pub async fn with_har_capture(self, path: impl Into<String>) -> Result<Self> {
        let path = path.into();
        let writer = HarWriter::new(&path).await?;
        {
            let mut config = self.inner.config.write().await;
            config.har_path = Some(path);
            drop(config);
        }
        *self.inner.har_writer.lock().await = Some(writer);
        Ok(self)
    }

    /// Returns the response id of the most recently ingested turn, if any.
    ///
    /// This is a best-effort accessor intended for integration tests and the
    /// live probe example. It returns `None` if no turn has been generated yet
    /// or if the conversation state has been cleared.
    #[must_use]
    pub async fn last_response_id(&self) -> Option<String> {
        self.inner
            .session
            .lock()
            .await
            .conversation_state
            .as_ref()
            .map(|s| s.response_id.clone())
    }

    /// Returns a clone of the cookies used by this client.
    pub async fn cookies(&self) -> Cookies {
        self.inner.cookies.lock().await.clone()
    }

    pub(crate) async fn run_request_hook(&self, request: &PreparedRequest) -> Result<()> {
        let (hook, fatal) = {
            let config = self.inner.config.read().await;
            (config.http_hook.clone(), config.fatal_hook_errors)
        };
        if let Some(hook) = hook {
            if let Err(err) = hook.on_request(request).await {
                if fatal {
                    return Err(err);
                }
                warn!("request hook error (non-fatal): {}", err);
            }
        }
        Ok(())
    }

    pub(crate) async fn run_response_hook(&self, response: &ChatResponse) -> Result<()> {
        let (hook, fatal) = {
            let config = self.inner.config.read().await;
            (config.http_hook.clone(), config.fatal_hook_errors)
        };
        if let Some(hook) = hook {
            if let Err(err) = hook.on_response(response).await {
                if fatal {
                    return Err(err);
                }
                warn!("response hook error (non-fatal): {}", err);
            }
        }
        Ok(())
    }

    fn with_config(cookies: Cookies, config: ClientConfig) -> Result<Self> {
        let http = Client::builder()
            .pool_max_idle_per_host(20)
            .connect_timeout(Duration::from_secs(10))
            .timeout(config.timeout)
            .cookie_store(true)
            .build()
            .map_err(|e| Error::Config(format!("failed to build HTTP client: {e}")))?;

        Self::with_http_client(cookies, config, http)
    }

    /// Returns the currently configured Gemini frontend base URL.
    ///
    /// The default is `https://gemini.google.com`.
    pub async fn base_url(&self) -> String {
        self.inner.config.read().await.base_url.clone()
    }

    fn with_http_client(cookies: Cookies, config: ClientConfig, http: Client) -> Result<Self> {
        let mut session = SessionState::new();
        session.language.clone_from(&config.language);
        session.waa_fingerprint = Some(WAA_FINGERPRINT_DEFAULT.to_string());

        Ok(Self {
            inner: Arc::new(Inner {
                http,
                cookies: Mutex::new(cookies),
                session: Mutex::new(session),
                config: RwLock::new(config),
                provider: Mutex::new(None),
                har_writer: Mutex::new(None),
            }),
        })
    }

    /// Returns a direct reference to the internal session mutex.
    ///
    /// # Safety
    ///
    /// This is intended for integration tests and advanced mocking scenarios
    /// only. Mutating the session can cause requests to fail or behave
    /// unexpectedly.
    pub fn inner_session_for_tests(&self) -> &tokio::sync::Mutex<crate::session::SessionState> {
        &self.inner.session
    }

    pub(crate) async fn build_headers_for_test(
        &self,
        reqid: Option<&str>,
        waa_context: Option<&str>,
        authorization: Option<&str>,
        auth_user: Option<&str>,
        endpoint: Option<&str>,
    ) -> Vec<(String, String)> {
        self.build_headers(reqid, waa_context, authorization, auth_user, endpoint).await
    }

    /// Returns a builder for sending a single chat message.
    pub fn chat(&self) -> ChatBuilder<'_> {
        ChatBuilder {
            client: self,
            conversation: None,
            category: ModelCategory::Auto,
            config: None,
            tools: None,
            refresh_on_auth_error: false,
        }
    }

    /// Returns a builder that continues an existing [`Conversation`].
    pub fn continue_conversation(&self, conversation: Conversation) -> ChatBuilder<'_> {
        let category = conversation.model_category.unwrap_or(ModelCategory::Auto);
        ChatBuilder {
            client: self,
            conversation: Some(conversation),
            category,
            config: None,
            tools: None,
            refresh_on_auth_error: false,
        }
    }

    /// Serialises this client's credentials and session state into a JSON snapshot.
    ///
    /// The snapshot includes the current credentials (with secrets) and session
    /// state. It does **not** include the current conversation; use
    /// [`save_session_with_conversation`][Self::save_session_with_conversation]
    /// to include one.
    ///
    /// # Security
    ///
    /// The returned string contains recoverable credentials. Store it securely
    /// and never log it.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot cannot be serialised to JSON.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn run() -> gemini_sdk::Result<()> {
    /// # let client = gemini_sdk::GeminiClient::from_cookie_header("__Secure-1PSID=a; __Secure-1PSIDCC=b").unwrap();
    /// let snapshot = client.save_session().await?;
    /// let (restored, conversation) = gemini_sdk::GeminiClient::restore_session(&snapshot).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn save_session(&self) -> Result<String> {
        self.save_session_with_conversation_inner(None).await
    }

    /// Serialises this client's credentials, session state, and a conversation
    /// into a JSON snapshot.
    ///
    /// # Security
    ///
    /// The returned string contains recoverable credentials. Store it securely
    /// and never log it.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot cannot be serialised to JSON.
    pub async fn save_session_with_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<String> {
        self.save_session_with_conversation_inner(Some(conversation.clone())).await
    }

    async fn save_session_with_conversation_inner(
        &self,
        conversation: Option<Conversation>,
    ) -> Result<String> {
        let credentials = {
            let cookies = self.inner.cookies.lock().await;
            cookies.to_credentials().map_err(|e| Error::Config(e.to_string()))?
        };
        let session = self.inner.session.lock().await.clone();
        let snapshot = crate::session::Snapshot {
            format_version: crate::session::SNAPSHOT_FORMAT_VERSION,
            credentials,
            session,
            conversation,
        };
        serde_json::to_string(&snapshot).map_err(Error::Json)
    }

    /// Restores a client and optional conversation from a JSON snapshot.
    ///
    /// The restored client uses a fresh [`reqwest::Client`] with default SDK
    /// configuration.
    ///
    /// # Security
    ///
    /// The snapshot string contains recoverable credentials. Only pass snapshots
    /// from trusted storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot is invalid, missing required cookies, or
    /// the HTTP client cannot be built.
    pub async fn restore_session(snapshot: &str) -> Result<(Self, Option<Conversation>)> {
        let parsed: crate::session::Snapshot =
            serde_json::from_str(snapshot).map_err(Error::Json)?;
        let cookies: Cookies = parsed.credentials.into();
        let client = Self::from_cookies(cookies)?;
        {
            let mut session = client.inner.session.lock().await;
            *session = parsed.session;
        }
        Ok((client, parsed.conversation))
    }

    /// Regenerates the model response for a single conversation turn.
    ///
    /// Sends the `PCck7e` batchexecute RPC to `/app/{conversation_id}` with
    /// the response id to regenerate.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(name = REGENERATE_TURN, level = "info", skip_all, fields(response_id = %response_id.as_ref()))]
    pub async fn regenerate_turn(
        &self,
        conversation_id: impl AsRef<str>,
        response_id: impl AsRef<str>,
    ) -> Result<ConversationActionResult> {
        self.conversation_action(
            conversation_id.as_ref(),
            response_id.as_ref(),
            ConversationAction::Regenerate,
            build_regenerate_payload,
        )
        .await
    }

    /// Rates a model response for a single conversation turn.
    ///
    /// Sends the `PCck7e` batchexecute RPC to `/app/{conversation_id}` with
    /// the response id and the selected [`TurnRating`].
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(name = RATE_TURN, level = "info", skip_all, fields(response_id = %response_id.as_ref()))]
    pub async fn rate_turn(
        &self,
        conversation_id: impl AsRef<str>,
        response_id: impl AsRef<str>,
        rating: TurnRating,
    ) -> Result<ConversationActionResult> {
        let response_id = response_id.as_ref();
        self.conversation_action(
            conversation_id.as_ref(),
            response_id,
            ConversationAction::Rate(rating),
            |id| build_rate_payload(id, rating),
        )
        .await
    }

    /// Deletes a single conversation turn.
    ///
    /// Sends the `PCck7e` batchexecute RPC to `/app/{conversation_id}` with
    /// the response id to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(name = DELETE_TURN, level = "info", skip_all, fields(response_id = %response_id.as_ref()))]
    pub async fn delete_turn(
        &self,
        conversation_id: impl AsRef<str>,
        response_id: impl AsRef<str>,
    ) -> Result<ConversationActionResult> {
        self.conversation_action(
            conversation_id.as_ref(),
            response_id.as_ref(),
            ConversationAction::Delete,
            build_delete_payload,
        )
        .await
    }

    /// Sends a `PCck7e` batchexecute request for a conversation action.
    ///
    /// This helper centralises request construction and response parsing for
    /// [`Self::regenerate_turn`], [`Self::rate_turn`], and
    /// [`Self::delete_turn`].
    async fn conversation_action(
        &self,
        conversation_id: &str,
        response_id: &str,
        action: ConversationAction,
        build_payload: impl FnOnce(&str) -> Value,
    ) -> Result<ConversationActionResult> {
        // Avoid re-initialising the session if it already has the values we
        // need, so tests against a mock server can inject session state and
        // skip the live /app init flow.
        let already_initialised = {
            let session = self.inner.session.lock().await;
            session.build_label.is_some() && session.session_id.is_some()
        };
        if !already_initialised {
            self.ensure_session().await?;
        }

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let (params, body, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let source_path = format!("{CONVERSATION_ACTION_SOURCE_PATH_PREFIX}{conversation_id}");
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, PCCK7E_RPC_ID.to_string()),
                (SOURCE_PATH, source_path),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let inner_payload = build_payload(response_id);
            let body = crate::proto::build_batchexecute_body_for_rpc(
                PCCK7E_RPC_ID,
                &serde_json::to_string(&inner_payload).unwrap_or_default(),
                session.access_token.as_deref(),
            );
            let cookie_header = cookies.to_header_value();
            (params, body, cookie_header)
        };
        let headers = self
            .build_headers(None, None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
            .await;

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(PCCK7E_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        parse_conversation_action_response(&response.body, action, response_id.to_string())
    }

    /// Returns the signed-in user's profile information.
    ///
    /// Sends the `o30O0e` batchexecute RPC to `/` and parses the response into
    /// a [`UserInfo`] struct. Missing or null fields are returned as `None`.
    ///
    /// # Security
    ///
    /// The returned values contain PII. Callers must not log them at info level
    /// or expose them in telemetry.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(
        name = GET_USER_INFO,
        level = "info",
        skip_all,
        fields(operation = GET_USER_INFO)
    )]
    pub async fn get_user_info(&self) -> Result<UserInfo> {
        self.ensure_session().await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let (params, body, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, O30O0E_RPC_ID.to_string()),
                (SOURCE_PATH, "/".to_string()),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let inner_payload = build_get_user_info_payload();
            let body = crate::proto::build_batchexecute_body_for_rpc(
                O30O0E_RPC_ID,
                &serde_json::to_string(&inner_payload).unwrap_or_default(),
                session.access_token.as_deref(),
            );
            let cookie_header = cookies.to_header_value();
            (params, body, cookie_header)
        };
        let headers = self
            .build_headers(None, None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
            .await;

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(O30O0E_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        parse_user_info_response(&response.body)
    }

    /// Returns the user's last-selected Gemini mode preference.
    ///
    /// Sends the `L5adhe` batchexecute RPC to `/` and parses the response into
    /// a [`LastSelectedMode`] struct. If no mode has been selected, the
    /// returned `mode_id` is `None`.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(
        name = GET_LAST_SELECTED_MODE,
        level = "info",
        skip_all,
        fields(operation = GET_LAST_SELECTED_MODE)
    )]
    pub async fn get_last_selected_mode(&self) -> Result<LastSelectedMode> {
        self.ensure_session().await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let (params, body, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, L5ADHE_RPC_ID.to_string()),
                (SOURCE_PATH, "/".to_string()),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let inner_payload = build_get_last_selected_mode_payload(None);
            let body = crate::proto::build_batchexecute_body_for_rpc(
                L5ADHE_RPC_ID,
                &serde_json::to_string(&inner_payload).unwrap_or_default(),
                session.access_token.as_deref(),
            );
            let cookie_header = cookies.to_header_value();
            (params, body, cookie_header)
        };
        let headers = self
            .build_headers(None, None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
            .await;

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(L5ADHE_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        parse_last_selected_mode_response(&response.body)
    }

    /// Sets the user's last-selected Gemini mode preference.
    ///
    /// Sends the `L5adhe` batchexecute RPC to `/` with the provided `mode_id`
    /// and returns `Ok(())` on HTTP success. The response body is not parsed.
    ///
    /// # Security
    ///
    /// `mode_id` is treated as an opaque string and passed through JSON
    /// serialization. It is never interpreted as a path or logged at info level.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized or the request fails.
    #[tracing::instrument(
        name = SET_LAST_SELECTED_MODE,
        level = "info",
        skip_all,
        fields(operation = SET_LAST_SELECTED_MODE)
    )]
    pub async fn set_last_selected_mode(&self, mode_id: impl AsRef<str>) -> Result<()> {
        self.ensure_session().await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let mode_id = mode_id.as_ref();
        let (params, body, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, L5ADHE_RPC_ID.to_string()),
                (SOURCE_PATH, "/".to_string()),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let inner_payload = build_set_last_selected_mode_payload(mode_id);
            let body = crate::proto::build_batchexecute_body_for_rpc(
                L5ADHE_RPC_ID,
                &serde_json::to_string(&inner_payload).unwrap_or_default(),
                session.access_token.as_deref(),
            );
            let cookie_header = cookies.to_header_value();
            (params, body, cookie_header)
        };
        let headers = self
            .build_headers(None, None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
            .await;

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(L5ADHE_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        Ok(())
    }

    /// Returns the locale tools configuration.
    ///
    /// Sends the `cYRIkd` batchexecute RPC to `/` and parses the response into
    /// a [`LocaleTools`] wrapper. The inner value is an opaque
    /// [`serde_json::Value`] to tolerate undocumented shape drift.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(
        name = GET_LOCALE_TOOLS,
        level = "info",
        skip_all,
        fields(operation = GET_LOCALE_TOOLS)
    )]
    pub async fn get_locale_tools(&self) -> Result<LocaleTools> {
        self.ensure_session().await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let (params, body, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, CYRIKD_RPC_ID.to_string()),
                (SOURCE_PATH, "/".to_string()),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let inner_payload = build_get_locale_tools_payload(&session.language);
            let body = crate::proto::build_batchexecute_body_for_rpc(
                CYRIKD_RPC_ID,
                &serde_json::to_string(&inner_payload).unwrap_or_default(),
                session.access_token.as_deref(),
            );
            let cookie_header = cookies.to_header_value();
            (params, body, cookie_header)
        };
        let headers = self
            .build_headers(None, None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
            .await;

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(CYRIKD_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        parse_locale_tools_response(&response.body)
    }

    /// Returns the model configuration.
    ///
    /// Sends the `whPPme` batchexecute RPC to `/` and parses the response into
    /// a [`ModelConfig`] wrapper. The inner value is an opaque
    /// [`serde_json::Value`] to tolerate undocumented shape drift.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(
        name = GET_MODEL_CONFIG,
        level = "info",
        skip_all,
        fields(operation = GET_MODEL_CONFIG)
    )]
    pub async fn get_model_config(&self) -> Result<ModelConfig> {
        self.ensure_session().await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let (params, body, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, WHPPME_RPC_ID.to_string()),
                (SOURCE_PATH, "/".to_string()),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let inner_payload = build_get_model_config_payload(&session.language);
            let body = crate::proto::build_batchexecute_body_for_rpc(
                WHPPME_RPC_ID,
                &serde_json::to_string(&inner_payload).unwrap_or_default(),
                session.access_token.as_deref(),
            );
            let cookie_header = cookies.to_header_value();
            (params, body, cookie_header)
        };
        let headers = self
            .build_headers(None, None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
            .await;

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(WHPPME_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        parse_model_config_response(&response.body)
    }

    /// Returns the locale configuration.
    ///
    /// Sends the `Te6DCf` batchexecute RPC to `/` and parses the response into
    /// a [`LocaleConfig`] wrapper. The inner value is an opaque
    /// [`serde_json::Value`] to tolerate undocumented shape drift.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(
        name = GET_LOCALE_CONFIG,
        level = "info",
        skip_all,
        fields(operation = GET_LOCALE_CONFIG)
    )]
    pub async fn get_locale_config(&self) -> Result<LocaleConfig> {
        self.ensure_session().await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let (params, body, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, TE6DCF_RPC_ID.to_string()),
                (SOURCE_PATH, "/".to_string()),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let inner_payload = build_get_locale_config_payload(&session.language);
            let body = crate::proto::build_batchexecute_body_for_rpc(
                TE6DCF_RPC_ID,
                &serde_json::to_string(&inner_payload).unwrap_or_default(),
                session.access_token.as_deref(),
            );
            let cookie_header = cookies.to_header_value();
            (params, body, cookie_header)
        };
        let headers = self
            .build_headers(None, None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
            .await;

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(TE6DCF_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        parse_locale_config_response(&response.body)
    }

    /// Returns the tools configuration.
    ///
    /// Sends the `ku4Jyf` batchexecute RPC to `/` and parses the response into
    /// a [`ToolsConfig`] wrapper. The inner value is an opaque
    /// [`serde_json::Value`] to tolerate undocumented shape drift.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(
        name = GET_TOOLS_CONFIG,
        level = "info",
        skip_all,
        fields(operation = GET_TOOLS_CONFIG)
    )]
    pub async fn get_tools_config(&self) -> Result<ToolsConfig> {
        self.ensure_session().await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let (params, body, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, KU4JYF_RPC_ID.to_string()),
                (SOURCE_PATH, "/".to_string()),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let inner_payload = build_get_tools_config_payload(&session.language);
            let body = crate::proto::build_batchexecute_body_for_rpc(
                KU4JYF_RPC_ID,
                &serde_json::to_string(&inner_payload).unwrap_or_default(),
                session.access_token.as_deref(),
            );
            let cookie_header = cookies.to_header_value();
            (params, body, cookie_header)
        };
        let headers = self
            .build_headers(None, None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
            .await;

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(KU4JYF_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        parse_tools_config_response(&response.body)
    }

    /// Returns usage statistics for the signed-in account.
    ///
    /// Sends the `jSf9Qc` batchexecute RPC to `/usage` and parses the response
    /// into a [`UsageStats`] wrapper. The inner value is an opaque
    /// [`serde_json::Value`] to tolerate undocumented shape drift.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(
        name = GET_USAGE_STATS,
        level = "info",
        skip_all,
        fields(operation = GET_USAGE_STATS)
    )]
    pub async fn get_usage_stats(&self) -> Result<UsageStats> {
        self.ensure_session().await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let auth = credentials_to_sapisid_hash(&cookies, &base_url);
        let (params, body, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, JSF9QC_RPC_ID.to_string()),
                (SOURCE_PATH, USAGE_SOURCE_PATH.to_string()),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let inner_payload = build_get_usage_stats_payload();
            let body = crate::proto::build_batchexecute_body_for_rpc(
                JSF9QC_RPC_ID,
                &serde_json::to_string(&inner_payload).unwrap_or_default(),
                session.access_token.as_deref(),
            );
            let cookie_header = cookies.to_header_value();
            (params, body, cookie_header)
        };
        let headers = self
            .build_headers(
                None,
                None,
                auth.as_deref(),
                Some(crate::constants::headers::X_GOOG_AUTHUSER_VALUE),
                Some(transport::BATCHEXECUTE_ENDPOINT),
            )
            .await;

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(JSF9QC_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        parse_usage_stats_response(&response.body)
    }

    /// Returns the user's scheduled prompts.
    ///
    /// Sends the `XPSWpd` batchexecute RPC to `/scheduled` and parses the
    /// response into a [`ScheduledPrompts`] wrapper. The inner value is an
    /// opaque [`serde_json::Value`] to tolerate undocumented shape drift.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not initialized, the request fails,
    /// or the response cannot be parsed.
    #[tracing::instrument(
        name = GET_SCHEDULED_PROMPTS,
        level = "info",
        skip_all,
        fields(operation = GET_SCHEDULED_PROMPTS)
    )]
    pub async fn get_scheduled_prompts(&self) -> Result<ScheduledPrompts> {
        self.ensure_session().await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let (params, body, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, XPSWPD_RPC_ID.to_string()),
                (SOURCE_PATH, SCHEDULED_SOURCE_PATH.to_string()),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let inner_payload = build_get_scheduled_prompts_payload();
            let body = crate::proto::build_batchexecute_body_for_rpc(
                XPSWPD_RPC_ID,
                &serde_json::to_string(&inner_payload).unwrap_or_default(),
                session.access_token.as_deref(),
            );
            let cookie_header = cookies.to_header_value();
            (params, body, cookie_header)
        };
        let headers = self
            .build_headers(None, None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
            .await;

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(XPSWPD_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        parse_scheduled_prompts_response(&response.body)
    }

    /// Refreshes credentials from a provider and re-initializes the session.
    ///
    /// This replaces the stored cookies, clears session state, and runs the
    /// `/app` init flow (including consent acquisition if required).
    pub async fn refresh_credentials<P: CredentialsProvider>(&self, provider: P) -> Result<()> {
        let credentials = provider.credentials().await?;
        let cookies: Cookies = credentials.into();
        {
            let mut guard = self.inner.cookies.lock().await;
            *guard = cookies;
        }
        {
            let language = self.inner.config.read().await.language.clone();
            let mut session = self.inner.session.lock().await;
            *session = SessionState::new();
            session.language = language;
        }
        self.init_session().await
    }

    /// Lists the models available to the signed-in account.
    ///
    /// Internally calls `BardFrontendService.GetUserStatus` through the
    /// batchexecute transport using the `otAQ7b` RPC id.
    #[tracing::instrument(
        name = LIST_MODELS,
        level = "info",
        skip_all,
        fields(operation = LIST_MODELS)
    )]
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.ensure_session().await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let cookies = self.cookies().await;
        let (params, body, headers, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid(None);
            let mut params: Vec<(&str, String)> = vec![
                (RPCIDS, rpc_ids::OTAQ7B_RPC_ID.to_string()),
                (SOURCE_PATH, crate::constants::urls::APP_PATH.to_string()),
                (HL, session.language.clone()),
                (REQID, reqid),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            let body = build_batchexecute_body(session.access_token.as_deref());
            let waa_context = session.waa_context.clone();
            let headers = self
                .build_headers(
                    None,
                    waa_context.as_deref(),
                    None,
                    None,
                    Some(transport::BATCHEXECUTE_ENDPOINT),
                )
                .await;
            let cookie_header = cookies.to_header_value();
            (params, body, headers, cookie_header)
        };

        let response = self
            .send_batchexecute_with_retry(
                || async { SessionState::generate_reqid(Some(rpc_ids::OTAQ7B_RPC_ID)) },
                || {
                    let client = self.inner.http.clone();
                    let url = url.clone();
                    let params = params.clone();
                    let body = body.clone();
                    let headers = headers.clone();
                    let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(body.clone());
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                },
            )
            .await?;

        if !response.status.is_success() {
            return Err(Error::api(response.status, response.body));
        }

        parse_model_list(&response.body)
    }

    /// Sends a generation request and returns the parsed response.
    ///
    /// Prefer using [`GeminiClient::chat`] for an ergonomic API. If you need to
    /// pass an existing [`Conversation`] use [`GeminiClient::generate_with_conversation`].
    #[tracing::instrument(name = GENERATE, level = "info", skip_all, fields(operation = GENERATE, category = ?category))]
    pub async fn generate(
        &self,
        message: &ChatMessage,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<ChatResponse> {
        self.generate_with_conversation(message, None, category, config).await
    }

    /// Sends a generation request with tool declarations, invokes any tool
    /// calls returned by the model, and returns the final parsed response.
    ///
    /// This method performs a round-trip: it sends the prompt with tool
    /// declarations, parses [`ContentPart::ToolCall`] parts from the response,
    /// invokes the matching registered tools, sends a follow-up turn containing
    /// the results, and repeats up to `max_tool_turns` (default 5).
    #[tracing::instrument(name = GENERATE_WITH_TOOLS, level = "info", skip_all, fields(operation = GENERATE_WITH_TOOLS, category = ?category))]
    pub async fn generate_with_tools(
        &self,
        message: &ChatMessage,
        tools: Vec<Arc<dyn Tool>>,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<ChatResponse> {
        let (mut config, _) = self.resolve_config(config).await?;
        let declarations: Vec<serde_json::Value> = tools
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name(),
                    "parameters": tool.schema(),
                })
            })
            .collect();
        if let Some(ref mut cfg) = config {
            cfg.tools = Some(declarations);
        } else {
            config = Some(GenerationConfig::default().with_tools(declarations));
        }

        let max_turns = config.as_ref().and_then(|c| c.max_tool_turns).unwrap_or(5);

        let mut current_message = message.clone();
        let mut last_response = None;

        for turn in 0..max_turns {
            let prepared = prepare_request(None, &current_message, config.clone(), category)?;
            self.run_request_hook(&prepared).await?;
            let body = self.generate_raw_with_prepared(&prepared).await?;
            let response = self.parse_response_with_conversation_id(&body)?;
            self.run_response_hook(&response).await?;

            let parsed_parts =
                crate::proto::parser::parse_response_parts(&body).unwrap_or_default();
            let mut tool_calls = Vec::new();
            for part in &parsed_parts {
                if let ContentPart::ToolCall(call) = part {
                    tool_calls.push(call.clone());
                }
            }

            if tool_calls.is_empty() {
                return Ok(response);
            }

            last_response = Some(response);

            let mut tool_results = Vec::new();
            for call in tool_calls {
                let tool = tools
                    .iter()
                    .find(|t| t.name() == call.name)
                    .ok_or_else(|| Error::Tool(ToolError::NotFound(call.name.clone())))?;
                let result = tool.invoke(call.args).await.map_err(Error::Tool)?;
                tool_results.push(ToolResult::new(call.name, result));
            }

            let mut parts: Vec<ContentPart> = vec![ContentPart::Text("".to_string())];
            parts.extend(tool_results.into_iter().map(ContentPart::ToolResult));
            current_message = ChatMessage {
                role: "user".to_string(),
                parts,
            };

            // Prevent re-emitting tool declarations on follow-up turns.
            if turn == 0 {
                if let Some(ref mut cfg) = config {
                    cfg.tools = None;
                }
            }
        }

        last_response.ok_or_else(|| {
            Error::Tool(ToolError::InvokeFailed(
                "reached maximum tool-call turns without a final response".to_string(),
            ))
        })
    }

    /// Returns a stream of incremental `ChatResponse` chunks.
    ///
    /// The stream parses each line-delimited WIZ frame as it arrives and yields
    /// a [`ChatResponse`] built from the accumulated text and thinking content
    /// seen so far. After the upstream stream ends, the full response body is
    /// ingested into the client's conversation state so that multi-turn chats
    /// can continue.
    #[tracing::instrument(name = GENERATE_STREAM, level = "info", skip_all, fields(operation = GENERATE_STREAM, category = ?category))]
    pub async fn generate_stream(
        &self,
        message: &ChatMessage,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatResponse>> + Send>>> {
        let response = self.stream_generate_raw(message, None, category, config).await?;
        let client = self.clone();
        Ok(Self::stream_responses(response.bytes_stream(), client))
    }

    fn stream_responses<S>(
        byte_stream: S,
        client: GeminiClient,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatResponse>> + Send>>
    where
        S: Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>>
            + Send
            + Unpin
            + 'static,
    {
        use async_stream::try_stream;
        use futures::StreamExt;

        Box::pin(try_stream! {
            let mut bytes_stream = byte_stream;
            let mut line_buffer = String::new();
            let mut full_body = String::new();

            while let Some(chunk) = bytes_stream.next().await {
                let chunk = chunk.map_err(Error::Request)?;
                let text = String::from_utf8_lossy(&chunk);
                line_buffer.push_str(&text);
                full_body.push_str(&text);

                let mut remaining = String::new();
                let mut split = line_buffer.split('\n').peekable();
                while let Some(line) = split.next() {
                    if split.peek().is_none() {
                        remaining.push_str(line);
                        break;
                    }
                    if !line.trim().is_empty() {
                        if let Ok(parts) = parse_response_parts(line) {
                            if !parts.is_empty() {
                                let mut response = build_chat_response_from_parts(&parts)?;
                                if let Some(id) = Self::extract_conversation_id_from_body(&full_body) {
                                    response = response.with_conversation_id(id);
                                }
                                client.run_response_hook(&response).await?;
                                yield response;
                            }
                        }
                    }
                }
                line_buffer = remaining;
            }

            if !line_buffer.trim().is_empty() {
                if let Ok(parts) = parse_response_parts(&line_buffer) {
                    if !parts.is_empty() {
                        let mut response = build_chat_response_from_parts(&parts)?;
                        if let Some(id) = Self::extract_conversation_id_from_body(&full_body) {
                            response = response.with_conversation_id(id);
                        }
                        client.run_response_hook(&response).await?;
                        yield response;
                    }
                }
            }

            client.ingest_conversation_state(&full_body).await?;
        })
    }

    fn extract_conversation_id_from_body(body: &str) -> Option<String> {
        crate::proto::parser::extract_conversation_id(body)
    }

    /// Sends a generation request with optional conversation state and returns
    /// the parsed response.
    ///
    /// This is the public entry point for callers that manage a
    /// [`Conversation`] manually instead of using the builder API.
    #[tracing::instrument(name = GENERATE_WITH_CONVERSATION, level = "info", skip_all, fields(operation = GENERATE_WITH_CONVERSATION, category = ?category))]
    pub async fn generate_with_conversation(
        &self,
        message: &ChatMessage,
        conversation: Option<&Conversation>,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<ChatResponse> {
        let (config, refresh_on_auth_error) = self.resolve_config(config).await?;
        let prepared = prepare_request(conversation, message, config, category)?;
        let prepared = PreparedRequest {
            refresh_on_auth_error,
            ..prepared
        };
        self.execute_generate(prepared).await
    }

    async fn resolve_config(
        &self,
        config: Option<GenerationConfig>,
    ) -> Result<(Option<GenerationConfig>, bool)> {
        let config =
            if config.is_some() {
                config
            } else {
                self.inner.config.read().await.system_instruction.clone().map(|instruction| {
                    GenerationConfig::default().with_system_instruction(instruction)
                })
            };
        Ok((config, false))
    }

    async fn execute_generate(&self, prepared: PreparedRequest) -> Result<ChatResponse> {
        let refresh_on_auth_error = prepared.refresh_on_auth_error;
        self.run_request_hook(&prepared).await?;
        match self.generate_raw_with_prepared(&prepared).await {
            Ok(body) => {
                let response = self.parse_response_with_conversation_id(&body)?;
                self.run_response_hook(&response).await?;
                Ok(response)
            }
            Err(Error::NotSignedIn(_)) if refresh_on_auth_error => {
                if let Some(provider) = self.inner.provider.lock().await.clone() {
                    self.refresh_credentials(provider).await?;
                    let body = self.generate_raw_with_prepared(&prepared).await?;
                    let response = self.parse_response_with_conversation_id(&body)?;
                    self.run_response_hook(&response).await?;

                    return Ok(response);
                }
                Err(Error::NotSignedIn(
                    "refresh_on_auth_error enabled but no provider registered".to_string(),
                ))
            }
            Err(other) => Err(other),
        }
    }
    /// Sends an already prepared request and returns the raw response body.
    async fn generate_raw_with_prepared(&self, prepared: &PreparedRequest) -> Result<String> {
        let mut response = self.stream_generate_raw_with_prepared(prepared).await?;

        let mut body_bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(Error::Request)? {
            body_bytes.extend_from_slice(&chunk);
        }
        let body = String::from_utf8(body_bytes)
            .map_err(|e| Error::Parse(format!("invalid UTF-8 in response: {e}")))?;

        self.ingest_conversation_state(&body).await?;
        Ok(body)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(operation = PARSE_RESPONSE, bytes = body.len()))]
    fn parse_response(&self, body: &str) -> Result<ChatResponse> {
        parse_chat_response(body)
    }

    fn parse_response_with_conversation_id(&self, body: &str) -> Result<ChatResponse> {
        let mut response = parse_chat_response(body)?;
        if let Some(id) = Self::extract_conversation_id_from_body(body) {
            response = response.with_conversation_id(id);
        }
        Ok(response)
    }

    /// Sends a generation request and returns the raw response body.
    ///
    /// This is useful when implementing custom streaming or logging.
    #[tracing::instrument(level = "debug", skip_all, fields(operation = GENERATE_RAW, category = ?category))]
    pub async fn generate_raw(
        &self,
        message: &ChatMessage,
        conversation: Option<&Conversation>,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<String> {
        let prepared = prepare_request(conversation, message, config, category)?;
        self.generate_raw_with_prepared(&prepared).await
    }

    /// Starts a streaming generation request and returns the upstream response.
    ///
    /// The returned [`reqwest::Response`] can be consumed as a stream of bytes;
    /// callers are responsible for parsing the WIZ frames.
    pub async fn stream_generate(
        &self,
        message: &ChatMessage,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<reqwest::Response> {
        self.stream_generate_raw(message, None, category, config).await
    }

    /// Returns a stream of upload progress events for a single file.
    ///
    /// The stream yields [`UploadEvent::Progress`] at least once and a final
    /// [`UploadEvent::Complete`] when the upload finishes. Dropping the stream
    /// before `Complete` leaves server-side upload state best-effort.
    #[tracing::instrument(name = UPLOAD_WITH_PROGRESS, level = "info", skip_all, fields(operation = UPLOAD_WITH_PROGRESS, bytes = bytes.len()))]
    pub async fn upload_with_progress(
        &self,
        filename: impl Into<String>,
        mime_type: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Pin<Box<dyn Stream<Item = Result<UploadEvent>> + Send + 'static>> {
        let base_url = self.inner.config.read().await.base_url.clone();
        upload::upload_progress_stream(
            self.inner.http.clone(),
            self.cookies().await,
            self.inner.session.lock().await.clone(),
            filename.into(),
            mime_type.into(),
            bytes,
            base_url,
        )
    }

    /// Starts a streaming generation request and returns raw bytes.
    ///
    /// This lower-level method gives callers direct access to the upstream WIZ
    /// byte stream. After the stream is consumed, callers should use
    /// [`GeminiClient::ingest_conversation_state`] to persist state, or call
    /// [`GeminiClient::generate_raw`] which does both.
    pub async fn stream_generate_raw(
        &self,
        message: &ChatMessage,
        conversation: Option<&Conversation>,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<reqwest::Response> {
        let prepared = prepare_request(conversation, message, config, category)?;
        self.run_request_hook(&prepared).await?;
        self.stream_generate_raw_with_prepared(&prepared).await
    }

    async fn stream_generate_raw_with_prepared(
        &self,
        prepared: &PreparedRequest,
    ) -> Result<reqwest::Response> {
        self.ensure_session().await?;

        let (inner_req_list, request_uuid, _headers, cookie_header) =
            self.build_stream_generate_request(prepared).await?;

        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!(
            "{base_url}/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate"
        );

        let (params, at, waa_context, waa_fingerprint) = {
            let session = self.inner.session.lock().await;
            let mut params: Vec<(&str, String)> = vec![
                (HL, session.language.clone()),
                (REQID, request_uuid.clone()),
                (RT, RT_VALUE.to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push((BL, bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push((F_SID, sid.to_string()));
            }
            (
                params,
                session.access_token.clone(),
                session.waa_context.clone(),
                session.waa_fingerprint.clone(),
            )
        };

        let form_body = build_stream_generate_body(&inner_req_list, at.as_deref());
        let waa_header = build_waa_context_header(
            waa_fingerprint.as_deref(),
            waa_context.as_deref(),
            &request_uuid,
        );
        let headers = self
            .build_headers(Some(&request_uuid), Some(&waa_header), None, None, Some("stream_generate"))
            .await;

        let response = self
            .send_with_retry(|| {
                let client = self.inner.http.clone();
                let url = url.clone();
                let params = params.clone();
                let form_body = form_body.clone();
                let headers = headers.clone();
                let cookie_header = cookie_header.clone();
                    async move {
                        let mut req = client.post(&url).query(&params).body(form_body);
                        for (key, value) in &headers {
                            req = req.header(key, value);
                        }
                        req = req.header(header_constants::COOKIE, cookie_header);
                        req.send().await
                    }
                })

            .await?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Error::api(status, text));
        }

        Ok(response)
    }

    /// Extracts multi-turn conversation state from a fully consumed
    /// `stream_generate_raw` body and stores it in the session.
    ///
    /// This helper is intended for callers that consume the byte stream
    /// themselves; [`GeminiClient::generate_raw`] calls it automatically.
    ///
    /// # Errors
    ///
    /// Returns an error if the response body cannot be parsed for conversation
    /// state. On success the session state is updated; on error it is left
    /// unchanged so the next turn does not send corrupt state upstream.
    pub async fn ingest_conversation_state(&self, body: &str) -> Result<()> {
        let state = extract_conversation_state(body)?;
        let mut session = self.inner.session.lock().await;
        session.conversation_state = Some(map_state(state));
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(operation = INGEST_CONVERSATION_STATE, bytes = body.len()))]
    async fn ingest_conversation_state_inner(&self, body: &str) -> Result<()> {
        self.ingest_conversation_state(body).await
    }

    async fn build_stream_generate_request(
        &self,
        prepared: &PreparedRequest,
    ) -> Result<(Vec<Value>, String, Vec<(String, String)>, String)> {
        let request_uuid = fresh_request_uuid();
        let cookies = self.cookies().await;
        let (conversation_state, session_for_upload, language, waa_token, nonce) = {
            let mut session = self.inner.session.lock().await;
            session.nonce = Some(crate::proto::fresh_request_nonce());
            (
                session.conversation_state.clone(),
                session.clone(),
                session.language.clone(),
                session.waa_token.clone(),
                session.take_nonce(),
            )
        };

        let base_url = self.inner.config.read().await.base_url.clone();
        let attachments = upload::upload_attachments(
            &self.inner.http,
            &cookies,
            &session_for_upload,
            prepared,
            &base_url,
        )
        .await?;

        let proto_state = conversation_state.as_ref().map(map_proto_state);
        let inner_req_list = build_inner_req_list(
            prepared,
            proto_state.as_ref(),
            None,
            &attachments,
            &request_uuid,
            &language,
            waa_token.as_deref(),
            &nonce,
        );

        let waa_header = build_waa_context_header(
            session_for_upload.waa_fingerprint.as_deref(),
            session_for_upload.waa_context.as_deref(),
            &request_uuid,
        );
        let headers = self
            .build_headers(Some(&request_uuid), Some(&waa_header), None, None, Some("stream_generate"))
            .await;
        let cookie_header = cookies.to_header_value();

        Ok((inner_req_list, request_uuid, headers, cookie_header))
    }

    /// Ensures that session parameters have been extracted from `/app`.
    async fn ensure_session(&self) -> Result<()> {
        let needs_init = self.inner.session.lock().await.needs_init();
        if needs_init {
            self.init_session().await?;
        }
        Ok(())
    }

    /// Verifies that the stored cookies are accepted by Gemini as a signed-in
    /// session.
    ///
    /// Performs a `GET /app?hl={language}` with cookies, follows redirects, and
    /// inspects the returned HTML. Returns `true` only if the page is not a
    /// sign-in redirect and contains `window.WIZ_global_data` with a non-empty
    /// numeric `S06Grb` Gaia id and a present `oPEP7c` email address.
    #[tracing::instrument(
        name = VERIFY_SIGNED_IN,
        level = "info",
        skip_all,
        fields(operation = VERIFY_SIGNED_IN)
    )]
    pub async fn verify_signed_in(&self) -> Result<bool> {
        let body = self.fetch_app_page().await?;
        Ok(crate::session::looks_like_signed_in_html(&body))
    }

    /// Fetches `/app` and returns a diagnostic result describing whether the
    /// cookies were accepted as a signed-in session.
    ///
    /// On success the returned [`AppDiagnostics`] includes the extracted
    /// `S06Grb` Gaia id and `oPEP7c` email address. On failure it includes the
    /// reason the HTML was rejected and the list of legacy cookies that were
    /// absent from the supplied header. This is useful for the live probe and
    /// integration tests when debugging cookie issues.
    #[tracing::instrument(
        name = DIAGNOSE_SIGNED_IN,
        level = "info",
        skip_all,
        fields(operation = DIAGNOSE_SIGNED_IN)
    )]
    pub async fn diagnose_signed_in(&self) -> Result<AppDiagnostics> {
        let body = self.fetch_app_page().await?;
        let (gaia_id, email) = match extract_signed_in_state(&body) {
            Some(state) => state,
            None => {
                let reason = diagnose_signed_in_state(&body)
                    .err()
                    .map(|f| f.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let missing_legacy = {
                    let cookies = self.cookies().await;
                    cookies.to_credentials().map(|c| c.missing_legacy_cookies()).unwrap_or_default()
                };
                // Modern sessions may no longer expose S06Grb/oPEP7c even when
                // the cookies are accepted. If the page otherwise looks like a
                // valid app session, report signed-in via the fallback.
                let signed_in = crate::session::looks_like_app_session_html(&body);
                return Ok(AppDiagnostics {
                    signed_in,
                    gaia_id: None,
                    email: None,
                    failure_reason: if signed_in { None } else { Some(reason) },
                    missing_legacy_cookies: missing_legacy,
                });
            }
        };
        Ok(AppDiagnostics {
            signed_in: true,
            gaia_id: Some(gaia_id),
            email: Some(email),
            failure_reason: None,
            missing_legacy_cookies: Vec::new(),
        })
    }

    /// Returns the async token (`SNlM0e`) extracted during session init, if any.
    ///
    /// This is useful for CLI diagnostics: an authenticated session can still
    /// miss the async token required for some batchexecute RPCs when the cookie
    /// set is incomplete.
    #[must_use]
    pub async fn access_token(&self) -> Option<String> {
        self.inner.session.lock().await.access_token.clone()
    }

    async fn init_session(&self) -> Result<()> {
        let body = self.fetch_app_page().await?;

        if !crate::session::looks_like_signed_in_html(&body) {
            let reason = diagnose_signed_in_state(&body)
                .err()
                .map(|f| f.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let missing_legacy = {
                let cookies = self.cookies().await;
                cookies.to_credentials().map(|c| c.missing_legacy_cookies()).unwrap_or_default()
            };
            let mut message = format!(
                "cookies rejected by Gemini /app ({reason}); page did not contain signed-in markers"
            );
            if !missing_legacy.is_empty() {
                message.push_str(". Likely missing legacy cookies: ");
                message.push_str(&missing_legacy.join(", "));
                message.push_str(". Copy the full signed-in cookie header from the browser, including SID, HSID, SSID, APISID, SAPISID, SIDCC, __Secure-ENID, and NID.");
            }
            return Err(Error::not_signed_in(message));
        }

        let final_body = if let Some(save_url) = extract_consent_save_url(&body) {
            debug!("consent banner detected; acquiring SOCS cookie");
            self.accept_consent_and_refresh(&save_url).await?
        } else {
            body
        };

        let extracted = extract_from_app_html(&final_body);
        {
            let mut session = self.inner.session.lock().await;
            session.access_token = extracted.access_token.or_else(|| session.access_token.clone());
            session.build_label = extracted.build_label.or_else(|| session.build_label.clone());
            session.session_id = extracted.session_id.or_else(|| session.session_id.clone());
            session.push_id = extracted.push_id.or_else(|| session.push_id.clone());
        }

        // The async token (`SNlM0e`) required for `at=` query parameters is
        // sometimes present only on the root `/` page, not on `/app`. Fall back
        // to fetching `/` and merging any missing session fields.
        let needs_root_fallback = self.inner.session.lock().await.access_token.is_none();
        if needs_root_fallback {
            debug!("access token missing from /app; fetching root page as fallback");
            match self.fetch_root_page().await {
                Ok(root_body) => {
                    let root_extracted = extract_from_app_html(&root_body);
                    let mut session = self.inner.session.lock().await;
                    session.access_token = root_extracted
                        .access_token
                        .or_else(|| session.access_token.clone());
                    session.build_label = root_extracted
                        .build_label
                        .or_else(|| session.build_label.clone());
                    session.session_id = root_extracted
                        .session_id
                        .or_else(|| session.session_id.clone());
                    session.push_id = root_extracted.push_id.or_else(|| session.push_id.clone());
                }
                Err(e) => {
                    debug!(error = %e, "root page fallback failed; continuing without async token");
                }
            }
        }

        // Run the WAA / warm-up chain. WAA Create failures are surfaced as
        // AttestationFailed; batchexecute warm-up failures are tolerated so the
        // session can still be used without attestation context.
        self.run_waa_init_chain().await?;

        Ok(())
    }

    /// Performs the warm-up/WAA RPC chain captured from the Gemini frontend.
    ///
    /// Stores the resulting WAA token and `x-goog-ext-525001261-jspb` context in
    /// the session state. Failures from the WAA `Create` step are surfaced as
    /// [`Error::AttestationFailed`]. Failures from the batchexecute warm-up
    /// RPCs and ogads `GetAsyncData` are tolerated and fall back to a synthetic
    /// context so the session can still be used.
    #[tracing::instrument(level = "info", skip_all, fields(operation = WAA_INIT_CHAIN))]
    async fn run_waa_init_chain(&self) -> Result<()> {
        let (at, language, build_label, session_id, cookie_header, credentials) = {
            let (cookie_header, credentials) = {
                let cookies = self.cookies().await;
                (cookies.to_header_value(), cookies.clone())
            };
            let session = self.inner.session.lock().await;
            (
                session.access_token.clone(),
                session.language.clone(),
                session.build_label.clone(),
                session.session_id.clone(),
                cookie_header,
                credentials,
            )
        };

        // 1. otAQ7b warm-up / model list.
        let models_response = self
            .batchexecute_rpc(
                rpc_ids::OTAQ7B_RPC_ID,
                build_batchexecute_body(at.as_deref()),
                &language,
                build_label.as_deref(),
                session_id.as_deref(),
                &cookie_header,
                Some(crate::constants::urls::DEFAULT_SOURCE_PATH),
            )
            .await
            .unwrap_or_default();
        let fingerprint = extract_waa_fingerprint_from_model_list(&models_response);
        let default_fingerprint = self.inner.session.lock().await.waa_fingerprint.clone();

        // 2. sJBwce [[1,2]] prerequisite.
        if let Err(e) = self
            .batchexecute_rpc(
                "sJBwce",
                build_sjbwce_body(at.as_deref()),
                &language,
                build_label.as_deref(),
                session_id.as_deref(),
                &cookie_header,
                Some(crate::constants::urls::DEFAULT_SOURCE_PATH),
            )
            .await
        {
            debug!(error = %e, "sJBwce warm-up failed; continuing without WAA context");
        }

        // 3. WAA Create.
        let waa_token =
            self.waa_create(&cookie_header).await.map_err(|e| Error::AttestationFailed {
                reason: format!("WAA Create failed: {e}"),
            })?;

        // 4. ogads GetAsyncData.
        let waa_context = self
            .ogads_get_async_data(&cookie_header, &credentials, &waa_token)
            .await
            .unwrap_or_else(|e| {
                debug!(error = %e, "ogads GetAsyncData failed; using synthetic WAA context");
                build_default_waa_context()
            });

        // 5. ESY5D feature flags.
        if let Err(e) = self
            .batchexecute_rpc(
                "ESY5D",
                build_esy5d_body(at.as_deref()),
                &language,
                build_label.as_deref(),
                session_id.as_deref(),
                &cookie_header,
                None,
            )
            .await
        {
            debug!(error = %e, "ESY5D feature flags warm-up failed; continuing without WAA context");
        }

        {
            let mut session = self.inner.session.lock().await;
            session.waa_token = Some(waa_token);
            session.waa_context = Some(waa_context);
            session.waa_fingerprint = fingerprint.or(default_fingerprint);
        }

        Ok(())
    }

    // REASON: internal batchexecute RPC helper mirrors the Google endpoint's
    // many query parameters; no sensible grouping exists.
    #[allow(clippy::too_many_arguments)]
    async fn batchexecute_rpc(
        &self,
        rpcids: &str,
        body: String,
        language: &str,
        build_label: Option<&str>,
        session_id: Option<&str>,
        cookie_header: &str,
        source_path_override: Option<&str>,
    ) -> Result<String> {
        let base_url = self.inner.config.read().await.base_url.clone();
        let url = format!("{base_url}{BATCHEXECUTE_PATH}");
        let reqid = SessionState::generate_reqid(None);
        let mut params: Vec<(&str, String)> = vec![
            (RPCIDS, rpcids.to_string()),
            (SOURCE_PATH, source_path_override.unwrap_or(crate::constants::urls::APP_PATH).to_string()),
            (HL, language.to_string()),
            (REQID, reqid),
            (RT, RT_VALUE.to_string()),
        ];
        if let Some(bl) = build_label {
            params.push((BL, bl.to_string()));
        }
        if let Some(sid) = session_id {
            params.push((F_SID, sid.to_string()));
        }

        let headers = self
            .build_headers(None, None, None, None, Some(transport::BATCHEXECUTE_ENDPOINT))
            .await;
        let response = self
            .send_with_retry(|| {
                let client = self.inner.http.clone();
                let url = url.clone();
                let params = params.clone();
                let body = body.clone();
                let headers = headers.clone();
                let cookie_header = cookie_header.to_string();
                async move {
                    let mut req = client.post(&url).query(&params).body(body);
                    for (key, value) in &headers {
                        req = req.header(key, value);
                    }
                    req = req.header(header_constants::COOKIE, cookie_header);
                    req.send().await
                }
            })
            .await?;

        let status = response.status();
        let text = response.text().await.map_err(Error::Request)?;
        if !status.is_success() {
            return Err(Error::api(status, text));
        }
        Ok(text)
    }

    async fn waa_create(&self, cookie_header: &str) -> Result<String> {
        let url = format!("{WAA_BASE_URL}/$rpc/google.internal.waa.v1.Waa/Create");
        let base_url = self.inner.config.read().await.base_url.clone();
        let body = build_waa_create_body();
        let response = self
            .inner
            .http
            .post(&url)
            .header(header_constants::CONTENT_TYPE, crate::constants::mime::JSON_PROTOBUF)
            .header(header_constants::X_GOOG_API_KEY, WAA_API_KEY)
            .header(header_constants::X_USER_AGENT, crate::constants::headers::X_USER_AGENT_VALUE)
            .header(header_constants::COOKIE, cookie_header)
            .header(header_constants::USER_AGENT, BROWSER_LIKE)
            .header(header_constants::REFERER, format!("{base_url}/"))
            .header(header_constants::ORIGIN, base_url.clone())
            .header(header_constants::X_CLIENT_DATA, crate::constants::headers::X_CLIENT_DATA_VALUE)
            .body(body.clone())
            .send()
            .await
            .map_err(|e| Error::Transient(format!("WAA Create request failed: {e}")))?;

        let status = response.status();
        let response_headers = response.headers().clone();
        let text = response.text().await.map_err(Error::Request)?;
        self.maybe_record_har(
            POST,
            &url,
            &HeaderMap::new(),
            body.as_bytes(),
            status.as_u16(),
            &response_headers,
            text.as_bytes(),
            Duration::from_secs(0),
        )
        .await?;
        if !status.is_success() {
            return Err(Error::api(status, text));
        }

        // Response is a JSON array; the token is typically the first/only string.
        let parsed: Value = serde_json::from_str(&text).map_err(Error::Json)?;
        let token = parsed
            .get(0)
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.get(4))
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
            .or_else(|| {
                parsed.get(0).and_then(|v| v.as_str()).map(std::string::ToString::to_string)
            })
            .ok_or_else(|| Error::parse("WAA Create response missing token"))?;
        Ok(token)
    }

    async fn ogads_get_async_data(
        &self,
        cookie_header: &str,
        credentials: &Cookies,
        waa_token: &str,
    ) -> Result<String> {
        let url = format!("{OGADS_BASE_URL}/$rpc/google.internal.onegoogle.asyncdata.v1.AsyncDataService/GetAsyncData");
        let base_url = self.inner.config.read().await.base_url.clone();
        let body = build_ogads_body(waa_token, self.inner.session.lock().await.language.as_str());
        let auth = credentials_to_sapisid_hash(credentials, &base_url);
        let mut req = self
            .inner
            .http
            .post(&url)
            .header(header_constants::CONTENT_TYPE, crate::constants::mime::JSON_PROTOBUF)
            .header(header_constants::X_GOOG_API_KEY, OGADS_API_KEY)
            .header(header_constants::COOKIE, cookie_header)
            .header(header_constants::USER_AGENT, BROWSER_LIKE)
            .header(header_constants::REFERER, format!("{base_url}/"))
            .header(header_constants::ORIGIN, base_url.clone())
            .header(header_constants::X_CLIENT_DATA, crate::constants::headers::X_CLIENT_DATA_VALUE);
        if let Some(auth) = auth.clone() {
            req = req.header(header_constants::AUTHORIZATION, auth);
        }
        let started = std::time::Instant::now();
        let response = req
            .body(body.clone())
            .send()
            .await
            .map_err(|e| Error::Transient(format!("ogads GetAsyncData request failed: {e}")))?;

        let status = response.status();
        let response_headers = response.headers().clone();
        let text = response.text().await.map_err(Error::Request)?;
        self.maybe_record_har(
            POST,
            &url,
            &HeaderMap::new(),
            body.as_bytes(),
            status.as_u16(),
            &response_headers,
            text.as_bytes(),
            started.elapsed(),
        )
        .await?;
        if !status.is_success() {
            return Err(Error::api(status, text));
        }

        Ok(text)
    }

    async fn fetch_app_page(&self) -> Result<String> {
        let (language, cookie_header, base_url) = {
            let session = self.inner.session.lock().await;
            let config = self.inner.config.read().await;
            let cookie_header = self.cookies().await.to_header_value();
            (session.language.clone(), cookie_header, config.base_url.clone())
        };

        let url = format!("{base_url}{}", crate::constants::urls::APP_LANGUAGE_PATH_TEMPLATE.replace("{}", &language));
        let started = std::time::Instant::now();
        let response = self
            .inner
            .http
            .get(&url)
            .header(header_constants::COOKIE, &cookie_header)
            .header(header_constants::USER_AGENT, BROWSER_LIKE)
            .header(header_constants::ACCEPT, "text/html")
            .send()
            .await
            .map_err(|e| Error::Transient(format!("failed to fetch Gemini /app: {e}")))?;

        let status = response.status();
        let response_headers = response.headers().clone();
        let cookies: Vec<(String, String)> = response
            .cookies()
            .map(|c| (c.name().to_string(), c.value().to_string()))
            .collect();
        let text = response.text().await.map_err(Error::Request)?;

        self.merge_response_cookies_owned(cookies.into_iter()).await;

        self.maybe_record_har(
            GET,
            &url,
            &HeaderMap::new(),
            &[],
            status.as_u16(),
            &response_headers,
            text.as_bytes(),
            started.elapsed(),
        )
        .await?;

        if !status.is_success() {
            if status == StatusCode::BAD_REQUEST
                && !crate::session::looks_like_signed_in_html(&text)
            {
                return Err(self.build_not_signed_in_error(&text).await);
            }
            return Err(Error::api(status, text));
        }

        if !crate::session::looks_like_signed_in_html(&text) {
            return Err(self.build_not_signed_in_error(&text).await);
        }

        Ok(text)
    }

    async fn build_not_signed_in_error(&self, body: &str) -> Error {
        let reason = diagnose_signed_in_state(body)
            .err()
            .map(|f| f.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let missing_legacy = {
            let cookies = self.cookies().await;
            cookies.to_credentials().map(|c| c.missing_legacy_cookies()).unwrap_or_default()
        };
        let mut message = format!(
            "cookies rejected by Gemini /app ({reason}); page did not contain signed-in markers"
        );
        if !missing_legacy.is_empty() {
            message.push_str(". Likely missing legacy cookies: ");
            message.push_str(&missing_legacy.join(", "));
            message.push_str(MISSING_LEGACY_COOKIES_ADVICE);
        }
        Error::not_signed_in(message)
    }

    /// Fetches the Gemini root page (`/?hl={language}`) to extract session
    /// metadata such as the `SNlM0e` async token that is only present on the
    /// root page in some live sessions.
    async fn fetch_root_page(&self) -> Result<String> {
        let (language, cookie_header, base_url) = {
            let session = self.inner.session.lock().await;
            let config = self.inner.config.read().await;
            let cookie_header = self.cookies().await.to_header_value();
            (session.language.clone(), cookie_header, config.base_url.clone())
        };

        let url = format!(
            "{base_url}{}",
            crate::constants::urls::ROOT_LANGUAGE_PATH_TEMPLATE.replace("{}", &language)
        );
        let started = std::time::Instant::now();
        let response = self
            .inner
            .http
            .get(&url)
            .header(header_constants::COOKIE, &cookie_header)
            .header(header_constants::USER_AGENT, BROWSER_LIKE)
            .header(header_constants::ACCEPT, "text/html")
            .send()
            .await
            .map_err(|e| Error::Transient(format!("failed to fetch Gemini root page: {e}")))?;

        let status = response.status();
        let response_headers = response.headers().clone();
        let cookies: Vec<(String, String)> = response
            .cookies()
            .map(|c| (c.name().to_string(), c.value().to_string()))
            .collect();
        let text = response.text().await.map_err(Error::Request)?;

        self.merge_response_cookies_owned(cookies.into_iter()).await;

        self.maybe_record_har(
            GET,
            &url,
            &HeaderMap::new(),
            &[],
            status.as_u16(),
            &response_headers,
            text.as_bytes(),
            started.elapsed(),
        )
        .await?;

        if !status.is_success() {
            return Err(Error::api(status, text));
        }

        Ok(text)
    }

    fn build_request_header_map(headers: &[(String, String)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (key, value) in headers {
            if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(v) = reqwest::header::HeaderValue::from_str(value) {
                    map.insert(name, v);
                }
            }
        }
        map
    }

    async fn accept_consent_and_refresh(&self, save_url: &str) -> Result<String> {
        let cookie_header = self.cookies().await.to_header_value();

        let (language, base_url) = {
            let config = self.inner.config.read().await;
            (config.language.clone(), config.base_url.clone())
        };
        let response = self
            .inner
            .http
            .post(save_url)
            .header(header_constants::COOKIE, &cookie_header)
            .header(header_constants::USER_AGENT, BROWSER_LIKE)
            .header(header_constants::REFERER, format!("{base_url}{}", crate::constants::urls::APP_LANGUAGE_PATH_TEMPLATE.replace("{}", &language)))
            .header(header_constants::ORIGIN, base_url)
            .header(header_constants::CONTENT_LENGTH, "0")
            .body("")
            .send()
            .await
            .map_err(|e| Error::Transient(format!("consent save request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() && status.as_u16() != 204 {
            let text = response.text().await.unwrap_or_default();
            return Err(Error::api(status, text));
        }

        self.merge_response_cookies(response.cookies()).await;

        self.fetch_app_page().await
    }

    async fn build_headers(
        &self,
        reqid: Option<&str>,
        waa_context: Option<&str>,
        authorization: Option<&str>,
        auth_user: Option<&str>,
        endpoint: Option<&str>,
    ) -> Vec<(String, String)> {
        let origin = self.inner.config.read().await.base_url.clone();
        let mut headers = vec![
            (
                header_constants::CONTENT_TYPE.to_string(),
                har_constants::REQUEST_MIME_TYPE.to_string(),
            ),
            (header_constants::USER_AGENT.to_string(), BROWSER_LIKE.to_string()),
            (header_constants::ORIGIN.to_string(), origin.clone()),
            (header_constants::REFERER.to_string(), format!("{origin}/")),
            (
                header_constants::X_SAME_DOMAIN.to_string(),
                header_constants::X_SAME_DOMAIN_VALUE.to_string(),
            ),
            (
                header_constants::CACHE_CONTROL.to_string(),
                header_constants::CACHE_CONTROL_NO_CACHE.to_string(),
            ),
            (
                header_constants::PRAGMA.to_string(),
                header_constants::PRAGMA_NO_CACHE.to_string(),
            ),
            (
                header_constants::X_CLIENT_DATA.to_string(),
                crate::constants::headers::X_CLIENT_DATA_VALUE.to_string(),
            ),
            (
                "sec-ch-ua".to_string(),
                header_constants::SEC_CH_UA.to_string(),
            ),
            (
                "sec-ch-ua-mobile".to_string(),
                header_constants::SEC_CH_UA_MOBILE.to_string(),
            ),
            (
                "sec-ch-ua-platform".to_string(),
                header_constants::SEC_CH_UA_PLATFORM.to_string(),
            ),
            (
                "sec-fetch-dest".to_string(),
                header_constants::SEC_FETCH_DEST.to_string(),
            ),
            (
                "sec-fetch-mode".to_string(),
                header_constants::SEC_FETCH_MODE.to_string(),
            ),
            (
                header_constants::SEC_FETCH_SITE.to_string(),
                header_constants::SEC_FETCH_SITE_SAME_ORIGIN.to_string(),
            ),
        ];
        if let Some(id) = reqid {
            let ext = serde_json::json!([id, 1]).to_string();
            headers.push(("x-goog-ext-525005358-jspb".to_string(), ext));
        }
        if let Some(ctx) = waa_context {
            headers.push(("x-goog-ext-525001261-jspb".to_string(), ctx.to_string()));
        }
        if endpoint == Some(transport::BATCHEXECUTE_ENDPOINT) {
            headers.push(("x-goog-ext-73010989-jspb".to_string(), "[]".to_string()));
        } else {
            headers.push(("x-goog-ext-73010989-jspb".to_string(), "[0]".to_string()));
            headers.push(("x-goog-ext-73010990-jspb".to_string(), "[0,0,0]".to_string()));
        }
        if let Some(auth) = authorization {
            headers.push((header_constants::AUTHORIZATION.to_string(), auth.to_string()));
        }
        if let Some(user) = auth_user {
            headers.push((header_constants::X_GOOG_AUTHUSER.to_string(), user.to_string()));
        }
        headers
    }

    async fn merge_response_cookies<'a>(
        &self,
        cookies: impl Iterator<Item = reqwest::cookie::Cookie<'a>>,
    ) {
        let owned: Vec<(String, String)> =
            cookies.map(|c| (c.name().to_string(), c.value().to_string())).collect();
        self.merge_response_cookies_owned(owned.into_iter()).await;
    }

    async fn merge_response_cookies_owned(&self, cookies: impl Iterator<Item = (String, String)>) {
        let mut guard = self.inner.cookies.lock().await;
        guard.merge_response_cookie_pairs(cookies);
    }

    async fn send_with_retry<F, Fut>(&self, operation: F) -> Result<reqwest::Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<reqwest::Response, reqwest::Error>>,
    {
        crate::retry::with_backoff(operation).await
    }

    /// Sends a batchexecute request and retries when the transient WIZ 400
    /// pattern is detected.
    ///
    /// The `make_reqid` closure is called before every attempt so retries use a
    /// fresh `_reqid`. The `operation` closure must return a fresh request each
    /// call. The helper reads the status and body eagerly so it can reclassify
    /// transient 400s before the retry loop commits them as permanent. Any
    /// `Set-Cookie` headers received on the final response are merged back into
    /// the stored credentials.
    async fn send_batchexecute_with_retry<ReqIdFn, ReqIdFut, OpFn, OpFut>(
        &self,
        make_reqid: ReqIdFn,
        operation: OpFn,
    ) -> Result<ResponseWithBody>
    where
        ReqIdFn: Fn() -> ReqIdFut,
        ReqIdFut: std::future::Future<Output = String>,
        OpFn: Fn() -> OpFut,
        OpFut: std::future::Future<Output = std::result::Result<reqwest::Response, reqwest::Error>>,
    {
        let transient_body = std::sync::atomic::AtomicBool::new(false);

        let result = crate::retry::with_backoff_generic(|| {
            let operation = &operation;
            let make_reqid = &make_reqid;
            let transient_body = &transient_body;
            async move {
                let _fresh_reqid = make_reqid().await;
                match operation().await {
                    Ok(response) => {
                        let status = response.status();
                        let headers = response.headers().clone();
                        let cookies: Vec<(String, String)> = response
                            .cookies()
                            .map(|c| (c.name().to_string(), c.value().to_string()))
                            .collect();
                        let body = response.text().await.unwrap_or_default();
                        if status == StatusCode::BAD_REQUEST && is_wiz_transient_400(status, &body)
                        {
                            transient_body.store(true, std::sync::atomic::Ordering::SeqCst);
                            Err(crate::Error::transient(
                                "Google rejected batchexecute with WIZ error frames",
                            ))
                        } else {
                            Ok(ResponseWithBody { status, headers, cookies, body })
                        }
                    }
                    Err(err) => Err(crate::Error::Request(err)),
                }
            }
        })
        .await;

        if transient_body.load(std::sync::atomic::Ordering::SeqCst) {
            if let Some(recorder) = self.inner.config.read().await.metrics_recorder.clone() {
                recorder.increment_counter(
                    METRIC_RETRIES,
                    &[(OPERATION, "batchexecute")],
                );
            }
        }

        if let Ok(ref response) = result {
            let mut guard = self.inner.cookies.lock().await;
            guard.merge_response_cookie_pairs(response.cookies.clone().into_iter());
        }

        result
    }

    #[allow(clippy::too_many_arguments)]
    async fn maybe_record_har(
        &self,
        method: &str,
        url: &str,
        request_headers: &HeaderMap,
        request_body: &[u8],
        status: u16,
        response_headers: &HeaderMap,
        response_body: &[u8],
        duration: Duration,
    ) -> Result<()> {
        let mut guard = self.inner.har_writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            writer
                .record(
                    method,
                    url,
                    request_headers,
                    request_body,
                    status,
                    response_headers,
                    response_body,
                    duration,
                )
                .await?;
        }
        Ok(())
    }

    async fn clear_conversation_state(&self) {
        let mut session = self.inner.session.lock().await;
        session.conversation_state = None;
    }
}

/// A response whose body has already been read.
///
/// Used by the batchexecute retry helper so transient 400 classification can
/// inspect the body before deciding whether to retry.
struct ResponseWithBody {
    status: reqwest::StatusCode,
    headers: reqwest::header::HeaderMap,
    cookies: Vec<(String, String)>,
    body: String,
}

fn build_default_waa_context() -> String {
    serde_json::json!([
        1,
        null,
        null,
        null,
        WAA_FINGERPRINT_DEFAULT,
        null,
        null,
        0,
        [4, 5, 6, 8],
        null,
        null,
        2,
        null,
        null,
        3,
        1,
        null
    ])
    .to_string()
}

fn build_waa_context_header(
    fingerprint: Option<&str>,
    context: Option<&str>,
    uuid: &str,
) -> String {
    // Prefer a context returned by ogads if it matches the known header shape.
    if let Some(ctx) = context {
        if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(ctx) {
            if is_valid_waa_context_array(&arr) {
                let mut arr = arr;
                arr[15] = serde_json::json!(uuid);
                if arr.get(4).map_or(true, |v| v.is_null()) {
                    arr[4] = serde_json::json!(fingerprint.unwrap_or(WAA_FINGERPRINT_DEFAULT));
                }
                return serde_json::to_string(&arr).unwrap_or_default();
            }
            debug!("ogads WAA context has unexpected shape; falling back to default template");
        }
    }
    serde_json::json!([
        1,
        null,
        null,
        null,
        fingerprint.unwrap_or(WAA_FINGERPRINT_DEFAULT),
        null,
        null,
        0,
        [4, 5, 6, 8],
        null,
        null,
        2,
        null,
        null,
        3,
        1,
        uuid
    ])
    .to_string()
}

/// Validates that an ogads response array looks like the expected
/// `x-goog-ext-525001261-jspb` header shape before we mutate it.
fn is_valid_waa_context_array(arr: &[Value]) -> bool {
    // The header is a 17-element array with specific scalar shapes at indices
    // we touch. This is intentionally conservative: if the upstream response
    // changes, we fall back to the default template rather than send garbage.
    const EXPECTED_LEN: usize = 17;
    if arr.len() != EXPECTED_LEN {
        return false;
    }
    // Index 4 must be a fingerprint (string) or null.
    if !arr.get(4).is_some_and(|v| v.is_null() || v.is_string()) {
        return false;
    }
    // Index 15 must be a uuid/string or null (we will overwrite it).
    if !arr.get(15).is_some_and(|v| v.is_null() || v.is_string()) {
        return false;
    }
    true
}

fn extract_waa_fingerprint_from_model_list(body: &str) -> Option<String> {
    // The Pro model block contains a 16-char hex id that is reused as the WAA
    // fingerprint. Anchor the search to the Pro model entry and require the
    // candidate id to appear inside the model list array (not anywhere else in
    // the page). The mode list in the otAQ7b response is a JSON array where
    // each model is represented as [id, name, description, ...].
    let pro_block_start = body.find("\"Pro\"")?;
    // Find the enclosing array that contains the Pro model entry so we only
    // consider tokens inside the model list.
    let list_start = body[..pro_block_start].rfind('[').unwrap_or(0);
    let list_end = body[pro_block_start..]
        .find("]]]")
        .map(|i| pro_block_start + i + 3)
        .unwrap_or(body.len());
    let model_list = &body[list_start..list_end];

    for (start, _) in model_list.match_indices('"') {
        let inner = &model_list[start + 1..];
        let end = inner.find('"').unwrap_or(inner.len());
        let token = &inner[..end];
        if token.len() == 16
            && token.chars().all(|c| c.is_ascii_hexdigit())
            && model_list.matches(token).count() > 1
        {
            return Some(token.to_string());
        }
    }
    None
}

fn credentials_to_sapisid_hash(cookies: &Cookies, origin: &str) -> Option<String> {
    cookies.to_credentials().ok()?.sapisid_hash(origin)
}

/// Builds a [`ChatResponse`] from parsed content parts.
fn build_chat_response_from_parts(parts: &[ContentPart]) -> Result<ChatResponse> {
    let mut texts = Vec::new();
    let mut thinkings = Vec::new();
    for part in parts {
        match part {
            ContentPart::Text(t) => texts.push(t.clone()),
            ContentPart::Thinking(t) => thinkings.push(t.clone()),
            ContentPart::Image(_)
            | ContentPart::Audio(_)
            | ContentPart::Video(_)
            | ContentPart::ToolCall(_)
            | ContentPart::ToolResult(_) => {}
        }
    }
    Ok(ChatResponse::new(texts.join("")).with_thinking(thinkings.join("")))
}

fn map_state(state: ProtoConversationState) -> crate::session::ConversationState {
    crate::session::ConversationState {
        conversation_id: state.conversation_id,
        response_id: state.response_id,
        response_part_id: state.response_part_id,
        continuation_token: state.continuation_token,
    }
}

/// Parses the `/app` HTML and returns the signed-in account identifiers when
/// the page represents an authenticated Gemini session.
///
/// Specifically, `S06Grb` must be a non-empty numeric string and `oPEP7c` must
/// be present and look like an email address.
pub(crate) fn extract_signed_in_state(body: &str) -> Option<(String, String)> {
    crate::session::diagnose_signed_in_html(body).ok()
}

/// Diagnoses why the `/app` HTML does not look like a signed-in session.
///
/// This mirrors `extract_signed_in_state` but returns the reason instead of
/// `None`. Used by the live probe and `fetch_app_page` for richer error
/// messages.
pub(crate) fn diagnose_signed_in_state(
    body: &str,
) -> std::result::Result<(String, String), crate::session::SignedInFailure> {
    crate::session::diagnose_signed_in_html(body)
}

fn looks_like_email(value: &str) -> bool {
    // Minimal email-shaped check: non-empty local and domain parts separated by
    // a single '@', with at least one '.' in the domain.
    let mut parts = value.splitn(2, '@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !local.starts_with('\'')
        && !domain.starts_with('\'')
}

fn map_proto_state(state: &crate::session::ConversationState) -> ProtoConversationState {
    ProtoConversationState {
        conversation_id: state.conversation_id.clone(),
        response_id: state.response_id.clone(),
        response_part_id: state.response_part_id.clone(),
        continuation_token: state.continuation_token.clone(),
    }
}

/// Builder returned by [`GeminiClient::chat`].
#[non_exhaustive]
pub struct ChatBuilder<'a> {
    client: &'a GeminiClient,
    conversation: Option<Conversation>,
    category: ModelCategory,
    config: Option<GenerationConfig>,
    tools: Option<Vec<Arc<dyn Tool>>>,
    refresh_on_auth_error: bool,
}

// ChatBuilder consumes `self` on send, so cloning the optional config at the
// call site is unnecessary: `generate_raw` only borrows it for the request.

impl<'a> ChatBuilder<'a> {
    /// Sets the model category (and therefore the model family) to use.
    pub fn with_category(mut self, category: ModelCategory) -> Self {
        self.category = category;
        self
    }

    /// Sets generation configuration for this turn.
    pub fn with_config(mut self, config: GenerationConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Sets a system instruction for this turn.
    pub fn with_system_instruction(mut self, instruction: impl Into<String>) -> Self {
        let config = self.config.unwrap_or_default();
        self.config = Some(config.with_system_instruction(instruction));
        self
    }

    /// Registers tools for function calling on this turn.
    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Enables a single retry on `NotSignedIn` errors when a credentials
    /// provider has been registered with the client (Plan 03 wiring).
    pub fn with_refresh_on_auth_error(mut self, refresh: bool) -> Self {
        self.refresh_on_auth_error = refresh;
        self
    }

    /// Sends a text-only message.
    pub async fn send_message(self, text: impl Into<String>) -> Result<ChatResponse> {
        let message = ChatMessage::user(text);
        self.send_message_with_content(message).await
    }

    /// Sends a message that may contain images.
    pub async fn send_message_with_images(
        self,
        text: impl Into<String>,
        images: Vec<ImageSource>,
    ) -> Result<ChatResponse> {
        let message = images
            .into_iter()
            .fold(ChatMessage::user(text), |m, image| m.with_part(ContentPart::Image(image)));
        self.send_message_with_content(message).await
    }

    /// Returns the model category that will be used for this turn.
    #[must_use]
    pub fn category(&self) -> ModelCategory {
        self.category
    }

    /// Sends a fully built [`ChatMessage`].
    ///
    /// This is useful for callers that need to control both text and image
    /// parts (or other future content types) directly.
    pub async fn send_message_with_content(self, message: ChatMessage) -> Result<ChatResponse> {
        let mut config =
            if self.config.is_some() {
                self.config
            } else {
                self.client.inner.config.read().await.system_instruction.clone().map(
                    |instruction| GenerationConfig::default().with_system_instruction(instruction),
                )
            };

        let refresh_on_auth_error = self.refresh_on_auth_error;
        if refresh_on_auth_error {
            if let Some(ref mut cfg) = config {
                // Use a sentinel to carry the flag through the config path.
                // Since GenerationConfig does not expose refresh flag, we store
                // it temporarily on PreparedRequest after prepare_request.
                let _ = cfg.max_tool_turns;
            }
        }

        let response = if let Some(tools) = self.tools {
            self.client.generate_with_tools(&message, tools, self.category, config).await?
        } else {
            let prepared =
                prepare_request(self.conversation.as_ref(), &message, config, self.category)?;
            let prepared = PreparedRequest {
                refresh_on_auth_error,
                ..prepared
            };
            let body = self.client.generate_raw_with_prepared(&prepared).await?;
            parse_chat_response(&body)?
        };

        if let Some(mut conversation) = self.conversation {
            conversation.add_message(message);
            conversation.add_model_text(response.text.clone());
        }

        Ok(response)
    }
}

#[cfg(test)]
mod client_tests {
    use super::*;
    use futures::StreamExt;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Default)]
    struct CountingHook {
        requests: AtomicUsize,
        responses: AtomicUsize,
        request_prompt: tokio::sync::Mutex<Option<String>>,
    }

    impl Clone for CountingHook {
        fn clone(&self) -> Self {
            Self {
                requests: AtomicUsize::new(self.requests.load(Ordering::SeqCst)),
                responses: AtomicUsize::new(self.responses.load(Ordering::SeqCst)),
                request_prompt: tokio::sync::Mutex::new(None),
            }
        }
    }

    impl HttpHook for CountingHook {
        fn on_request<'a>(
            &'a self,
            request: &'a PreparedRequest,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
            self.requests.fetch_add(1, Ordering::SeqCst);
            let text = request.prompt.clone();
            Box::pin(async move {
                *self.request_prompt.lock().await = Some(text);
                Ok(())
            })
        }

        fn on_response<'a>(
            &'a self,
            _response: &'a ChatResponse,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
            self.responses.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move { Ok(()) })
        }
    }

    #[derive(Debug, Default, Clone)]
    struct ErrorHook {
        on_request: bool,
    }

    impl HttpHook for ErrorHook {
        fn on_request<'a>(
            &'a self,
            _request: &'a PreparedRequest,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
            if self.on_request {
                Box::pin(async move { Err(Error::Hook("request hook error".to_string())) })
            } else {
                Box::pin(async move { Ok(()) })
            }
        }

        fn on_response<'a>(
            &'a self,
            _response: &'a ChatResponse,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
            if !self.on_request {
                Box::pin(async move { Err(Error::Hook("response hook error".to_string())) })
            } else {
                Box::pin(async move { Ok(()) })
            }
        }
    }

    async fn build_client(hook: impl HttpHook + 'static) -> GeminiClient {
        GeminiClient::from_cookie_header(
            "__Secure-1PSID=abc; __Secure-1PSIDCC=def; __Secure-1PAPISID=papi; SID=s; HSID=h; SSID=s",
        )
        .unwrap()
        .with_http_hook(hook)
        .await
        .with_max_retries(0)
        .await
    }

    #[tokio::test]
    async fn response_hook_called_inside_stream() {
        let concrete = std::sync::Arc::new(CountingHook::default());
        let hook: std::sync::Arc<dyn HttpHook> = concrete.clone();
        let client = build_client(hook).await;

        let body = include_str!("../tests/fixtures/turn1_response_raw.txt");
        let bytes_stream = futures::stream::iter(vec![Ok(bytes::Bytes::from(body))]);

        let mut stream = GeminiClient::stream_responses(bytes_stream, client);
        let mut chunks = Vec::new();
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk.unwrap());
        }

        assert!(!chunks.is_empty());
        assert_eq!(concrete.responses.load(Ordering::SeqCst), chunks.len());
    }

    #[tokio::test]
    async fn request_hook_observes_prepared_request() {
        let concrete = std::sync::Arc::new(CountingHook::default());
        let hook: std::sync::Arc<dyn HttpHook> = concrete.clone();
        let client = GeminiClient::from_cookie_header(
            "__Secure-1PSID=abc; __Secure-1PSIDCC=def; __Secure-1PAPISID=papi; SID=s; HSID=h; SSID=s",
        )
        .unwrap()
        .with_http_hook(hook)
        .await;

        let prepared = PreparedRequest {
            prompt: "What is Rust?".to_string(),
            inline_images: vec![],
            inline_audio: vec![],
            inline_video: vec![],
            config: None,
            category: ModelCategory::Auto,
            tools: None,
            refresh_on_auth_error: false,
        };

        client.run_request_hook(&prepared).await.unwrap();

        assert_eq!(concrete.requests.load(Ordering::SeqCst), 1);
        assert_eq!(concrete.request_prompt.lock().await.as_deref(), Some("What is Rust?"));
    }

    #[tokio::test]
    async fn response_hook_is_non_fatal() {
        let hook: std::sync::Arc<dyn HttpHook> =
            std::sync::Arc::new(ErrorHook { on_request: false });
        let client = GeminiClient::from_cookie_header(
            "__Secure-1PSID=abc; __Secure-1PSIDCC=def; __Secure-1PAPISID=papi; SID=s; HSID=h; SSID=s",
        )
        .unwrap()
        .with_http_hook(hook.clone())
        .await;

        let response = ChatResponse::new("hello");
        client
            .run_response_hook(&response)
            .await
            .expect("non-fatal response hook errors should be swallowed");
    }

    #[tokio::test]
    async fn fatal_request_hook_errors_abort() {
        let hook: std::sync::Arc<dyn HttpHook> =
            std::sync::Arc::new(ErrorHook { on_request: true });
        let client = GeminiClient::from_cookie_header(
            "__Secure-1PSID=abc; __Secure-1PSIDCC=def; __Secure-1PAPISID=papi; SID=s; HSID=h; SSID=s",
        )
        .unwrap()
        .with_http_hook(hook.clone())
        .await
        .with_fatal_hook_errors(true)
        .await;

        let prepared = PreparedRequest {
            prompt: "hello".to_string(),
            inline_images: vec![],
            inline_audio: vec![],
            inline_video: vec![],
            config: None,
            category: ModelCategory::Auto,
            tools: None,
            refresh_on_auth_error: false,
        };

        let err = client
            .run_request_hook(&prepared)
            .await
            .expect_err("fatal hook errors should abort");

        match err {
            Error::Hook(msg) => assert_eq!(msg, "request hook error"),
            other => panic!("expected Error::Hook, got {other:?}"),
        }
    }

    #[test]
    fn extract_waa_fingerprint_anchors_to_pro_model_block() {
        // Minimal otAQ7b-like model list with a decoy hex token outside the list.
        let body = r#"decoytoken00000000 [[["cf41b0e0dd7d53e5","Flash-Lite",...],["fbb127bbb056c959","Flash",...],["9d8ca3786ebdfbea","Pro","Advanced",...,"9d8ca3786ebdfbea"]]]"#;
        assert_eq!(
            extract_waa_fingerprint_from_model_list(body),
            Some("9d8ca3786ebdfbea".to_string())
        );
    }

    #[test]
    fn extract_waa_fingerprint_ignores_decoy_outside_model_list() {
        let body = r#"outside1234567890 [[["fbb127bbb056c959","Flash",...]]]"#;
        assert!(extract_waa_fingerprint_from_model_list(body).is_none());
    }

    #[tokio::test]
    async fn stream_responses_yields_text_and_ingests_state() {
        let body = include_str!("../tests/fixtures/conversation_state.json");
        let bytes_stream = futures::stream::iter(vec![Ok(bytes::Bytes::from(body))]);
        let client =
            GeminiClient::from_cookie_header("__Secure-1PSID=abc; __Secure-1PSIDCC=def").unwrap();

        let mut stream = GeminiClient::stream_responses(bytes_stream, client.clone());
        let mut chunks = Vec::new();
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk.unwrap());
        }

        assert!(!chunks.is_empty());
        let last = chunks.last().unwrap();
        assert!(!last.text().is_empty());

        let session = client.inner.session.lock().await;
        assert!(session.conversation_state.is_some());
    }

    #[tokio::test]
    async fn stream_responses_handles_empty_body() {
        let bytes_stream =
            futures::stream::iter(Vec::<std::result::Result<bytes::Bytes, reqwest::Error>>::new());
        let client =
            GeminiClient::from_cookie_header("__Secure-1PSID=abc; __Secure-1PSIDCC=def").unwrap();

        let mut stream = GeminiClient::stream_responses(bytes_stream, client);
        let first = stream.next().await;
        // Empty body has no parseable chunks; conversation-state ingestion may
        // produce an error, but the stream must not panic.
        assert!(first.is_none() || first.unwrap().is_err());
    }
}
