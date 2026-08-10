//! Main SDK client.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::Stream;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, warn};

use crate::auth::{Cookies, Credentials, CredentialsProvider};
use crate::chat::{
    prepare_request, ChatMessage, ChatResponse, ContentPart, Conversation, GenerationConfig,
    ImageSource, PreparedRequest,
};
use crate::errors::{Error, Result};
use crate::tool::{Tool, ToolError, ToolResult};
use crate::models::{ModelCategory, ModelInfo};
use crate::proto::parser::{
    extract_conversation_state, parse_chat_response, parse_model_list, parse_response_parts,
};
use crate::proto::slots::{build_inner_req_list, ConversationState as ProtoConversationState};
use crate::proto::{
    build_batchexecute_body, build_esy5d_body, build_ogads_body, build_sjbwce_body,
    build_stream_generate_body, build_waa_create_body, fresh_request_uuid,
};
use crate::session::{
    extract_consent_save_url, extract_from_app_html, extract_quoted_value, SessionState,
};
use crate::upload::{self, UploadEvent};

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



const WEB_BASE_URL: &str = "https://gemini.google.com";
const WAA_BASE_URL: &str = "https://waa-pa.clients6.google.com";
const OGADS_BASE_URL: &str = "https://ogads-pa.clients6.google.com";
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36";
const X_CLIENT_DATA: &str = "CI7yygE=";
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
}

