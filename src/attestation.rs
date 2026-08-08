//! Browser attestation support using headless Chrome CDP.
//!
//! This module is only available when the `browser-attestation` feature is
//! enabled. It uses raw Chrome DevTools Protocol (CDP) over a local WebSocket to
//! capture valid `StreamGenerate` payloads from a real browser session.
//!
//! The captured payloads include the Google-signed attestation tokens required
//! for image uploads and true multi-turn conversation state.

use std::process::Stdio;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::process::{Child, Command};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::debug;

use crate::auth::{Cookies, Credentials};
use crate::errors::{Error, Result};

const NAVIGATE_TIMEOUT: Duration = Duration::from_secs(60);
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(60);

/// A handle to a headless Chrome process used for attestation.
pub struct BrowserAttestationClient {
    chrome_path: String,
    process: Option<Child>,
    ws_url: Option<String>,
}

impl Drop for BrowserAttestationClient {
    fn drop(&mut self) {
        // Best-effort kill; async runtime may already be shutting down.
        if let Some(mut child) = self.process.take() {
            let _ = child.start_kill();
        }
    }
}

impl BrowserAttestationClient {
    /// Creates a new attestation client for the given Chrome/Chromium executable.
    pub fn new(chrome_path: impl Into<String>) -> Self {
        Self {
            chrome_path: chrome_path.into(),
            process: None,
            ws_url: None,
        }
    }

    /// Launches Chrome and captures a fresh `StreamGenerate` payload for the
    /// given prompt.
    ///
    /// The returned 97-slot array can be replayed by the SDK, overriding only
    /// the prompt, category, and request UUID.
    pub async fn capture_payload(
        &mut self,
        credentials: &Credentials,
        prompt: &str,
    ) -> Result<Vec<Value>> {
        let cookies: Cookies = credentials.clone().into();
        self.ensure_browser().await?;
        let ws_url = self
            .ws_url
            .as_ref()
            .ok_or_else(|| Error::Attestation("browser WebSocket URL not available".to_string()))?;

        let (ws_stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| Error::Attestation(format!("failed to connect to CDP: {e}")))?;

        let (mut write, mut read) = ws_stream.split();

        // Enable required CDP domains.
        send_cdp(&mut write, "Runtime.enable", json!({})).await?;
        send_cdp(&mut write, "Network.enable", json!({})).await?;
        send_cdp(&mut write, "Page.enable", json!({})).await?;

        // Inject cookies before navigation so the page loads as an authenticated session.
        for (name, value) in cookies.iter() {
            send_cdp(
                &mut write,
                "Network.setCookie",
                json!({
                    "name": name,
                    "value": value,
                    "domain": ".google.com",
                    "path": "/",
                    "secure": true,
                }),
            )
            .await?;
        }

        // Navigate to Gemini /app.
        send_cdp(
            &mut write,
            "Page.navigate",
            json!({ "url": "https://gemini.google.com/app?hl=en" }),
        )
        .await?;

        // Wait for navigation to complete.
        wait_for_event(&mut read, "Page.loadEventFired", NAVIGATE_TIMEOUT).await?;

        // Inject the prompt and submit via JS.
        let escaped = prompt.replace('\\', "\\\\").replace('"', "\\\"");
        let js = format!(
            r#"
            const area = document.querySelector('textarea');
            if (!area) throw new Error('prompt textarea not found');
            area.value = "{}";
            area.dispatchEvent(new Event('input', {{ bubbles: true }}));
            const btn = document.querySelector('[data-test-id="send-button"]') || document.querySelector('button[aria-label*="Send"]');
            if (btn) btn.click();
            "#,
            escaped
        );
        send_cdp(&mut write, "Runtime.evaluate", json!({ "expression": js })).await?;

        // Wait for the StreamGenerate request.
        let post_data = wait_for_stream_generate_post_data(&mut read, CAPTURE_TIMEOUT).await?;

        // Extract f.req and parse the 97-slot array.
        let f_req = extract_f_req(&post_data)?;
        let inner = parse_inner_req_list(&f_req)?;

        Ok(inner)
    }

    async fn ensure_browser(&mut self) -> Result<()> {
        if self.process.is_some() {
            return Ok(());
        }

        let mut child = Command::new(&self.chrome_path)
            .arg("--headless=new")
            .arg("--no-sandbox")
            .arg("--disable-gpu")
            .arg("--disable-dev-shm-usage")
            .arg("--remote-debugging-port=0")
            .arg("--user-data-dir=/tmp/gemini-sdk-chrome-profile")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Attestation(format!("failed to launch Chrome: {e}")))?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::Attestation("failed to capture Chrome stderr".to_string()))?;

        let ws_url = read_devtools_url(stderr).await?;
        debug!(ws_url = %ws_url, "Chrome DevTools ready");

        self.process = Some(child);
        self.ws_url = Some(ws_url);
        Ok(())
    }

    /// Closes the browser process if it is running.
    pub async fn close(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill().await;
        }
        self.ws_url = None;
    }
}

