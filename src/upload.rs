//! File upload support for the Gemini web frontend.
//!
//! Inline images are uploaded to `push.clients6.google.com` using the same
//! two-step resumable upload flow that the Gemini web UI uses.

use crate::auth::Cookies;
use crate::errors::{Error, Result};
use crate::proto::slots::WebAttachment;
use crate::session::SessionState;

const PUSH_UPLOAD_URL: &str = "https://push.clients6.google.com/upload/";
const WEB_BASE_URL: &str = "https://gemini.google.com";
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

/// Uploads a file to Google's resumable upload endpoint.
pub(crate) async fn upload_file(
    client: &reqwest::Client,
    cookies: &Cookies,
    session: &SessionState,
    filename: &str,
    mime_type: &str,
    bytes: Vec<u8>,
) -> Result<String> {
    let cookie_header = cookies.to_header_value();
    let push_id = session.effective_push_id();
    let push_id_str = push_id.as_str();

    // Step 1: initiate the resumable upload.
    let start_response = client
        .post(PUSH_UPLOAD_URL)
        .header("x-goog-upload-command", "start")
        .header("x-goog-upload-header-content-length", bytes.len().to_string())
        .header("x-goog-upload-protocol", "resumable")
        .header("x-tenant-id", "bard-storage")
        .header("push-id", push_id_str)
        .header("Cookie", &cookie_header)
        .header("Origin", WEB_BASE_URL)
        .header("Referer", format!("{WEB_BASE_URL}/"))
        .header("User-Agent", USER_AGENT)
        .header("sec-ch-ua", "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"133\", \"Chromium\";v=\"133\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "cross-site")
        .body(format!("File name: {filename}"))
        .send()
        .await
        .map_err(|e| Error::Transient(format!("failed to start file upload: {e}")))?;

    let status = start_response.status();
    if !status.is_success() {
        let body = start_response.text().await.unwrap_or_default();
        return Err(Error::api(status, format!("file upload start failed: {body}")));
    }

    let upload_url = start_response
        .headers()
        .get("x-goog-upload-url")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Error::parse("file upload start response missing X-Goog-Upload-URL"))?;

    // Step 2: upload the bytes and finalize.
    let finalize_response = client
        .post(upload_url)
        .header("x-goog-upload-command", "upload, finalize")
        .header("x-goog-upload-offset", "0")
        .header("x-tenant-id", "bard-storage")
        .header("push-id", push_id_str)
        .header("Cookie", &cookie_header)
        .header("Origin", WEB_BASE_URL)
        .header("Referer", format!("{WEB_BASE_URL}/"))
        .header("User-Agent", USER_AGENT)
        .header("sec-ch-ua", "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"133\", \"Chromium\";v=\"133\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "cross-site")
        .header("Content-Type", mime_type)
        .body(bytes)
        .send()
        .await
        .map_err(|e| Error::Transient(format!("failed to finalize file upload: {e}")))?;

    let status = finalize_response.status();
    if !status.is_success() {
        let body = finalize_response.text().await.unwrap_or_default();
        return Err(Error::api(status, format!("file upload finalize failed: {body}")));
    }

    let reference = finalize_response
        .text()
        .await
        .map_err(|e| Error::parse(format!("failed to read upload response: {e}")))?
        .trim()
        .to_string();

    if reference.is_empty() {
        return Err(Error::parse("file upload returned empty reference"));
    }

    Ok(reference)
}

/// Uploads all inline images found in a prepared request and returns attachment
/// descriptors.
pub(crate) async fn upload_attachments(
    client: &reqwest::Client,
    cookies: &Cookies,
    session: &SessionState,
    prepared: &crate::chat::PreparedRequest,
) -> Result<Vec<WebAttachment>> {
    let mut attachments = Vec::with_capacity(prepared.inline_images.len());
    for (idx, (mime_type, data)) in prepared.inline_images.iter().enumerate() {
        let bytes = crate::proto::slots::base64_decode(data)?;
        let filename = crate::proto::slots::derive_attachment_filename(mime_type, idx);
        let reference = upload_file(client, cookies, session, &filename, mime_type, bytes).await?;
        attachments.push(WebAttachment {
            reference,
            mime_type: mime_type.clone(),
            filename,
        });
    }
    Ok(attachments)
}
