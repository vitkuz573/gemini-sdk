//! File upload support for the Gemini web frontend.
//!
//! Inline images are uploaded to `push.clients6.google.com` using the same
//! two-step resumable upload flow that the Gemini web UI uses.

use std::pin::Pin;

use futures::Stream;

use crate::auth::Cookies;
use crate::constants::headers as header_constants;
use crate::constants::mime;
use crate::constants::upload as upload_constants;
use crate::constants::urls::PUSH_UPLOAD_BASE_URL;
use crate::constants::user_agents::UPLOAD_BROWSER_LIKE;
use crate::errors::{Error, Result};
use crate::proto::slots::WebAttachment;
use crate::session::SessionState;

/// Events emitted by upload progress streams.
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
    base_url: &str,
) -> Result<String> {
    let (upload_url, push_id_str, cookie_header) =
        start_upload(client, cookies, session, filename, bytes.len(), base_url).await?;
    finalize_upload(client, &upload_url, &push_id_str, &cookie_header, mime_type, bytes, base_url)
        .await
}

/// Initiates a resumable upload and returns the upload URL, push id, and cookie header.
pub(crate) async fn start_upload(
    client: &reqwest::Client,
    cookies: &Cookies,
    session: &SessionState,
    filename: &str,
    total_bytes: usize,
    base_url: &str,
) -> Result<(String, String, String)> {
    let cookie_header = cookies.to_header_value();
    let push_id = session.effective_push_id();
    let push_id_str = push_id;

    let upload_endpoint = format!("{PUSH_UPLOAD_BASE_URL}{}", upload_constants::UPLOAD_PATH);
    let start_response = client
        .post(upload_endpoint)
        .header(upload_constants::X_GOOG_UPLOAD_COMMAND, upload_constants::UPLOAD_COMMAND_START)
        .header(upload_constants::X_GOOG_UPLOAD_HEADER_CONTENT_LENGTH, total_bytes.to_string())
        .header(upload_constants::X_GOOG_UPLOAD_PROTOCOL, upload_constants::RESUMABLE_PROTOCOL)
        .header(upload_constants::X_TENANT_ID, upload_constants::BARD_STORAGE_TENANT)
        .header(upload_constants::PUSH_ID_HEADER, &push_id_str)
        .header(header_constants::COOKIE, &cookie_header)
        .header(header_constants::ORIGIN, base_url)
        .header(header_constants::REFERER, format!("{base_url}/"))
        .header(header_constants::USER_AGENT, UPLOAD_BROWSER_LIKE)
        .header("sec-ch-ua", header_constants::SEC_CH_UA)
        .header("sec-ch-ua-mobile", header_constants::SEC_CH_UA_MOBILE)
        .header("sec-ch-ua-platform", header_constants::SEC_CH_UA_PLATFORM)
        .header("sec-fetch-dest", header_constants::SEC_FETCH_DEST)
        .header("sec-fetch-mode", header_constants::SEC_FETCH_MODE)
        .header(header_constants::SEC_FETCH_SITE, header_constants::SEC_FETCH_SITE_CROSS_SITE)
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
        .get(upload_constants::X_GOOG_UPLOAD_URL)
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
    base_url: &str,
) -> Result<String> {
    let finalize_response = client
        .post(upload_url)
        .header(upload_constants::X_GOOG_UPLOAD_COMMAND, upload_constants::UPLOAD_COMMAND_FINALIZE)
        .header("x-goog-upload-offset", "0")
        .header(upload_constants::X_TENANT_ID, upload_constants::BARD_STORAGE_TENANT)
        .header(upload_constants::PUSH_ID_HEADER, push_id)
        .header(header_constants::COOKIE, cookie_header)
        .header(header_constants::ORIGIN, base_url)
        .header(header_constants::REFERER, format!("{base_url}/"))
        .header(header_constants::USER_AGENT, UPLOAD_BROWSER_LIKE)
        .header("sec-ch-ua", header_constants::SEC_CH_UA)
        .header("sec-ch-ua-mobile", header_constants::SEC_CH_UA_MOBILE)
        .header("sec-ch-ua-platform", header_constants::SEC_CH_UA_PLATFORM)
        .header("sec-fetch-dest", header_constants::SEC_FETCH_DEST)
        .header("sec-fetch-mode", header_constants::SEC_FETCH_MODE)
        .header(header_constants::SEC_FETCH_SITE, header_constants::SEC_FETCH_SITE_CROSS_SITE)
        .header(header_constants::CONTENT_TYPE, mime_type)
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
    base_url: String,
) -> Pin<Box<dyn Stream<Item = Result<UploadEvent>> + Send>> {
    use async_stream::stream;

    Box::pin(stream! {
        let total = Some(bytes.len() as u64);
        yield Ok(UploadEvent::Progress { uploaded: 0, total });

        let (upload_url, push_id, cookie_header) =
            start_upload(&client, &cookies, &session, &filename, bytes.len(), &base_url).await?;

        yield Ok(UploadEvent::Progress { uploaded: 0, total });

        let reference = finalize_upload(
            &client,
            &upload_url,
            &push_id,
            &cookie_header,
            &mime_type,
            bytes,
            &base_url,
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
        mime::MP3
            | mime::MPEG_AUDIO
            | mime::WAV
            | mime::OGG_AUDIO
            | mime::MP4_VIDEO
            | mime::WEBM_VIDEO
            | mime::QUICKTIME
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
    base_url: &str,
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
            return Err(Error::bad_request(format!("unsupported media type: {mime_type}")));
        }
        let bytes = crate::proto::slots::base64_decode(data)?;
        let filename = crate::proto::slots::derive_attachment_filename(mime_type, idx);
        let reference =
            upload_file(client, cookies, session, &filename, mime_type, bytes, base_url).await?;
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
        assert!(is_allowed_media_type(mime::PNG));
        assert!(is_allowed_media_type(mime::JPEG));
        assert!(is_allowed_media_type(mime::MP3));
        assert!(is_allowed_media_type(mime::MPEG_AUDIO));
        assert!(is_allowed_media_type(mime::WAV));
        assert!(is_allowed_media_type(mime::OGG_AUDIO));
        assert!(is_allowed_media_type(mime::MP4_VIDEO));
        assert!(is_allowed_media_type(mime::WEBM_VIDEO));
        assert!(is_allowed_media_type(mime::QUICKTIME));
    }

    #[test]
    fn is_allowed_media_type_rejects_unknown() {
        assert!(!is_allowed_media_type("audio/flac"));
        assert!(!is_allowed_media_type("video/avi"));
        assert!(!is_allowed_media_type(mime::JSON));
        assert!(!is_allowed_media_type(mime::PLAIN_TEXT));
    }
}
