use gemini_sdk::attestation::BrowserAttestationClient;
use gemini_sdk::auth::Credentials;

#[tokio::main]
async fn main() -> gemini_sdk::Result<()> {
    tracing_subscriber::fmt::init();
    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES required");
    let chrome_path = std::env::var("CHROME_PATH").expect("CHROME_PATH required");
    let creds = Credentials::from_header(&cookies).unwrap();
    let mut client = BrowserAttestationClient::new(chrome_path);
    let payload = client.capture_payload(&creds, "hello, what is your name?").await?;
    println!("Captured {} slots", payload.len());
    println!("Slot 0: {}", serde_json::to_string(&payload[0]).unwrap());
    println!("Slot 3 length: {}", payload[3].as_str().unwrap_or("").len());
    println!("Slot 4: {:?}", payload[4]);
    client.close().await;
    Ok(())
}
