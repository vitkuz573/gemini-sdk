//! Tests that `tracing` spans are created and do not leak secrets.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use gemini_sdk::{ChatMessage, ChatResponse, GeminiClient, HttpHook, ModelCategory};
use tracing::span::Attributes;
use tracing::Id;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::Registry;

const COOKIE_HEADER: &str = "__Secure-1PSID=abc; __Secure-1PSIDCC=def; __Secure-1PAPISID=papi; SID=s; HSID=h; SSID=s";

#[derive(Clone, Default)]
struct SpanCapture {
    names: Arc<Mutex<Vec<String>>>,
    field_keys: Arc<Mutex<Vec<String>>>,
}

impl<S> Layer<S> for SpanCapture
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut visitor = KeyVisitor::default();
        attrs.record(&mut visitor);
        self.field_keys.lock().unwrap().extend(visitor.keys);

        if let Some(span) = ctx.span(id) {
            self.names.lock().unwrap().push(span.name().to_string());
        }
    }
}

#[derive(Default)]
struct KeyVisitor {
    keys: Vec<String>,
}

impl tracing::field::Visit for KeyVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {
        self.keys.push(field.name().to_string());
    }
}

async fn with_capture<T>(
    capture: SpanCapture,
    f: impl Future<Output = T>,
) -> (T, Vec<String>, Vec<String>) {
    let subscriber = Registry::default().with(capture.clone());
    let _guard = tracing::subscriber::set_default(subscriber);
    let result = f.await;
    let names = capture.names.lock().unwrap().clone();
    let keys = capture.field_keys.lock().unwrap().clone();
    (result, names, keys)
}

#[derive(Debug, Default)]
struct ObservingHook {
    request_count: AtomicUsize,
}

impl HttpHook for ObservingHook {
    fn on_request<'a>(
        &'a self,
        _request: &'a gemini_sdk::PreparedRequest,
    ) -> Pin<Box<dyn Future<Output = gemini_sdk::Result<()>> + Send + 'a>> {
        self.request_count.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { Ok(()) })
    }

    fn on_response<'a>(
        &'a self,
        _response: &'a ChatResponse,
    ) -> Pin<Box<dyn Future<Output = gemini_sdk::Result<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }
}

#[tokio::test]
async fn generate_stream_creates_span_without_prompt() {
    let capture = SpanCapture::default();
    let client = GeminiClient::from_cookie_header(COOKIE_HEADER)
        .unwrap()
        .with_http_hook(ObservingHook::default())
        .await;
    let message = ChatMessage::user("What is Rust?");

    let (_result, names, keys) = with_capture(capture, async {
        let _ = client.generate_stream(&message, ModelCategory::Auto, None).await;
    })
    .await;

    assert!(
        names.iter().any(|n| n == "gemini.generate_stream"),
        "expected gemini.generate_stream span, got {names:?}"
    );

    assert!(
        !keys.iter().any(|k| k == "prompt"),
        "span fields must not contain a 'prompt' key: {keys:?}"
    );
}

#[tokio::test]
async fn generate_creates_span() {
    let capture = SpanCapture::default();
    let client = GeminiClient::from_cookie_header(COOKIE_HEADER)
        .unwrap()
        .with_http_hook(ObservingHook::default())
        .await;
    let message = ChatMessage::user("hello");

    let (_result, names, _keys) = with_capture(capture, async {
        let _ = client.generate(&message, ModelCategory::Auto, None).await;
    })
    .await;

    assert!(
        names.iter().any(|n| n == "gemini.generate"),
        "expected gemini.generate span, got {names:?}"
    );
}

#[tokio::test]
async fn list_models_creates_span() {
    let capture = SpanCapture::default();
    let client = GeminiClient::from_cookie_header(COOKIE_HEADER)
        .unwrap()
        .with_http_hook(ObservingHook::default())
        .await;

    let (_result, names, _keys) = with_capture(capture, async {
        let _ = client.list_models().await;
    })
    .await;

    assert!(
        names.iter().any(|n| n == "gemini.list_models"),
        "expected gemini.list_models span, got {names:?}"
    );
}

#[tokio::test]
async fn verify_signed_in_creates_span() {
    let capture = SpanCapture::default();
    let client = GeminiClient::from_cookie_header(COOKIE_HEADER)
        .unwrap()
        .with_http_hook(ObservingHook::default())
        .await;

    let (_result, names, _keys) = with_capture(capture, async {
        let _ = client.verify_signed_in().await;
    })
    .await;

    assert!(
        names.iter().any(|n| n == "gemini.verify_signed_in"),
        "expected gemini.verify_signed_in span, got {names:?}"
    );
}

#[tokio::test]
async fn span_fields_exclude_message_content() {
    let capture = SpanCapture::default();
    let client = GeminiClient::from_cookie_header(COOKIE_HEADER)
        .unwrap()
        .with_http_hook(ObservingHook::default())
        .await;
    let message = ChatMessage::user("secret prompt text");

    let (_result, _names, keys) = with_capture(capture, async {
        let _ = client.generate(&message, ModelCategory::Auto, None).await;
    })
    .await;

    assert!(
        !keys.iter().any(|k| k == "message"),
        "span fields must not contain a 'message' key: {keys:?}"
    );
}