async fn read_devtools_url(stderr: tokio::process::ChildStderr) -> Result<String> {
    use tokio::io::AsyncBufReadExt;
    let reader = tokio::io::BufReader::new(stderr);
    let mut lines = reader.lines();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while tokio::time::Instant::now() < deadline {
        let line = timeout(Duration::from_millis(500), lines.next_line())
            .await
            .map_err(|_| Error::Attestation("timeout reading Chrome stderr".to_string()))?
            .map_err(|e| Error::Attestation(format!("failed to read Chrome stderr: {e}")))?;

        if let Some(line) = line {
            if let Some(start) = line.find("ws://") {
                let url = &line[start..];
                if let Some(end) = url.find(' ') {
                    return Ok(url[..end].to_string());
                }
                return Ok(url.to_string());
            }
            if let Some(start) = line.find("DevTools listening on ") {
                return Ok(line[start + "DevTools listening on ".len()..].trim().to_string());
            }
        }
    }

    Err(Error::Attestation("timed out waiting for Chrome DevTools URL".to_string()))
}

async fn send_cdp<F>(write: &mut F, method: &str, params: Value) -> Result<()>
where
    F: futures::Sink<Message> + Unpin,
    F::Error: std::fmt::Debug,
{
    let message = json!({
        "id": rand::random::<u64>(),
        "method": method,
        "params": params,
    });
    write
        .send(Message::Text(message.to_string()))
        .await
        .map_err(|e| Error::Attestation(format!("failed to send CDP message: {e:?}")))?;
    Ok(())
}

async fn wait_for_event<S>(read: &mut S, event_name: &str, deadline: Duration) -> Result<Value>
where
    S: futures::Stream<Item = std::result::Result<Message, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    timeout(deadline, async {
        loop {
            match read.next().await {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(value) = serde_json::from_str::<Value>(&text) {
                        if value.get("method").and_then(|m| m.as_str()) == Some(event_name) {
                            return Ok(value);
                        }
                    }
                }
                Some(Ok(Message::Close(_))) | None => {
                    return Err(Error::Attestation("WebSocket closed unexpectedly".to_string()));
                }
                _ => {}
            }
        }
    })
    .await
    .map_err(|_| Error::Attestation(format!("timed out waiting for CDP event {event_name}")))?
}

async fn wait_for_stream_generate_post_data<S>(read: &mut S, _deadline: Duration) -> Result<String>
where
    S: futures::Stream<Item = std::result::Result<Message, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    loop {
        match read.next().await {
            Some(Ok(Message::Text(text))) => {
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    if let Some(params) = value.get("params") {
                        if let Some(request) = params.get("request") {
                            if request
                                .get("url")
                                .and_then(|u| u.as_str())
                                .map(|u| u.contains("StreamGenerate"))
                                .unwrap_or(false)
                            {
                                if let Some(post_data) =
                                    request.get("postData").and_then(|p| p.as_str())
                                {
                                    return Ok(post_data.to_string());
                                }
                            }
                        }
                    }
                }
            }
            Some(Ok(Message::Close(_))) | None => {
                return Err(Error::Attestation(
                    "WebSocket closed before StreamGenerate request was captured".to_string(),
                ));
            }
            _ => {}
        }
    }
}

fn extract_f_req(post_data: &str) -> Result<String> {
    for pair in post_data.split('&') {
        let mut it = pair.splitn(2, '=');
        if it.next() == Some("f.req") {
            if let Some(encoded) = it.next() {
                return urlencoding::decode(encoded)
                    .map(|s| s.into_owned())
                    .map_err(|e| Error::Attestation(format!("failed to decode f.req: {e}")));
            }
        }
    }
    Err(Error::Attestation("f.req not found in post data".to_string()))
}

fn parse_inner_req_list(f_req: &str) -> Result<Vec<Value>> {
    let outer: Value = serde_json::from_str(f_req)
        .map_err(|e| Error::Attestation(format!("failed to parse f.req JSON: {e}")))?;
    let inner_json = outer
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Attestation("f.req missing inner JSON".to_string()))?;
    serde_json::from_str(inner_json)
        .map_err(|e| Error::Attestation(format!("failed to parse inner request list: {e}")))
}

#[derive(Debug, Serialize, Deserialize)]
struct CdpMessage {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
}

impl Cookies {
    fn iter(&self) -> impl Iterator<Item = (String, String)> {
        let map: std::collections::HashMap<String, String> = self.clone().into();
        map.into_iter().map(|(k, v)| (k.to_string(), v.to_string()))
    }
}
