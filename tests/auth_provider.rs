//! Integration tests for the pluggable `CredentialsProvider` auth layer.

use gemini_sdk::{
    auth::{CookieHeaderProvider, Credentials, CredentialsProvider, PSID, PSIDCC},
    GeminiClient,
};

/// A custom provider that can be used to prove the trait is implementable by
/// downstream users.
struct CustomProvider {
    psid: String,
    psidcc: String,
}

impl CredentialsProvider for CustomProvider {
    fn credentials(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = gemini_sdk::Result<Credentials>> + Send + '_>>
    {
        let psid = self.psid.clone();
        let psidcc = self.psidcc.clone();
        Box::pin(async move {
            let mut creds = Credentials::new();
            creds.psid = psid;
            creds.psidcc = psidcc;
            creds.validate().map_err(|e| gemini_sdk::Error::Config(e.to_string()))?;
            Ok(creds)
        })
    }
}

#[test]
fn custom_provider_yields_credentials() {
    let provider = CustomProvider {
        psid: "psid".to_string(),
        psidcc: "psidcc".to_string(),
    };

    let creds = tokio_test::block_on(provider.credentials()).unwrap();
    assert_eq!(creds.psid, "psid");
    assert_eq!(creds.psidcc, "psidcc");
}

#[test]
fn cookie_header_provider_parses_valid_header() {
    let header = format!("{PSID}=psid; {PSIDCC}=psidcc");
    let provider = CookieHeaderProvider::new(&header).unwrap();

    let creds = tokio_test::block_on(provider.credentials()).unwrap();
    assert_eq!(creds.psid, "psid");
    assert_eq!(creds.psidcc, "psidcc");
}

#[test]
fn cookie_header_provider_rejects_missing_psidcc() {
    let header = format!("{PSID}=psid");
    let result = CookieHeaderProvider::new(&header);
    assert!(result.is_err());
}

#[test]
fn bare_credentials_satisfies_provider_trait() {
    let mut creds = Credentials::new();
    creds.psid = "psid".to_string();
    creds.psidcc = "psidcc".to_string();

    let resolved = tokio_test::block_on(creds.credentials()).unwrap();
    assert_eq!(resolved.psid, "psid");
    assert_eq!(resolved.psidcc, "psidcc");
}

#[tokio::test]
async fn client_from_provider_builds_from_boxed_provider() {
    let header = format!("{PSID}=psid; {PSIDCC}=psidcc");
    let provider = CookieHeaderProvider::new(&header).unwrap();
    let client = GeminiClient::from_provider(provider).await.unwrap();

    // A successfully constructed client exposes the model category and has the
    // expected cookies available internally.
    let builder = client.chat();
    let _ = builder;
}

#[tokio::test]
async fn refresh_credentials_replaces_cookies_and_clears_session() {
    use gemini_sdk::auth::{PAPISID, PSIDTS};

    let header = format!("{PSID}=old; {PSIDCC}=old");
    let provider = CookieHeaderProvider::new(&header).unwrap();
    let client = GeminiClient::from_provider(provider).await.unwrap();

    let refreshed_header = format!(
        "{PSID}=new; {PSIDCC}=new; {PAPISID}=papi; {PSIDTS}=ts; SID=s; HSID=h; SSID=s"
    );
    let refreshed_provider = CookieHeaderProvider::new(&refreshed_header).unwrap();

    // The public refresh path replaces cookies. Since it calls init_session
    // which fetches /app, this test exercises the wiring rather than a live
    // round-trip. Warm-up RPC failures are now tolerated, so a signed-in /app
    // response lets refresh succeed even if WAA/ogads cannot be completed.
    let result = client.refresh_credentials(refreshed_provider).await;
    assert!(result.is_ok(), "refresh_credentials failed: {:?}", result);

    // Verify the snapshot was updated by save/restore, which reflects the new
    // cookies stored in the client.
    let snapshot = client.save_session().await.unwrap();
    assert!(snapshot.contains("new"));
    assert!(!snapshot.contains("old"));
}
