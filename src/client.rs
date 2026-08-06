//! Main SDK client.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::debug;

use crate::auth::Cookies;
use crate::chat::{
    prepare_request, ChatMessage, ChatResponse, ContentPart, Conversation, GenerationConfig,
    ImageSource, PreparedRequest,
};
use crate::errors::{Error, Result};
use crate::models::{ModelCategory, ModelInfo};
use crate::proto::parser::{extract_conversation_state, parse_chat_response, parse_model_list};
use crate::proto::slots::{build_inner_req_list, ConversationState as ProtoConversationState};
use crate::proto::{build_batchexecute_body, build_stream_generate_body, fresh_request_uuid};
use crate::session::{extract_consent_save_url, extract_from_app_html, SessionState};
use crate::upload;

const WEB_BASE_URL: &str = "https://gemini.google.com";
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

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
    session: Mutex<SessionState>,
    config: ClientConfig,
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
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn from_cookie_header(header: &str) -> Result<Self> {
        Self::with_config(Cookies::from_header(header), ClientConfig::default())
    }

    /// Creates a client from a map of cookie names to values.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn from_cookies(cookies: impl Into<Cookies>) -> Result<Self> {
        Self::with_config(cookies.into(), ClientConfig::default())
    }

    /// Creates a client from a [`HashMap`] of cookies.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn from_hashmap(cookies: HashMap<String, String>) -> Result<Self> {
        Self::from_cookies(cookies)
    }

    /// Sets the language code sent to the Gemini frontend.
    ///
    /// # Panics
    ///
    /// Panics if the client has been cloned and is no longer uniquely owned.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        let mut config = self.inner.config.clone();
        config.language = language.into();
        let inner = Arc::get_mut(&mut self.inner).expect("client is uniquely owned");
        inner.config = config;
        self
    }

    /// Sets the maximum number of retries for transient failures.
    ///
    /// # Panics
    ///
    /// Panics if the client has been cloned and is no longer uniquely owned.
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        let mut config = self.inner.config.clone();
        config.max_retries = max_retries;
        let inner = Arc::get_mut(&mut self.inner).expect("client is uniquely owned");
        inner.config = config;
        self
    }

    /// Sets the request timeout.
    ///
    /// # Panics
    ///
    /// Panics if the client has been cloned and is no longer uniquely owned.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        let mut config = self.inner.config.clone();
        config.timeout = timeout;
        let inner = Arc::get_mut(&mut self.inner).expect("client is uniquely owned");
        inner.config = config;
        self
    }

    fn with_config(cookies: Cookies, config: ClientConfig) -> Result<Self> {
        let http = Client::builder()
            .pool_max_idle_per_host(20)
            .connect_timeout(Duration::from_secs(10))
            .timeout(config.timeout)
            .build()
            .map_err(|e| Error::Config(format!("failed to build HTTP client: {e}")))?;

        let mut session = SessionState::new(cookies.clone());
        session.language.clone_from(&config.language);

        Ok(Self {
            inner: Arc::new(Inner {
                http,
                session: Mutex::new(session),
                config,
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
    /// batchexecute transport.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.ensure_session().await?;

        let url = format!("{WEB_BASE_URL}/_/BardChatUi/data/batchexecute");
        let (params, body, headers, cookie_header) = {
            let session = self.inner.session.lock().await;
            let reqid = SessionState::generate_reqid();
            let mut params: Vec<(&str, String)> = vec![
                ("rpcids", "Fd0Qje".to_string()),
                ("source-path", "/app".to_string()),
                ("hl", session.language.clone()),
                ("_reqid", reqid),
                ("rt", "c".to_string()),
                ("pageId", "none".to_string()),
                ("authuser", "0".to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push(("bl", bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push(("f.sid", sid.to_string()));
            }
            let body = build_batchexecute_body(session.access_token.as_deref());
            let headers = Self::build_headers(None);
            let cookie_header = session.cookies.to_header_value();
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
    pub async fn generate(&self, message: &ChatMessage, category: ModelCategory, config: Option<GenerationConfig>) -> Result<ChatResponse> {
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
        self.ensure_session().await?;

        let prepared = prepare_request(conversation, message, config, category)?;
        let (inner_req_list, request_uuid, headers, cookie_header) = self
            .build_stream_generate_request(&prepared)
            .await?;

        let url = format!(
            "{WEB_BASE_URL}/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate"
        );

        let body = {
            let session = self.inner.session.lock().await;
            let mut params: Vec<(&str, String)> = vec![
                ("hl", session.language.clone()),
                ("_reqid", request_uuid.clone()),
                ("rt", "c".to_string()),
                ("pageId", "none".to_string()),
            ];
            if let Some(bl) = session.build_label.as_deref() {
                params.push(("bl", bl.to_string()));
            }
            if let Some(sid) = session.session_id.as_deref() {
                params.push(("f.sid", sid.to_string()));
            }

            let at = session.access_token.clone();
            let form_body = build_stream_generate_body(&inner_req_list, at.as_deref());

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
            let text = response.text().await.map_err(Error::Request)?;
            if !status.is_success() {
                if is_attestation_error(&text) {
                    self.clear_conversation_state().await;
                }
                return Err(Error::api(status, text));
            }
            text
        };

        Ok(body)
    }

    /// Starts a streaming generation request.
    ///
    /// The returned [`reqwest::Response`] can be consumed as a stream of bytes;
    /// callers are responsible for parsing the WIZ frames.
    pub async fn stream_generate(
        &self,
        message: &ChatMessage,
        category: ModelCategory,
        config: Option<GenerationConfig>,
    ) -> Result<reqwest::Response> {
        self.ensure_session().await?;

        let prepared = prepare_request(None, message, config, category)?;
        let (inner_req_list, request_uuid, headers, cookie_header) = self
            .build_stream_generate_request(&prepared)
            .await?;

        let url = format!(
            "{WEB_BASE_URL}/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate"
        );

        let session = self.inner.session.lock().await;
        let mut params: Vec<(&str, String)> = vec![
            ("hl", session.language.clone()),
            ("_reqid", request_uuid),
            ("rt", "c".to_string()),
            ("pageId", "none".to_string()),
        ];
        if let Some(bl) = session.build_label.as_deref() {
            params.push(("bl", bl.to_string()));
        }
        if let Some(sid) = session.session_id.as_deref() {
            params.push(("f.sid", sid.to_string()));
        }
        let at = session.access_token.clone();
        drop(session);

        let form_body = build_stream_generate_body(&inner_req_list, at.as_deref());

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
        let (conversation_state, cookies, session_for_upload) = {
            let session = self.inner.session.lock().await;
            (
                session.conversation_state.clone(),
                session.cookies.clone(),
                session.clone(),
            )
        };

        let attachments = upload::upload_attachments(
            &self.inner.http,
            &cookies,
            &session_for_upload,
            prepared,
        )
        .await?;

        let proto_state = conversation_state.as_ref().map(map_proto_state);
        let inner_req_list = build_inner_req_list(
            prepared,
            proto_state.as_ref(),
            None,
            &attachments,
            &request_uuid,
        );

        let headers = Self::build_headers(Some(&request_uuid));
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

    async fn init_session(&self) -> Result<()> {
        let body = self.fetch_app_page().await?;

        let final_body = if let Some(save_url) = extract_consent_save_url(&body) {
            debug!("consent banner detected; acquiring SOCS cookie");
            self.accept_consent_and_refresh(&save_url).await?
        } else {
            body
        };

        let extracted = extract_from_app_html(&final_body);
        let mut session = self.inner.session.lock().await;
        session.access_token = extracted.access_token.or_else(|| session.access_token.clone());
        session.build_label = extracted.build_label.or_else(|| session.build_label.clone());
        session.session_id = extracted.session_id.or_else(|| session.session_id.clone());
        session.push_id = extracted.push_id.or_else(|| session.push_id.clone());
        Ok(())
    }

    async fn fetch_app_page(&self) -> Result<String> {
        let (language, cookie_header) = {
            let session = self.inner.session.lock().await;
            (session.language.clone(), session.cookies.to_header_value())
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
        let cookie_header = {
            let session = self.inner.session.lock().await;
            session.cookies.to_header_value()
        };

        let language = self.inner.config.language.clone();
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
            let mut session = self.inner.session.lock().await;
            session
                .cookies
                .merge_response_cookies(response.cookies());
        }

        self.fetch_app_page().await
    }

    fn build_headers(reqid: Option<&str>) -> Vec<(String, String)> {
        let mut headers = vec![
            (
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded;charset=UTF-8".to_string(),
            ),
            ("User-Agent".to_string(), USER_AGENT.to_string()),
            ("Origin".to_string(), WEB_BASE_URL.to_string()),
            ("Referer".to_string(), format!("{WEB_BASE_URL}/app")),
            ("X-Same-Domain".to_string(), "1".to_string()),
            ("Cache-Control".to_string(), "no-cache".to_string()),
            ("Pragma".to_string(), "no-cache".to_string()),
            (
                "sec-ch-ua".to_string(),
                "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"133\", \"Chromium\";v=\"133\"".to_string(),
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

fn is_attestation_error(body: &str) -> bool {
    body.contains("1096") || body.contains("BardErrorInfo") || body.contains("rs:108")
}

fn map_state(state: ProtoConversationState) -> crate::session::ConversationState {
    crate::session::ConversationState {
        conversation_id: state.conversation_id,
        response_id: state.response_id,
        response_part_id: state.response_part_id,
        continuation_token: state.continuation_token,
    }
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
        let response = self
            .client
            .generate(&message, self.category, self.config)
            .await?;

        if let Some(mut conversation) = self.conversation {
            conversation.add_message(message);
            conversation.add_model_text(response.text.clone());
        }

        Ok(response)
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
        let response = self
            .client
            .generate(&message, self.category, self.config)
            .await?;

        if let Some(mut conversation) = self.conversation {
            conversation.add_message(message);
            conversation.add_model_text(response.text.clone());
        }

        Ok(response)
    }
}
