//! Main SDK client.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::debug;

use crate::auth::{Cookies, Credentials};
use crate::chat::{
    prepare_request, ChatMessage, ChatResponse, ContentPart, Conversation, GenerationConfig,
    ImageSource, PreparedRequest,
};
use crate::errors::{Error, Result};
use crate::models::{ModelCategory, ModelInfo};
use crate::proto::parser::{extract_conversation_state, parse_chat_response, parse_model_list};
use crate::proto::slots::{build_inner_req_list, ConversationState as ProtoConversationState};
use crate::proto::{
    build_batchexecute_body, build_esy5d_body, build_ogads_body, build_sjbwce_body,
    build_stream_generate_body, build_waa_create_body, fresh_request_uuid,
};
use crate::session::{
    extract_consent_save_url, extract_from_app_html, extract_quoted_value, SessionState,
};
use crate::upload;

const WEB_BASE_URL: &str = "https://gemini.google.com";
const WAA_BASE_URL: &str = "https://waa-pa.clients6.google.com";
const OGADS_BASE_URL: &str = "https://ogads-pa.clients6.google.com";
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36";
const X_CLIENT_DATA: &str = "CI7yygE=";
const WAA_FINGERPRINT_DEFAULT: &str = "e6fa609c3fa255c0";
const WAA_API_KEY: &str = "AIzaSyBGb5fGAyC-pRcRU6MUHb__b_vKha71HRE";
const OGADS_API_KEY: &str = "AIzaSyCbsbvGCe7C9mCtdaTycZB2eUFuzsYKG_E";

/// The main entry point for interacting with the Gemini web frontend.
///
/// A `GeminiClient` holds the HTTP client, cookies, and extracted session state.
/// It is cheaply cloneable; clones share the same underlying session.
#[derive(Clone)]
pub struct GeminiClient {
    inner: Arc<Inner>,
}

struct Inner {
    http: Client,
    cookies: Cookies,
    session: Mutex<SessionState>,
    config: Mutex<ClientConfig>,
}

#[derive(Debug, Clone)]
struct ClientConfig {
    language: String,
    max_retries: usize,
    timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            max_retries: 3,
            timeout: Duration::from_secs(120),
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

    /// Creates a client from a [`HashMap`] of cookies.
    ///
    /// # Errors
    ///
    /// Returns an error if the map is missing required cookies or if the
    /// underlying HTTP client cannot be built.
    pub fn from_hashmap(cookies: HashMap<String, String>) -> Result<Self> {
        Self::from_cookies(cookies)
    }

    /// Sets the language code sent to the Gemini frontend.
    pub fn with_language(self, language: impl Into<String>) -> Self {
        let language = language.into();
        self.update_config_blocking(|config| {
            config.language.clone_from(&language);
        });
        self
    }

    /// Sets the maximum number of retries for transient failures.
    pub fn with_max_retries(self, max_retries: usize) -> Self {
        self.update_config_blocking(|config| config.max_retries = max_retries);
        self
    }

    /// Sets the request timeout.
    pub fn with_timeout(self, timeout: Duration) -> Self {
        self.update_config_blocking(|config| config.timeout = timeout);
        self
    }

    fn update_config_blocking<F>(&self, f: F)
    where
        F: FnOnce(&mut ClientConfig),
    {
        let mut config = self.inner.config.blocking_lock();
        f(&mut config);
    }

    /// Returns a clone of the cookies used by this client.
    pub(crate) fn cookies(&self) -> Cookies {
        self.inner.cookies.clone()
    }

