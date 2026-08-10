//! Tests for injecting a custom `reqwest::Client`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use gemini_sdk::{Cookies, GeminiClient};

const COOKIE_HEADER: &str = "__Secure-1PSID=abc; __Secure-1PSIDCC=def; __Secure-1PAPISID=papi; SID=s; HSID=h; SSID=s";

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
async fn injected_client_is_stored() {
    let resolver = CountingResolver::default();
    let http_client = reqwest::Client::builder()
        .dns_resolver(Arc::new(resolver.clone()))
        .timeout(std::time::Duration::from_secs(1))
        .build()
        .unwrap();

    let cookies = Cookies::from_header(COOKIE_HEADER);
    let client = GeminiClient::from_http_client(http_client, cookies).unwrap();

    // The SDK should use the injected client, so even a network failure
    // proves the resolver (and therefore the client) was reached.
    let _ = client.verify_signed_in().await;
    assert!(
        resolver.calls() > 0,
        "injected client was not used for the request"
    );
}

#[test]
fn from_http_client_rejects_missing_cookies() {
    let http_client = reqwest::Client::new();
    let cookies = Cookies::from_header("__Secure-1PSID=only");
    let result = GeminiClient::from_http_client(http_client, cookies);
    assert!(result.is_err());
}
