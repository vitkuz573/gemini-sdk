//! Optional W3C HAR 1.2 capture for request/response auditing.
//!
//! HAR capture is opt-in via `GeminiClient::with_har_capture`. Every HTTP
//! transaction is written as a HAR entry to the configured file path. All
//! cookies, authorization headers, and `x-goog-ext-*` values are
//! redacted before writing so captured audit files do not contain recoverable
//! secrets.

use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(test)]
use reqwest::header::HeaderName;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};

use crate::errors::{Error, Result};

/// Writer for W3C HAR 1.2 audit files.
///
/// The writer stores captured entries in memory and flushes the full document
/// to disk after every entry. This limits data loss on panic or crash at the
/// cost of higher I/O; HAR files are expected to remain small for SDK audit
/// use cases.
#[derive(Debug)]
pub struct HarWriter {
    path: PathBuf,
    entries: Vec<Value>,
}

impl HarWriter {
    /// Creates a new HAR writer that will write to `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the parent directory cannot be created or if the
    /// file cannot be opened for writing.
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Config(format!("failed to create HAR directory: {e}")))?;
        }
        // Touch the file early so permission problems surface during client
        // construction rather than mid-request.
        tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .await
            .map_err(|e| Error::Config(format!("failed to open HAR file: {e}")))?;

        Ok(Self { path, entries: Vec::new() })
    }

    /// Records a single HTTP transaction.
    ///
    /// Request and response bodies are passed as already-read byte vectors so
    /// the live request/response streams are not consumed. All secret-bearing
    /// fields are redacted before serialization.
    ///
    /// # Errors
    ///
    /// Returns an error if the HAR document cannot be serialized or written.
    #[allow(clippy::too_many_arguments)]
    pub async fn record(
        &mut self,
        method: &str,
        url: &str,
        request_headers: &HeaderMap,
        request_body: &[u8],
        status: u16,
        response_headers: &HeaderMap,
        response_body: &[u8],
        duration: Duration,
    ) -> Result<()> {
        let started = std::time::SystemTime::now();
        let started_iso = humantime::format_rfc3339_millis(started).to_string();
        let time_ms = duration.as_millis().max(1) as u64;

        let request_entry = json!({
            "method": method,
            "url": url,
            "httpVersion": "HTTP/1.1",
            "headers": redact_headers(request_headers),
            "cookies": redact_cookies_from_header(request_headers),
            "queryString": [],
            "postData": redact_post_data(request_body),
            "headersSize": -1,
            "bodySize": request_body.len() as i64,
        });

        let response_entry = json!({
            "status": status,
            "statusText": "",
            "httpVersion": "HTTP/1.1",
            "headers": redact_headers(response_headers),
            "cookies": redact_cookies_from_header(response_headers),
            "content": {
                "size": response_body.len() as i64,
                "mimeType": "application/json",
                "text": String::from_utf8_lossy(response_body),
            },
            "redirectURL": "",
            "headersSize": -1,
            "bodySize": response_body.len() as i64,
        });

        let entry = json!({
            "startedDateTime": started_iso,
            "time": time_ms,
            "request": request_entry,
            "response": response_entry,
            "cache": {},
            "timings": {
                "send": 0,
                "wait": time_ms,
                "receive": 0,
            },
        });

        self.entries.push(entry);
        self.flush().await
    }

    async fn flush(&self) -> Result<()> {
        let doc = json!({
            "log": {
                "version": "1.2",
                "creator": {
                    "name": "gemini-sdk",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "entries": self.entries,
            }
        });

        let bytes = serde_json::to_vec_pretty(&doc).map_err(Error::Json)?;
        tokio::fs::write(&self.path, bytes)
            .await
            .map_err(|e| Error::Config(format!("failed to write HAR file: {e}")))?;
        Ok(())
    }
}

fn redact_headers(headers: &HeaderMap) -> Vec<Value> {
    headers
        .iter()
        .map(|(name, value)| {
            let name_str = name.as_str();
            let redacted = is_secret_header(name_str);
            json!({
                "name": name_str,
                "value": if redacted { "<redacted>".to_string() } else { value_to_string(value) },
            })
        })
        .collect()
}

fn is_secret_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "cookie"
        || lower == "authorization"
        || lower == "set-cookie"
        || lower.starts_with("x-goog-ext-")
}

fn value_to_string(value: &HeaderValue) -> String {
    value.to_str().unwrap_or("<binary>").to_string()
}

fn redact_cookies_from_header(headers: &HeaderMap) -> Vec<Value> {
    let mut result = Vec::new();
    for (name, value) in headers {
        if name.as_str().eq_ignore_ascii_case("cookie")
            || name.as_str().eq_ignore_ascii_case("set-cookie")
        {
            if let Ok(text) = value.to_str() {
                for cookie in text.split(';') {
                    let trimmed = cookie.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let mut parts = trimmed.splitn(2, '=');
                    let cookie_name = parts.next().unwrap_or("").trim();
                    result.push(json!({
                        "name": cookie_name,
                        "value": "<redacted>",
                    }));
                }
            }
        }
    }
    result
}

