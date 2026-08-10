//! File upload support for the Gemini web frontend.
//!
//! Inline images are uploaded to `push.clients6.google.com` using the same
//! two-step resumable upload flow that the Gemini web UI uses.

use std::pin::Pin;

use futures::Stream;

use crate::auth::Cookies;
use crate::errors::{Error, Result};
use crate::proto::slots::WebAttachment;
use crate::session::SessionState;

const PUSH_UPLOAD_URL: &str = "https://push.clients6.google.com/upload/";
const WEB_BASE_URL: &str = "https://gemini.google.com";
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

/// Events emitted by [`GeminiClient::upload_with_progress`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum UploadEvent {
    /// Upload progress: bytes sent so far and total size if known.
    Progress {
        /// Bytes uploaded so far.
        uploaded: u64,
        /// Total bytes to upload, if known.
        total: Option<u64>,
    },
    /// Upload completed successfully.
    Complete {
        /// The attachment descriptor returned by the server.
        attachment: WebAttachment,
    },
}

/// Uploads a file to Google's resumable upload endpoint.
#[tracing::instrument(level = "debug", skip_all, fields(operation = "gemini.upload_file", bytes = bytes.len()))]
pub(crate) async fn upload_file(
    client: &reqwest::Client,
    cookies: &Cookies,
    session: &SessionState,
    filename: &str,
    mime_type: &str,
    bytes: Vec<u8>,
) -> Result<String> {
    let (upload_url, push_id_str, cookie_header) =
        start_upload(client, cookies, session, filename, bytes.len()).await?;
    finalize_upload(
        client,
        &upload_url,
        &push_id_str,
        &cookie_header,
        mime_type,
        bytes,
    )
    .await
}

/// Initiates a resumable upload and returns the upload URL, push id, and cookie header.
pub(crate) async fn start_upload(
    client: &reqwest::Client,
    cookies: &Cookies,
    session: &SessionState,
    filename: &str,
    total_bytes: usize,
) -> Result<(String, String, String)> {
    let cookie_header = cookies.to_header_value();
    let push_id = session.effective_push_id();
    let push_id_str = push_id;

    let start_response = client
        .post(PUSH_UPLOAD_URL)
        .header("x-goog-upload-command", "start")
        .header("x-goog-upload-header-content-length", total_bytes.to_string())
        .header("x-goog-upload-protocol", "resumable")
        .header("x-tenant-id", "bard-storage")
        .header("push-id", &push_id_str)
        .header("Cookie", &cookie_header)
        .header("Origin", WEB_BASE_URL)
        .header("Referer", format!("{WEB_BASE_URL}/"))
        .header("User-Agent", USER_AGENT)
        .header(
            "sec-ch-ua",
            "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"133\", \"Chromium\";v=\"133\"",
        )
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
        .ok_or_else(|| Error::parse("file upload start response missing X-Goog-Upload-URL"))?
        .to_string();

    let parsed = reqwest::Url::parse(&upload_url)
        .map_err(|e| Error::parse(format!("invalid upload URL: {e}")))?;
    if parsed.scheme() != "https"
        || !matches!(parsed.host_str(), Some(host) if host.ends_with(".google.com"))
    {
        return Err(Error::parse("upload URL has untrusted origin"));
    }

    Ok((upload_url, push_id_str, cookie_header))
}

/// Uploads the bytes and finalizes a resumable upload, returning the attachment reference.
pub(crate) async fn finalize_upload(
    client: &reqwest::Client,
    upload_url: &str,
    push_id: &str,
    cookie_header: &str,
    mime_type: &str,
    bytes: Vec<u8>,
) -> Result<String> {
    let finalize_response = client
        .post(upload_url)
        .header("x-goog-upload-command", "upload, finalize")
        .header("x-goog-upload-offset", "0")
        .header("x-tenant-id", "bard-storage")
        .header("push-id", push_id)
        .header("Cookie", cookie_header)
        .header("Origin", WEB_BASE_URL)
        .header("Referer", format!("{WEB_BASE_URL}/"))
        .header("User-Agent", USER_AGENT)
        .header(
            "sec-ch-ua",
            "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"133\", \"Chromium\";v=\"133\"",
        )
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

/// Returns a stream of upload progress events for a single file.
pub(crate) fn upload_progress_stream(
    client: reqwest::Client,
    cookies: Cookies,
    session: SessionState,
    filename: String,
    mime_type: String,
    bytes: Vec<u8>,
) -> Pin<Box<dyn Stream<Item = Result<UploadEvent>> + Send>> {
    use async_stream::stream;

    Box::pin(stream! {
        let total = Some(bytes.len() as u64);
        yield Ok(UploadEvent::Progress { uploaded: 0, total });

        let (upload_url, push_id, cookie_header) =
            start_upload(&client, &cookies, &session, &filename, bytes.len()).await?;

        yield Ok(UploadEvent::Progress { uploaded: 0, total });

        let reference = finalize_upload(
            &client,
            &upload_url,
            &push_id,
            &cookie_header,
            &mime_type,
            bytes,
        )
        .await?;

        yield Ok(UploadEvent::Complete {
            attachment: WebAttachment {
                reference,
                mime_type: mime_type.clone(),
                filename: filename.clone(),
            },
        });
    })
}

fn is_allowed_media_type(mime_type: &str) -> bool {
    let clean = mime_type.split(';').next().unwrap_or(mime_type);
    if let Some(kind) = clean.split('/').next() {
        if kind == "image" {
            return true;
        }
    }
    matches!(
        clean,
        "audio/mp3"
            | "audio/mpeg"
            | "audio/wav"
            | "audio/ogg"
            | "video/mp4"
            | "video/webm"
            | "video/quicktime"
    )
}

/// Uploads all inline attachments found in a prepared request.
///
/// Images, audio, and video are uploaded through the same resumable upload
/// endpoint. Unsupported MIME types are rejected before any network call.
pub(crate) async fn upload_attachments(
    client: &reqwest::Client,
    cookies: &Cookies,
    session: &SessionState,
    prepared: &crate::chat::PreparedRequest,
) -> Result<Vec<WebAttachment>> {
    let total = prepared
        .inline_images
        .len()
        .saturating_add(prepared.inline_audio.len())
        .saturating_add(prepared.inline_video.len());
    let mut attachments = Vec::with_capacity(total);

    for (idx, (mime_type, data)) in prepared
        .inline_images
        .iter()
        .chain(prepared.inline_audio.iter())
        .chain(prepared.inline_video.iter())
        .enumerate()
    {
        if !is_allowed_media_type(mime_type) {
            return Err(Error::bad_request(format!(
                "unsupported media type: {mime_type}"
            )));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_allowed_media_type_accepts_images_audio_video() {
        assert!(is_allowed_media_type("image/png"));
        assert!(is_allowed_media_type("image/jpeg"));
        assert!(is_allowed_media_type("audio/mp3"));
        assert!(is_allowed_media_type("audio/mpeg"));
        assert!(is_allowed_media_type("audio/wav"));
        assert!(is_allowed_media_type("audio/ogg"));
        assert!(is_allowed_media_type("video/mp4"));
        assert!(is_allowed_media_type("video/webm"));
        assert!(is_allowed_media_type("video/quicktime"));
    }

    #[test]
    fn is_allowed_media_type_rejects_unknown() {
        assert!(!is_allowed_media_type("audio/flac"));
        assert!(!is_allowed_media_type("video/avi"));
        assert!(!is_allowed_media_type("application/json"));
        assert!(!is_allowed_media_type("text/plain"));
    }
}
