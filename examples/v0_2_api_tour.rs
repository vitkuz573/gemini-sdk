//! Example: read-only tour of the v0.2 API expansion surfaces.
//!
//! This example demonstrates the new Gemini SDK v0.2 methods added across
//! Phases 7-10: conversation actions, user profile, locale/model/tool config,
//! and settings pages. It is intentionally read-only so it can be safely run
//! against a live environment with valid cookies.
//!
//! A configurable base URL lets you point the example at a mock server or
//! a different Gemini frontend host:
//!
//! ```text
//! GEMINI_COOKIES="__Secure-1PSID=...; __Secure-1PSIDCC=..." \
//!   GEMINI_BASE_URL="https://gemini.google.com" \
//!   cargo run --example v0_2_api_tour
//! ```

use gemini_sdk::GeminiClient;

#[tokio::main]
async fn main() -> gemini_sdk::Result<()> {
    tracing_subscriber::fmt::init();

    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES env var required");
    let base_url = std::env::var("GEMINI_BASE_URL")
        .unwrap_or_else(|_| "https://gemini.google.com".to_string());

    let client = GeminiClient::from_cookie_header(&cookies)?.with_base_url(&base_url).await;

    // User profile (Phase 8)
    match client.get_user_info().await {
        Ok(info) => {
            println!("user info:");
            println!("  name: {}", info.name().unwrap_or("<none>"));
            println!("  email: {}", info.email().unwrap_or("<none>"));
        }
        Err(e) => eprintln!("get_user_info failed: {e}"),
    }

    // Last selected mode preference (Phase 8)
    match client.get_last_selected_mode().await {
        Ok(mode) => {
            println!("last selected mode: {}", mode.mode_id().unwrap_or("<none>"));
        }
        Err(e) => eprintln!("get_last_selected_mode failed: {e}"),
    }

    // Locale, model, and tools configuration (Phase 9)
    match client.get_locale_tools().await {
        Ok(v) => println!("locale tools: {}", v.value()),
        Err(e) => eprintln!("get_locale_tools failed: {e}"),
    }

    match client.get_locale_config().await {
        Ok(v) => println!("locale config: {}", v.value()),
        Err(e) => eprintln!("get_locale_config failed: {e}"),
    }

    match client.get_model_config().await {
        Ok(v) => println!("model config: {}", v.value()),
        Err(e) => eprintln!("get_model_config failed: {e}"),
    }

    match client.get_tools_config().await {
        Ok(v) => println!("tools config: {}", v.value()),
        Err(e) => eprintln!("get_tools_config failed: {e}"),
    }

    // Settings pages (Phase 10)
    match client.get_usage_stats().await {
        Ok(v) => println!("usage stats: {}", v.value()),
        Err(e) => eprintln!("get_usage_stats failed: {e}"),
    }

    match client.get_scheduled_prompts().await {
        Ok(v) => println!("scheduled prompts: {}", v.value()),
        Err(e) => eprintln!("get_scheduled_prompts failed: {e}"),
    }

    // Conversation actions are mutating, so only demonstrate the client surface
    // without calling them live. See the integration tests for mocked usage of
    // regenerate_turn, rate_turn, and delete_turn.
    println!("v0.2 API tour complete");

    Ok(())
}