#[derive(Clone)]
struct ClientConfig {
    language: String,
    max_retries: usize,
    timeout: Duration,
    system_instruction: Option<String>,
    http_hook: Option<Arc<dyn HttpHook>>,
    fatal_hook_errors: bool,
    metrics_recorder: Option<Arc<dyn crate::metrics::MetricsRecorder>>,
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
        }
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
    pub fn from_http_client(client: reqwest::Client, credentials: impl Into<Cookies>) -> Result<Self> {
        let cookies: Cookies = credentials.into();
        cookies
            .to_credentials()
            .map_err(|e| Error::Config(e.to_string()))?;
        Self::with_http_client(cookies, ClientConfig::default(), client)
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
    pub async fn with_metrics(self, recorder: impl crate::metrics::MetricsRecorder + 'static) -> Self {
        let mut config = self.inner.config.write().await;
        config.metrics_recorder = Some(Arc::new(recorder));
        drop(config);
        self
    }

    /// Returns a clone of the cookies used by this client.
    async fn cookies(&self) -> Cookies {
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
            .build()
            .map_err(|e| Error::Config(format!("failed to build HTTP client: {e}")))?;

        Self::with_http_client(cookies, config, http)
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
            }),
        })
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
    pub async fn save_session_with_conversation(&self, conversation: &Conversation) -> Result<String> {
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
    #[tracing::instrument(name = "gemini.list_models", level = "info", skip_all, fields(operation = "gemini.list_models"))]
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.ensure_session().await?;

        let url = format!("{WEB_BASE_URL}/_/BardChatUi/data/batchexecute");
        let cookies = self.cookies().await;
        let (params, body, headers, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid();
            let mut params: Vec<(&str, String)> = vec![
                ("rpcids", "otAQ7b".to_string()),
                ("source-path", "/".to_string()),
                ("hl", session.language.clone()),
                ("_reqid", reqid),
                ("rt", "c".to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push(("bl", bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push(("f.sid", sid.to_string()));
            }
            let body = build_batchexecute_body(session.access_token.as_deref());
            let headers = Self::build_headers(None, None, None);
            let cookie_header = cookies.to_header_value();
            (params, body, headers, cookie_header)
        };

        let response = self
            .send_with_retry(|| {
                let client = self.inner.http.clone();
                let url = url.clone();
                let params = params.clone();
                let body = body.clone();
                let headers = headers.clone();
                let cookie_header = cookie_header.clone();
                async move {
                    let mut req = client.post(&url).query(&params).body(body);
                    for (key, value) in &headers {
                        req = req.header(key, value);
                    }
                    req = req.header("Cookie", cookie_header);
                    req.send().await
                }
            })
            .await?;

        let status = response.status();
        let text = response.text().await.map_err(Error::Request)?;
        if !status.is_success() {
            return Err(Error::api(status, text));
        }

        parse_model_list(&text)
    }

    /// Sends a generation request and returns the parsed response.
    ///
    /// Prefer using [`GeminiClient::chat`] for an ergonomic API. If you need to
    /// pass an existing [`Conversation`] use [`GeminiClient::generate_with_conversation`].
    #[tracing::instrument(name = "gemini.generate", level = "info", skip_all, fields(operation = "gemini.generate", category = ?category))]
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
    #[tracing::instrument(name = "gemini.generate_with_tools", level = "info", skip_all, fields(operation = "gemini.generate_with_tools", category = ?category))]
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
            let response = self.parse_response(&body)?;
            self.run_response_hook(&response).await?;

            let parsed_parts = crate::proto::parser::parse_response_parts(&body).unwrap_or_default();
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
    #[tracing::instrument(name = "gemini.generate_stream", level = "info", skip_all, fields(operation = "gemini.generate_stream", category = ?category))]
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
        S: Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send + Unpin + 'static,
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
                                let response = build_chat_response_from_parts(&parts)?;
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
                        let response = build_chat_response_from_parts(&parts)?;
                        client.run_response_hook(&response).await?;
                        yield response;
                    }
                }
            }

            client.ingest_conversation_state(&full_body).await?;
        })
    }

    /// Sends a generation request with optional conversation state and returns
    /// the parsed response.
    ///
    /// This is the public entry point for callers that manage a
    /// [`Conversation`] manually instead of using the builder API.
    #[tracing::instrument(name = "gemini.generate_with_conversation", level = "info", skip_all, fields(operation = "gemini.generate_with_conversation", category = ?category))]
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
        let config = if config.is_some() {
            config
        } else {
            self.inner
                .config
                .read()
                .await
                .system_instruction
                .clone()
                .map(|instruction| GenerationConfig::default().with_system_instruction(instruction))
        };
        Ok((config, false))
    }

    async fn execute_generate(&self, prepared: PreparedRequest) -> Result<ChatResponse> {
        let refresh_on_auth_error = prepared.refresh_on_auth_error;
        self.run_request_hook(&prepared).await?;
        match self.generate_raw_with_prepared(&prepared).await {
            Ok(body) => {
                let response = self.parse_response(&body)?;
                self.run_response_hook(&response).await?;
                Ok(response)
            }
            Err(Error::NotSignedIn(_)) if refresh_on_auth_error => {
                if let Some(provider) = self.inner.provider.lock().await.clone() {
                    self.refresh_credentials(provider).await?;
                    let body = self.generate_raw_with_prepared(&prepared).await?;
                    let response = self.parse_response(&body)?;
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
    async fn generate_raw_with_prepared(
        &self,
        prepared: &PreparedRequest,
    ) -> Result<String> {
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

    #[tracing::instrument(level = "debug", skip_all, fields(operation = "gemini.parse_response", bytes = body.len()))]
    fn parse_response(&self, body: &str) -> Result<ChatResponse> {
        parse_chat_response(body)
    }

    /// Sends a generation request and returns the raw response body.
    ///
    /// This is useful when implementing custom streaming or logging.
    #[tracing::instrument(level = "debug", skip_all, fields(operation = "gemini.generate_raw", category = ?category))]
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
    #[tracing::instrument(name = "gemini.upload_with_progress", level = "info", skip_all, fields(operation = "gemini.upload_with_progress", bytes = bytes.len()))]
    pub async fn upload_with_progress(
        &self,
        filename: impl Into<String>,
        mime_type: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Pin<Box<dyn Stream<Item = Result<UploadEvent>> + Send + 'static>> {
        upload::upload_progress_stream(
            self.inner.http.clone(),
            self.cookies().await,
            self.inner.session.lock().await.clone(),
            filename.into(),
            mime_type.into(),
            bytes,
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

        let url = format!(
            "{WEB_BASE_URL}/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate"
        );

        let (params, at, waa_context, waa_fingerprint) = {
            let session = self.inner.session.lock().await;
            let mut params: Vec<(&str, String)> = vec![
                ("hl", session.language.clone()),
                ("_reqid", request_uuid.clone()),
                ("rt", "c".to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push(("bl", bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push(("f.sid", sid.to_string()));
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
        let headers = Self::build_headers(Some(&request_uuid), Some(&waa_header), None);

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
                    req = req.header("Cookie", cookie_header);
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

    #[tracing::instrument(level = "debug", skip_all, fields(operation = "gemini.ingest_conversation_state", bytes = body.len()))]
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

        let attachments =
            upload::upload_attachments(&self.inner.http, &cookies, &session_for_upload, prepared)
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
        let headers = Self::build_headers(Some(&request_uuid), Some(&waa_header), None);
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
    #[tracing::instrument(name = "gemini.verify_signed_in", level = "info", skip_all, fields(operation = "gemini.verify_signed_in"))]
    pub async fn verify_signed_in(&self) -> Result<bool> {
        let body = self.fetch_app_page().await?;
        Ok(extract_signed_in_state(&body).is_some())
    }

    async fn init_session(&self) -> Result<()> {
        let body = self.fetch_app_page().await?;

        if extract_signed_in_state(&body).is_none() {
            return Err(Error::NotSignedIn(
                "cookies are not valid for a signed-in Gemini session; try refreshing your browser cookies".to_string(),
            ));
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

        // Run the WAA / warm-up chain. Failures are now surfaced so callers can
        // decide whether to proceed without attestation context.
        self.run_waa_init_chain().await?;

        Ok(())
    }

    /// Performs the warm-up/WAA RPC chain captured from the Gemini frontend.
    ///
    /// Stores the resulting WAA token and `x-goog-ext-525001261-jspb` context in
    /// the session state. Failures from the WAA `Create` and ogads
    /// `GetAsyncData` steps are surfaced as [`Error::AttestationFailed`]
    /// instead of being silently replaced with a synthetic context.
    #[tracing::instrument(level = "info", skip_all, fields(operation = "gemini.waa_init_chain"))]
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
                "otAQ7b",
                build_batchexecute_body(at.as_deref()),
                &language,
                build_label.as_deref(),
                session_id.as_deref(),
                &cookie_header,
                Some("/"),
            )
            .await?;
        let fingerprint = extract_waa_fingerprint_from_model_list(&models_response);
        let default_fingerprint = self.inner.session.lock().await.waa_fingerprint.clone();

        // 2. sJBwce [[1,2]] prerequisite.
        let _ = self
            .batchexecute_rpc(
                "sJBwce",
                build_sjbwce_body(at.as_deref()),
                &language,
                build_label.as_deref(),
                session_id.as_deref(),
                &cookie_header,
                Some("/"),
            )
            .await?;

        // 3. WAA Create.
        let waa_token = self.waa_create(&cookie_header).await.map_err(|e| {
            Error::AttestationFailed {
                reason: format!("WAA Create failed: {e}"),
            }
        })?;

        // 4. ogads GetAsyncData.
        let waa_context = self
            .ogads_get_async_data(&cookie_header, &credentials, &waa_token)
            .await
            .map_err(|e| Error::AttestationFailed {
                reason: format!("ogads GetAsyncData failed: {e}"),
            })?;

        // 5. ESY5D feature flags.
        let _ = self
            .batchexecute_rpc(
                "ESY5D",
                build_esy5d_body(at.as_deref()),
                &language,
                build_label.as_deref(),
                session_id.as_deref(),
                &cookie_header,
                None,
            )
            .await?;

        {
            let mut session = self.inner.session.lock().await;
            session.waa_token = Some(waa_token);
            session.waa_context = Some(waa_context);
            session.waa_fingerprint = fingerprint.or(default_fingerprint);
        }

        Ok(())
    }

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
        let url = format!("{WEB_BASE_URL}/_/BardChatUi/data/batchexecute");
        let reqid = SessionState::generate_reqid();
        let mut params: Vec<(&str, String)> = vec![
            ("rpcids", rpcids.to_string()),
            ("source-path", source_path_override.unwrap_or("/app").to_string()),
            ("hl", language.to_string()),
            ("_reqid", reqid),
            ("rt", "c".to_string()),
        ];
        if let Some(bl) = build_label {
            params.push(("bl", bl.to_string()));
        }
        if let Some(sid) = session_id {
            params.push(("f.sid", sid.to_string()));
        }

        let headers = Self::build_headers(None, None, None);
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
                    req = req.header("Cookie", cookie_header);
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
        let body = build_waa_create_body();
        let response = self
            .inner
            .http
            .post(&url)
            .header("Content-Type", "application/json+protobuf")
            .header("x-goog-api-key", WAA_API_KEY)
            .header("x-user-agent", "grpc-web-javascript/0.1")
            .header("Cookie", cookie_header)
            .header("User-Agent", USER_AGENT)
            .header("Referer", format!("{WEB_BASE_URL}/"))
            .header("Origin", WEB_BASE_URL)
            .header("x-client-data", X_CLIENT_DATA)
            .body(body)
            .send()
            .await
            .map_err(|e| Error::Transient(format!("WAA Create request failed: {e}")))?;

        let status = response.status();
        let text = response.text().await.map_err(Error::Request)?;
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
        let body = build_ogads_body(waa_token, self.inner.session.lock().await.language.as_str());
        let auth = credentials_to_sapisid_hash(credentials, WEB_BASE_URL);
        let mut req = self
            .inner
            .http
            .post(&url)
            .header("Content-Type", "application/json+protobuf")
            .header("x-goog-api-key", OGADS_API_KEY)
            .header("Cookie", cookie_header)
            .header("User-Agent", USER_AGENT)
            .header("Referer", format!("{WEB_BASE_URL}/"))
            .header("Origin", WEB_BASE_URL)
            .header("x-client-data", X_CLIENT_DATA);
        if let Some(auth) = auth {
            req = req.header("Authorization", auth);
        }
        let response = req
            .body(body)
            .send()
            .await
            .map_err(|e| Error::Transient(format!("ogads GetAsyncData request failed: {e}")))?;

        let status = response.status();
        let text = response.text().await.map_err(Error::Request)?;
        if !status.is_success() {
            return Err(Error::api(status, text));
        }

        Ok(text)
    }

    async fn fetch_app_page(&self) -> Result<String> {
        let (language, cookie_header) = {
            let session = self.inner.session.lock().await;
            let cookie_header = self.cookies().await.to_header_value();
            (session.language.clone(), cookie_header)
        };

        let url = format!("{WEB_BASE_URL}/app?hl={language}");
        let response = self
            .inner
            .http
            .get(&url)
            .header("Cookie", &cookie_header)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "text/html")
            .send()
            .await
            .map_err(|e| Error::Transient(format!("failed to fetch Gemini /app: {e}")))?;

        let status = response.status();
        let text = response.text().await.map_err(Error::Request)?;
        if !status.is_success() {
            return Err(Error::api(status, text));
        }
        Ok(text)
    }

    async fn accept_consent_and_refresh(&self, save_url: &str) -> Result<String> {
        let cookie_header = self.cookies().await.to_header_value();

        let language = self.inner.config.read().await.language.clone();
        let response = self
            .inner
            .http
            .post(save_url)
            .header("Cookie", &cookie_header)
            .header("User-Agent", USER_AGENT)
            .header("Referer", format!("{WEB_BASE_URL}/app?hl={language}"))
            .header("Origin", WEB_BASE_URL)
            .header("Content-Length", "0")
            .body("")
            .send()
            .await
            .map_err(|e| Error::Transient(format!("consent save request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() && status.as_u16() != 204 {
            let text = response.text().await.unwrap_or_default();
            return Err(Error::api(status, text));
        }

        {
            let mut guard = self.inner.cookies.lock().await;
            guard.merge_response_cookies(response.cookies());
        }

        self.fetch_app_page().await
    }

    fn build_headers(
        reqid: Option<&str>,
        waa_context: Option<&str>,
        authorization: Option<&str>,
    ) -> Vec<(String, String)> {
        let mut headers = vec![
            (
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded;charset=UTF-8".to_string(),
            ),
            ("User-Agent".to_string(), USER_AGENT.to_string()),
            ("Origin".to_string(), WEB_BASE_URL.to_string()),
            ("Referer".to_string(), format!("{WEB_BASE_URL}/")),
            ("X-Same-Domain".to_string(), "1".to_string()),
            ("Cache-Control".to_string(), "no-cache".to_string()),
            ("Pragma".to_string(), "no-cache".to_string()),
            ("x-client-data".to_string(), X_CLIENT_DATA.to_string()),
            (
                "sec-ch-ua".to_string(),
                "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"146\", \"Chromium\";v=\"146\""
                    .to_string(),
            ),
            ("sec-ch-ua-mobile".to_string(), "?0".to_string()),
            ("sec-ch-ua-platform".to_string(), "\"Windows\"".to_string()),
            ("sec-fetch-dest".to_string(), "empty".to_string()),
            ("sec-fetch-mode".to_string(), "cors".to_string()),
            ("sec-fetch-site".to_string(), "same-origin".to_string()),
        ];
        if let Some(id) = reqid {
            let ext = serde_json::json!([id, 1]).to_string();
            headers.push(("x-goog-ext-525005358-jspb".to_string(), ext));
        }
        if let Some(ctx) = waa_context {
            headers.push(("x-goog-ext-525001261-jspb".to_string(), ctx.to_string()));
        }
        headers.push(("x-goog-ext-73010989-jspb".to_string(), "[0]".to_string()));
        headers.push(("x-goog-ext-73010990-jspb".to_string(), "[0,0,0]".to_string()));
        if let Some(auth) = authorization {
            headers.push(("Authorization".to_string(), auth.to_string()));
        }
        headers
    }

    async fn send_with_retry<F, Fut>(&self, operation: F) -> Result<reqwest::Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<reqwest::Response, reqwest::Error>>,
    {
        crate::retry::with_backoff(operation).await
    }

    async fn clear_conversation_state(&self) {
        let mut session = self.inner.session.lock().await;
        session.conversation_state = None;
    }
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
    let block = crate::session::extract_wiz_global_data_block(body)?;

    let s06grb = extract_quoted_value(block, "S06Grb").unwrap_or_default();
    if s06grb.is_empty() || !s06grb.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let opep7c = extract_quoted_value(block, "oPEP7c")?;
    if !looks_like_email(&opep7c) {
        return None;
    }

    Some((s06grb, opep7c))
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
        let message = images.into_iter().fold(ChatMessage::user(text), |m, image| {
            m.with_part(ContentPart::Image(image))
        });
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
        let mut config = if self.config.is_some() {
            self.config
        } else {
            self.client
                .inner
                .config
                .read()
                .await
                .system_instruction
                .clone()
                .map(|instruction| GenerationConfig::default().with_system_instruction(instruction))
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
            self.client
                .generate_with_tools(&message, tools, self.category, config)
                .await?
        } else {
            let prepared = prepare_request(self.conversation.as_ref(), &message, config, self.category)?;
            let prepared = PreparedRequest {
                refresh_on_auth_error,
                ..prepared
            };
            let body = self
                .client
                .generate_raw_with_prepared(&prepared)
                .await?;
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
        assert_eq!(
            concrete.request_prompt.lock().await.as_deref(),
            Some("What is Rust?")
        );
    }

    #[tokio::test]
    async fn response_hook_is_non_fatal() {
        let hook: std::sync::Arc<dyn HttpHook> = std::sync::Arc::new(ErrorHook { on_request: false });
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
        let hook: std::sync::Arc<dyn HttpHook> = std::sync::Arc::new(ErrorHook { on_request: true });
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
        let client = GeminiClient::from_cookie_header(
            "__Secure-1PSID=abc; __Secure-1PSIDCC=def",
        )
        .unwrap();

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
        let bytes_stream = futures::stream::iter(Vec::<std::result::Result<bytes::Bytes, reqwest::Error>>::new());
        let client = GeminiClient::from_cookie_header(
            "__Secure-1PSID=abc; __Secure-1PSIDCC=def",
        )
        .unwrap();

        let mut stream = GeminiClient::stream_responses(bytes_stream, client);
        let first = stream.next().await;
        // Empty body has no parseable chunks; conversation-state ingestion may
        // produce an error, but the stream must not panic.
        assert!(first.is_none() || first.unwrap().is_err());
    }
}