    fn with_config(cookies: Cookies, config: ClientConfig) -> Result<Self> {
        let http = Client::builder()
            .pool_max_idle_per_host(20)
            .connect_timeout(Duration::from_secs(10))
            .timeout(config.timeout)
            .build()
            .map_err(|e| Error::Config(format!("failed to build HTTP client: {e}")))?;

        let mut session = SessionState::new();
        session.language.clone_from(&config.language);
        session.waa_fingerprint = Some(WAA_FINGERPRINT_DEFAULT.to_string());

        Ok(Self {
            inner: Arc::new(Inner {
                http,
                cookies,
                session: Mutex::new(session),
                config: Mutex::new(config),
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
        }
    }

    /// Lists the models available to the signed-in account.
    ///
    /// Internally calls `BardFrontendService.GetUserStatus` through the
    /// batchexecute transport using the `otAQ7b` RPC id.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.ensure_session().await?;

        let url = format!("{WEB_BASE_URL}/_/BardChatUi/data/batchexecute");
        let cookies = self.inner.cookies.clone();
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
    /// Prefer using [`GeminiClient::chat`] for an ergonomic API.
    pub async fn generate(
        &self,
        message: &ChatMessage,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<ChatResponse> {
        let body = self.generate_raw(message, None, category, config).await?;
        let response = parse_chat_response(&body)?;

        if let Ok(state) = extract_conversation_state(&body) {
            let mut session = self.inner.session.lock().await;
            session.conversation_state = Some(map_state(state));
        }

        Ok(response)
    }

    /// Sends a generation request and returns the raw response body.
    ///
    /// This is useful when implementing custom streaming or logging.
    pub async fn generate_raw(
        &self,
        message: &ChatMessage,
        conversation: Option<&Conversation>,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<String> {
        let mut response =
            self.stream_generate_raw(message, conversation, category, config).await?;

        let mut body_bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(Error::Request)? {
            body_bytes.extend_from_slice(&chunk);
        }
        let body = String::from_utf8_lossy(&body_bytes).to_string();

        if let Ok(state) = extract_conversation_state(&body) {
            let mut session = self.inner.session.lock().await;
            session.conversation_state = Some(map_state(state));
        }

        Ok(body)
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

    /// Starts a streaming generation request and returns raw bytes.
    ///
    /// This lower-level method gives callers direct access to the upstream WIZ
    /// byte stream. Conversation state is extracted from the consumed body by
    /// the caller or by [`GeminiClient::generate_raw`].
    pub async fn stream_generate_raw(
        &self,
        message: &ChatMessage,
        conversation: Option<&Conversation>,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<reqwest::Response> {
        self.ensure_session().await?;

        let prepared = prepare_request(conversation, message, config, category)?;
        let (inner_req_list, request_uuid, _headers, cookie_header) =
            self.build_stream_generate_request(&prepared).await?;

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

    async fn build_stream_generate_request(
        &self,
        prepared: &PreparedRequest,
    ) -> Result<(Vec<Value>, String, Vec<(String, String)>, String)> {
        let request_uuid = fresh_request_uuid();
        let cookies = self.inner.cookies.clone();
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

        // Run the WAA / warm-up chain. Individual failures are logged but do not
        // block the session: the server may still accept the request.
        if let Err(e) = self.run_waa_init_chain().await {
            debug!(error = %e, "WAA init chain failed; continuing without WAA token");
        }

        Ok(())
    }

    /// Performs the warm-up/WAA RPC chain captured from the Gemini frontend.
    ///
    /// Stores the resulting WAA token and `x-goog-ext-525001261-jspb` context in
    /// the session state.
    async fn run_waa_init_chain(&self) -> Result<()> {
        let (at, language, build_label, session_id, cookie_header, credentials) = {
            let session = self.inner.session.lock().await;
            (
                session.access_token.clone(),
                session.language.clone(),
                session.build_label.clone(),
                session.session_id.clone(),
                self.inner.cookies.to_header_value(),
                self.inner.cookies.clone(),
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
        let waa_token = self
            .waa_create(&cookie_header)
            .await
            .map_err(|e| Error::Transient(format!("WAA Create failed: {e}")))?;

        // 4. ogads GetAsyncData.
        let waa_context = self
            .ogads_get_async_data(&cookie_header, &credentials, &waa_token)
            .await
            .unwrap_or_else(|_| build_default_waa_context());

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
            (session.language.clone(), self.inner.cookies.to_header_value())
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
        let cookie_header = self.inner.cookies.to_header_value();

        let language = self.inner.config.lock().await.language.clone();
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
            let mut cookies = self.inner.cookies.clone();
            cookies.merge_response_cookies(response.cookies());
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
    // Prefer a context returned by ogads if it is a valid JSON array.
    if let Some(ctx) = context {
        if ctx.starts_with('[') {
            if let Ok(Value::Array(mut arr)) = serde_json::from_str::<Value>(ctx) {
                if arr.len() >= 16 {
                    arr[15] = serde_json::json!(uuid);
                    if arr.get(4).map_or(true, |v| v.is_null()) {
                        arr[4] = serde_json::json!(fingerprint.unwrap_or(WAA_FINGERPRINT_DEFAULT));
                    }
                    return serde_json::to_string(&arr).unwrap_or_default();
                }
            }
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

fn extract_waa_fingerprint_from_model_list(body: &str) -> Option<String> {
    // The Pro model block contains a 16-char hex id that is reused as the WAA
    // fingerprint. Scan the otAQ7b response for the first 16-char hex token
    // that appears more than once.
    for (start, _) in body.match_indices('"') {
        let inner = &body[start + 1..];
        let end = inner.find('"').unwrap_or(inner.len());
        let token = &inner[..end];
        if token.len() == 16
            && token.chars().all(|c| c.is_ascii_hexdigit())
            && body.matches(token).count() > 1
        {
            return Some(token.to_string());
        }
    }
    None
}

fn credentials_to_sapisid_hash(cookies: &Cookies, origin: &str) -> Option<String> {
    cookies.to_credentials().ok()?.sapisid_hash(origin)
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
pub struct ChatBuilder<'a> {
    client: &'a GeminiClient,
    conversation: Option<Conversation>,
    category: ModelCategory,
    config: Option<GenerationConfig>,
}

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
        let mut message = ChatMessage::user(text);
        for image in images {
            message.parts.push(ContentPart::Image(image));
        }
        self.send_message_with_content(message).await
    }

    /// Sends a fully built [`ChatMessage`].
    ///
    /// This is useful for callers that need to control both text and image
    /// parts (or other future content types) directly.
    pub async fn send_message_with_content(self, message: ChatMessage) -> Result<ChatResponse> {
        let response = self.client.generate(&message, self.category, self.config).await?;

        if let Some(mut conversation) = self.conversation {
            conversation.add_message(message);
            conversation.add_model_text(response.text.clone());
        }

        Ok(response)
    }
}
