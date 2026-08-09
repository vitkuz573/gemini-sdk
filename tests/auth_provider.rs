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
