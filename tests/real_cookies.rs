//! Integration tests against the live Gemini web frontend.
//!
//! These tests require real cookies in `/tmp/opencode/gemini_cookies.env`.
//! They skip gracefully when credentials are missing.

use std::path::PathBuf;

use base64::Engine;
use futures::StreamExt;
use gemini_sdk::{ChatMessage, GeminiClient, ImageSource, ModelCategory, TurnRating};

fn load_env() {
    let path = PathBuf::from("/tmp/opencode/gemini_cookies.env");
    if path.exists() {
        let _ = dotenvy::from_path(&path);
    }
}

fn cookies() -> Option<String> {
    load_env();
    std::env::var("GEMINI_COOKIES").ok().filter(|s| !s.is_empty())
}

fn push_id() -> Option<String> {
    std::env::var("GEMINI_PUSH_ID").ok().filter(|s| !s.is_empty())
}

#[tokio::test]
async fn list_models_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping list_models_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let models = client.list_models().await.expect("list_models should succeed");
    assert!(!models.is_empty(), "expected at least one model");
    for model in &models {
        assert!(!model.id().is_empty());
        assert!(!model.title().is_empty());
    }
}

#[tokio::test]
async fn chat_send_message_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping chat_send_message_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let response = client
        .chat()
        .send_message("Say a one-sentence hello in English.")
        .await
        .expect("send_message should succeed");
    assert!(!response.text().is_empty(), "expected non-empty response text");
}

#[tokio::test]
async fn stream_generate_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping stream_generate_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let message = ChatMessage::user("Say a one-sentence hello in English.");
    let response = client
        .stream_generate(&message, ModelCategory::Auto, None)
        .await
        .expect("stream_generate should succeed");

    let mut stream = response.bytes_stream();
    let mut seen = false;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("chunk should be valid");
        if !chunk.is_empty() {
            seen = true;
        }
    }
    assert!(seen, "expected at least one non-empty chunk");
}

#[tokio::test]
async fn upload_image_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping upload_image_works: GEMINI_COOKIES not set");
        return;
    };
    if push_id().is_none() {
        eprintln!("skipping upload_image_works: GEMINI_PUSH_ID not set");
        return;
    }

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    // A minimal 1x1 transparent PNG.
    let png = base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==")
        .expect("valid png");
    let image = ImageSource::from_bytes("image/png", &png);
    let response = client
        .chat()
        .send_message_with_images("Describe this image in one sentence.", vec![image])
        .await
        .expect("image chat should succeed");
    assert!(!response.text().is_empty(), "expected non-empty response text");
}

#[tokio::test]
async fn get_user_info_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping get_user_info_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let info = client
        .get_user_info()
        .await
        .expect("get_user_info should succeed");
    // At least one profile field should be present in a signed-in session.
    assert!(
        info.name().is_some() || info.email().is_some() || info.photo_url().is_some(),
        "expected at least one user info field"
    );
}

#[tokio::test]
async fn get_last_selected_mode_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping get_last_selected_mode_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    client
        .get_last_selected_mode()
        .await
        .expect("get_last_selected_mode should succeed");
}

#[tokio::test]
async fn set_last_selected_mode_round_trips() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping set_last_selected_mode_round_trips: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let original = client
        .get_last_selected_mode()
        .await
        .expect("get_last_selected_mode should succeed");
    let mode_id = original.mode_id().unwrap_or("12345");
    client
        .set_last_selected_mode(mode_id)
        .await
        .expect("set_last_selected_mode should succeed");
}

#[tokio::test]
async fn get_locale_tools_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping get_locale_tools_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let result = client
        .get_locale_tools()
        .await
        .expect("get_locale_tools should succeed");
    assert!(!result.value().is_null());
}

#[tokio::test]
async fn get_model_config_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping get_model_config_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let result = client
        .get_model_config()
        .await
        .expect("get_model_config should succeed");
    assert!(!result.value().is_null());
}

#[tokio::test]
async fn get_locale_config_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping get_locale_config_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let result = client
        .get_locale_config()
        .await
        .expect("get_locale_config should succeed");
    assert!(!result.value().is_null());
}

#[tokio::test]
async fn get_tools_config_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping get_tools_config_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let result = client
        .get_tools_config()
        .await
        .expect("get_tools_config should succeed");
    assert!(!result.value().is_null());
}

#[tokio::test]
async fn get_usage_stats_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping get_usage_stats_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let result = client
        .get_usage_stats()
        .await
        .expect("get_usage_stats should succeed");
    assert!(!result.value().is_null());
}

#[tokio::test]
async fn get_scheduled_prompts_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping get_scheduled_prompts_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let result = client
        .get_scheduled_prompts()
        .await
        .expect("get_scheduled_prompts should succeed");
    assert!(!result.value().is_null());
}

#[tokio::test]
async fn conversation_actions_works() {
    let Some(cookies) = cookies() else {
        eprintln!("skipping conversation_actions_works: GEMINI_COOKIES not set");
        return;
    };

    let client = GeminiClient::from_cookie_header(&cookies).expect("valid client");
    let response = client
        .chat()
        .send_message("Say a one-sentence hello in English.")
        .await
        .expect("send_message should succeed");
    let conversation_id = response
        .conversation_id()
        .expect("conversation_id should be present")
        .to_string();
    let response_id = client
        .last_response_id()
        .await
        .expect("response_id should be present");

    client
        .regenerate_turn(&conversation_id, &response_id)
        .await
        .expect("regenerate_turn should succeed");
    client
        .rate_turn(&conversation_id, &response_id, TurnRating::Good)
        .await
        .expect("rate_turn should succeed");
    client
        .delete_turn(&conversation_id, &response_id)
        .await
        .expect("delete_turn should succeed");
}
