//! Internal helper: capture live fixtures used by the test suite.
//!
//! Run with:
//!
//! ```text
//! GEMINI_COOKIES="..." cargo run --example capture_fixtures
//! ```
//!
//! Captured files are written to `tests/fixtures/` and always overwrite prior
//! versions.  Cookies and secrets are redacted from the saved output.

use std::fs;
use std::path::PathBuf;

use gemini_sdk::{ChatMessage, GeminiClient, ImageSource, ModelCategory};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES required");
    let client = GeminiClient::from_cookie_header(&cookies)?;

    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
    fs::create_dir_all(&fixtures_dir)?;

    println!("Fetching model list...");
    let models = client.list_models().await?;
    println!("  {} models", models.len());

    println!("Fetching first-turn text response...");
    let turn1 = client
        .generate_raw(
            &ChatMessage::user("Hello, my name is Alice. Remember my name."),
            None,
            ModelCategory::Auto,
            None,
        )
        .await?;

    println!("Fetching 1100 image attestation error...");
    let mut image_msg = ChatMessage::user("Describe this image.");
    image_msg
        .parts
        .push(gemini_sdk::ContentPart::Image(ImageSource::from_bytes(
            "image/png",
            b"fake",
        )));
    let err1100 = client
        .generate_raw(&image_msg, None, ModelCategory::Auto, None)
        .await;

    println!("Fetching 1096 session error...");
    let err1096 = trigger_1096(&cookies).await;

    println!("Fetching /app HTML...");
    let app_html = fetch_app_html(&cookies).await?;

    fs::write(
        fixtures_dir.join("model_list_response.txt"),
        redact(&format_model_list(models)),
    )?;
    fs::write(
        fixtures_dir.join("turn1_response_raw.txt"),
        redact(&turn1),
    )?;
    fs::write(
        fixtures_dir.join("stream_generate_error_1100.json"),
        redact(&err1100.unwrap_or_else(|e| e.to_string())),
    )?;
    fs::write(
        fixtures_dir.join("stream_generate_error_1096.json"),
        redact(&err1096.unwrap_or_else(|e| e.to_string())),
    )?;
    fs::write(
        fixtures_dir.join("app_html_snippet.txt"),
        redact(&extract_wiz_snippet(&app_html).unwrap_or_else(|| {
            r#"window.WIZ_global_data = {"cfb2h":"boq_assistant-bard-web-server_20260804.05_p0","FdrFJe":"4202905934864668489","qKIAYe":"feeds/mcudyrk2a4khkz","KnDnFf":"feeds/other"};"#.to_string()
        })),
    )?;

    println!("Fixtures written to {}", fixtures_dir.display());
    Ok(())
}

fn format_model_list(models: Vec<gemini_sdk::ModelInfo>) -> String {
    // Reconstruct a plausible batchexecute shape from parsed models.
    // The test suite only needs to parse this via parse_model_list, so keep
    // the WIZ framing identical to a live response.
    let modes: Vec<String> = models
        .iter()
        .map(|m| {
            format!(
                "[\"{}\",\"{}\",\"{}\",null,null,null,null,null,null,null,null,\"{}\",null,null,null,null,null,{}]",
                m.id,
                m.title,
                m.description,
                m.versioned_name.as_deref().unwrap_or(&m.title),
                m.category_enum
            )
        })
        .collect();
    let inner = format!(
        "[[],[],[],[],[],[],[],[],[],[],[],[],[],[],[],[{}]]",
        modes.join(",")
    );
    format!(
        ")] }} '\n\n[[[\"wrb.fr\",\"otAQ7b\",null,{},null,null,null,\"generic\"]]]\n58\n[[\"di\",1]]\n",
        serde_json::to_string(&inner).unwrap()
    )
}

async fn trigger_1096(cookies: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Send a StreamGenerate request with an invalid f.sid to naturally produce
    // a BardErrorInfo 1096 / session error.
    let http = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;

    let reqid = ((std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        % 900_000)
        + 100_000)
        .to_string();

    let inner = build_minimal_inner_req_list("Hello");
    let f_req = serde_json::to_string(&[serde_json::Value::Null, serde_json::Value::Array(inner)]).unwrap();
    let body = format!("f.req={}", urlencoding::encode(&f_req));

    let url = "https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate";
    let resp = http
        .post(url)
        .query(&[
            ("hl", "en"),
            ("_reqid", reqid.as_str()),
            ("rt", "c"),
            ("pageId", "none"),
            ("f.sid", "invalid-session-id"),
        ])
        .header("Cookie", cookies)
        .header("Content-Type", "application/x-www-form-urlencoded;charset=UTF-8")
        .header("Origin", "https://gemini.google.com")
        .header("Referer", "https://gemini.google.com/app")
        .header("X-Same-Domain", "1")
        .body(body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(format!("HTTP {status}: {text}").into());
    }
    Ok(text)
}

async fn fetch_app_html(cookies: &str) -> Result<String, Box<dyn std::error::Error>> {
    let http = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;
    let resp = http
        .get("https://gemini.google.com/app?hl=en")
        .header("Cookie", cookies)
        .header("Accept", "text/html")
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(format!("HTTP {status}: {text}").into());
    }
    Ok(text)
}

fn extract_wiz_snippet(html: &str) -> Option<String> {
    let start_marker = "window.WIZ_global_data = ";
    let idx = html.find(start_marker)?;
    let brace = html[idx..].find('{')? + idx;
    let mut depth = 0i32;
    for (i, ch) in html[brace..].char_indices() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(html[idx..=brace + i].to_string());
            }
        }
    }
    None
}

fn build_minimal_inner_req_list(prompt: &str) -> Vec<serde_json::Value> {
    use serde_json::json;
    let mut slots: Vec<serde_json::Value> = vec![json!(null); 97];
    slots[0] = json!([prompt, 0, null, null, null, null, 0]);
    slots[1] = json!(["en"]);
    slots[2] = json!(["", "", "", null, null, null, null, null, null, ""]);
    slots[3] = json!("");
    slots[4] = json!("");
    slots[6] = json!([1]);
    slots[7] = json!(1);
    slots[10] = json!(1);
    slots[11] = json!(0);
    slots[17] = json!([[0]]);
    slots[18] = json!(0);
    slots[27] = json!(1);
    slots[30] = json!([1]);
    slots[41] = json!([2]);
    slots[53] = json!(0);
    slots[59] = json!(uuid::Uuid::new_v4().to_string().to_uppercase());
    slots[61] = json!([]);
    slots[66] = json!([
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        0
    ]);
    slots[68] = json!(1);
    slots[79] = json!(6);
    slots[91] = json!(0);
    slots[96] = json!(0);
    slots
}

fn redact(text: &str) -> String {
    let mut out = text.to_string();
    for (pat, repl) in [
        (r#""SNlM0e":"[^"]*""#, r#""SNlM0e":"REDACTED""#),
        (r#""FdrFJe":"[^"]*""#, r#""FdrFJe":"REDACTED""#),
        (r#""at":"[^"]*""#, r#""at":"REDACTED""#),
        (r#""cfb2h":"[^"]*""#, r#""cfb2h":"REDACTED""#),
        (r#""qKIAYe":"feeds/[^"]*""#, r#""qKIAYe":"feeds/REDACTED""#),
        (r#""KnDnFf":"feeds/[^"]*""#, r#""KnDnFf":"feeds/REDACTED""#),
    ] {
        let re = regex::Regex::new(pat).unwrap();
        out = re.replace_all(&out, repl).to_string();
    }
    out
}
