//! Tests for the upload progress stream API.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use gemini_sdk::{Cookies, GeminiClient, UploadEvent};

const COOKIE_HEADER: &str =
    "__Secure-1PSID=abc; __Secure-1PSIDCC=def; __Secure-1PAPISID=papi; SID=s; HSID=h; SSID=s";

/// A custom resolver that records how many times it is asked to resolve a name.
#[derive(Clone, Default)]
struct CountingResolver {
    calls: Arc<AtomicUsize>,
}

impl CountingResolver {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl reqwest::dns::Resolve for CountingResolver {
    fn resolve(&self, _name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(std::future::ready(Err(std::io::Error::other("resolver called").into())))
    }
}

#[tokio::test]
async fn upload_progress_yields_progress_before_network() {
    let resolver = CountingResolver::default();
    let http_client = reqwest::Client::builder()
        .dns_resolver(Arc::new(resolver.clone()))
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();

    let cookies = Cookies::from_header(COOKIE_HEADER);
    let client = GeminiClient::from_http_client(http_client, cookies).unwrap();

    let mut stream =
        client.upload_with_progress("image.png", "image/png", vec![1, 2, 3, 4, 5]).await;

    let first = stream.next().await.expect("stream should yield an event");
    let event = first.expect("first event should be Ok");
    match event {
        UploadEvent::Progress { uploaded, total } => {
            assert_eq!(uploaded, 0);
            assert_eq!(total, Some(5));
        }
        UploadEvent::Complete { .. } => {
            panic!("first event should be Progress, not Complete");
        }
        _ => panic!("unexpected event variant"),
    }

    // The injected client must eventually be used, which fails because the
    // resolver rejects every lookup.
    let _ = stream.next().await;
    assert!(resolver.calls() > 0, "injected client should reach the resolver");
}

#[tokio::test]
async fn upload_progress_reports_total_size() {
    let resolver = CountingResolver::default();
    let http_client = reqwest::Client::builder()
        .dns_resolver(Arc::new(resolver.clone()))
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();

    let cookies = Cookies::from_header(COOKIE_HEADER);
    let client = GeminiClient::from_http_client(http_client, cookies).unwrap();

    let bytes: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
    let mut stream = client.upload_with_progress("image.png", "image/png", bytes.clone()).await;

    let first = stream.next().await.unwrap().unwrap();
    match first {
        UploadEvent::Progress { uploaded, total } => {
            assert_eq!(uploaded, 0);
            assert_eq!(total, Some(bytes.len() as u64));
        }
        UploadEvent::Complete { .. } => panic!("first event should be Progress"),
        _ => panic!("unexpected event variant"),
    }
}

#[tokio::test]
async fn upload_progress_is_send() {
    let client = GeminiClient::from_cookie_header(COOKIE_HEADER).unwrap();
    let stream = client.upload_with_progress("image.png", "image/png", vec![1, 2, 3]).await;

    fn assert_send<T: Send>(_t: T) {}
    assert_send(stream);
}