fn redact_post_data(body: &[u8]) -> Value {
    let text = String::from_utf8_lossy(body);
    let redacted = redact_cookie_like_substrings(&text);
    json!({
        "mimeType": "application/x-www-form-urlencoded;charset=UTF-8",
        "text": redacted,
    })
}

fn redact_cookie_like_substrings(text: &str) -> String {
    // Conservative regex-free scan: replace anything that looks like a
    // `name=value` pair inside form data where the name smells like a Google
    // auth cookie or bearer token.
    let mut out = text.to_string();
    let patterns: &[&str] = &[
        "__Secure-1PSID",
        "__Secure-1PSIDCC",
        "__Secure-3PSID",
        "__Secure-3PSIDCC",
        "SAPISID",
        "APISID",
        "SSID",
        "HSID",
        "SID",
        "SOCS",
        "authorization=Bearer ",
        "access_token=",
    ];
    for pattern in patterns {
        out = redact_after_pattern(&out, pattern);
    }
    out
}

fn redact_after_pattern(text: &str, pattern: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut start = 0;
    while let Some(idx) = text[start..].find(pattern) {
        let abs = start + idx;
        result.push_str(&text[start..abs]);
        result.push_str(pattern);
        // The value starts after the pattern; skip an optional '=' or ' '
        // separator, then consume until the next '&' or end of string.
        let rest = &text[abs + pattern.len()..];
        let (skip, end) = if rest.starts_with('=') || rest.starts_with(' ') {
            let sep_len = if rest.starts_with('=') { 1 } else { 1 };
            let end = rest[sep_len..].find('&').map_or(rest.len() - sep_len, |i| i);
            (sep_len, end)
        } else {
            let end = rest.find('&').map_or(rest.len(), |i| i);
            (0, end)
        };
        if end > 0 {
            result.push_str("=<redacted>");
        }
        start = abs + pattern.len() + skip + end;
    }
    result.push_str(&text[start..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_map(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        map
    }

    #[tokio::test]
    async fn redacts_cookie_header_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.har");
        let mut writer = HarWriter::new(&path).await.unwrap();

        let headers = header_map(&[("Cookie", "__Secure-1PSID=a; __Secure-1PSIDCC=b; SAPISID=c")]);
        writer
            .record(
                "POST",
                "https://gemini.google.com/_/BardChatUi/data/batchexecute",
                &headers,
                b"f.req=[[1]]",
                200,
                &HeaderMap::new(),
                b"ok",
                Duration::from_millis(50),
            )
            .await
            .unwrap();

        let text = std::fs::read_to_string(path).unwrap();
        assert!(!text.contains("__Secure-1PSID=a"));
        assert!(text.contains("<redacted>"));
        assert!(text.contains("__Secure-1PSID"));
    }

    #[tokio::test]
    async fn redacts_authorization_and_x_goog_ext_headers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.har");
        let mut writer = HarWriter::new(&path).await.unwrap();

        let headers = header_map(&[
            ("Authorization", "Bearer secret-token"),
            ("x-goog-ext-525001261-jspb", "[1,null,\"secret\"]"),
        ]);
        writer
            .record(
                "POST",
                "https://gemini.google.com/",
                &headers,
                b"",
                200,
                &HeaderMap::new(),
                b"ok",
                Duration::from_millis(10),
            )
            .await
            .unwrap();

        let text = std::fs::read_to_string(path).unwrap();
        assert!(!text.contains("secret-token"));
        assert!(!text.contains("[1,null,\"secret\"]"));
    }

    #[test]
    fn redacts_cookie_like_substrings_in_body() {
        let text = "f.req=[[1]]&__Secure-1PSID=abc123&SAPISID=xyz";
        let redacted = redact_cookie_like_substrings(text);
        assert!(redacted.contains("__Secure-1PSID=<redacted>"), "redacted: {redacted}");
        assert!(redacted.contains("SAPISID=<redacted>"), "redacted: {redacted}");
        assert!(!redacted.contains("abc123"), "redacted: {redacted}");
        assert!(!redacted.contains("xyz"), "redacted: {redacted}");
    }

    #[tokio::test]
    async fn har_entry_shape_matches_spec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shape.har");
        let mut writer = HarWriter::new(&path).await.unwrap();

        writer
            .record(
                "POST",
                "https://gemini.google.com/_/BardChatUi/data/batchexecute",
                &HeaderMap::new(),
                b"f.req=[[1]]",
                400,
                &HeaderMap::new(),
                b"[[\"er\",\"di\",null,\"af.httprm\"]]",
                Duration::from_millis(123),
            )
            .await
            .unwrap();

        let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let log = doc.get("log").unwrap();
        assert_eq!(log.get("version").unwrap(), "1.2");
        assert_eq!(log.get("creator").unwrap().get("name").unwrap(), "gemini-sdk");
        let entries = log.get("entries").unwrap().as_array().unwrap();
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert!(entry.get("startedDateTime").is_some());
        assert_eq!(entry.get("time").unwrap(), 123);
        assert!(entry.get("request").unwrap().get("method").is_some());
        assert_eq!(entry.get("response").unwrap().get("status").unwrap(), 400);
        assert!(entry.get("timings").unwrap().get("wait").is_some());
    }
}
