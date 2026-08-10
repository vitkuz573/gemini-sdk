//! Live probe: exercise all v0.2 APIs plus base chat/list_models and emit JSON telemetry.
//!
//! This binary is intentionally self-contained. It reads real cookies from the
//! environment, optionally captures a redacted HAR file, and writes a JSON
//! report describing the outcome of every call.
//!
//! ```text
//! GEMINI_COOKIES="__Secure-1PSID=...; __Secure-1PSIDCC=..." \
//!   GEMINI_HAR_PATH=/tmp/gemini_probe.har \
//!   GEMINI_REPORT_PATH=/tmp/gemini_live_probe_report.json \
//!   cargo run --example live_probe
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use futures::StreamExt;
use gemini_sdk::{ChatMessage, GeminiClient, ModelCategory, TurnRating};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProbeReport {
    sdk_version: String,
    started_at: String,
    finished_at: String,
    base_url: String,
    summary: Summary,
    calls: Vec<ProbeCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Summary {
    total: usize,
    passed: usize,
    failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProbeCall {
    operation: String,
    duration_ms: u128,
    success: bool,
    error: String,
    retry_count: usize,
    http_status: Option<u16>,
    transient_400_detected: bool,
    /// `/app` diagnostics populated when sign-in verification fails.
    app_diagnostics: Option<AppDiagnosticsReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppDiagnosticsReport {
    signed_in: bool,
    gaia_id: Option<String>,
    email: Option<String>,
    failure_reason: Option<String>,
    missing_legacy_cookies: Vec<String>,
}

#[derive(Debug)]
struct ProbeState {
    calls: Mutex<Vec<ProbeCall>>,
    retry_count: AtomicUsize,
}

impl ProbeState {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            retry_count: AtomicUsize::new(0),
        }
    }

    fn take_retry_count(&self) -> usize {
        self.retry_count.swap(0, Ordering::SeqCst)
    }

    async fn push_call(&self, call: ProbeCall) {
        self.calls.lock().await.push(call);
    }

    async fn into_calls(self) -> Vec<ProbeCall> {
        self.calls.into_inner()
    }
}

#[derive(Debug)]
struct ProbeResult {
    success: bool,
    error: String,
    http_status: Option<u16>,
    transient_400_detected: bool,
    app_diagnostics: Option<AppDiagnosticsReport>,
}

impl ProbeResult {
    fn ok() -> Self {
        Self {
            success: true,
            error: String::new(),
            http_status: Some(200),
            transient_400_detected: false,
            app_diagnostics: None,
        }
    }

    fn err<E: std::fmt::Display>(err: E) -> Self {
        Self::err_with_diagnostics(err, None)
    }

    fn err_with_diagnostics<E: std::fmt::Display>(
        err: E,
        diagnostics: Option<AppDiagnosticsReport>,
    ) -> Self {
        let message = err.to_string();
        let transient_400_detected = message.contains("WIZ error frames");
        let http_status = if message.contains("(HTTP 400)") {
            Some(400)
        } else {
            None
        };
        Self {
            success: false,
            error: message,
            http_status,
            transient_400_detected,
            app_diagnostics: diagnostics,
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let started_at = humantime::format_rfc3339(std::time::SystemTime::now()).to_string();
    let cookies = std::env::var("GEMINI_COOKIES").unwrap_or_default();
    if cookies.is_empty() {
        eprintln!("GEMINI_COOKIES not set; live_probe requires real cookies");
        std::process::exit(1);
    }

    let base_url = std::env::var("GEMINI_BASE_URL")
        .unwrap_or_else(|_| "https://gemini.google.com".to_string());
    let har_path = std::env::var("GEMINI_HAR_PATH").ok();
    let report_path = std::env::var("GEMINI_REPORT_PATH")
        .unwrap_or_else(|_| "/tmp/gemini_live_probe_report.json".to_string());

    let mut client = match GeminiClient::from_cookie_header(&cookies) {
        Ok(c) => c.with_base_url(&base_url).await,
        Err(e) => {
            eprintln!("failed to build client: {e}");
            std::process::exit(1);
        }
    };

    if let Some(path) = har_path {
        client = match client.with_har_capture(path).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("failed to enable HAR capture: {e}");
                std::process::exit(1);
            }
        };
    }

    let state = Arc::new(ProbeState::new());

    // Independent calls.
    run_named_call(&client, &state, "verify_signed_in", || async {
        match client.diagnose_signed_in().await {
            Ok(diag) if diag.signed_in => ProbeResult::ok(),
            Ok(diag) => {
                let report = AppDiagnosticsReport {
                    signed_in: diag.signed_in,
                    gaia_id: diag.gaia_id,
                    email: diag.email,
                    failure_reason: diag.failure_reason,
                    missing_legacy_cookies: diag
                        .missing_legacy_cookies
                        .into_iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                };
                ProbeResult::err_with_diagnostics("not signed in", Some(report))
            }
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "list_models", || async {
        match client.list_models().await {
            Ok(_) => ProbeResult::ok(),
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "chat_send_message", || async {
        match client.chat().send_message("Say a one-sentence hello in English.").await {
            Ok(_) => ProbeResult::ok(),
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "generate_stream_first_chunk", || async {
        let message = ChatMessage::user("Say a one-sentence hello in English.");
        match client.generate_stream(&message, ModelCategory::Auto, None).await {
            Ok(mut stream) => {
                let mut seen = false;
                while let Some(chunk) = stream.next().await {
                    if chunk.is_ok() {
                        seen = true;
                        break;
                    }
                }
                if seen {
                    ProbeResult::ok()
                } else {
                    ProbeResult::err("no chunk received")
                }
            }
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "get_user_info", || async {
        match client.get_user_info().await {
            Ok(_) => ProbeResult::ok(),
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "get_last_selected_mode", || async {
        match client.get_last_selected_mode().await {
            Ok(_) => ProbeResult::ok(),
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    // set_last_selected_mode round-trip
    run_named_call(&client, &state, "set_last_selected_mode_round_trip", || async {
        match client.get_last_selected_mode().await {
            Ok(mode) => {
                let mode_id = mode.mode_id().unwrap_or("12345");
                match client.set_last_selected_mode(mode_id).await {
                    Ok(_) => ProbeResult::ok(),
                    Err(e) => ProbeResult::err(e),
                }
            }
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "get_locale_tools", || async {
        match client.get_locale_tools().await {
            Ok(_) => ProbeResult::ok(),
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "get_model_config", || async {
        match client.get_model_config().await {
            Ok(_) => ProbeResult::ok(),
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "get_locale_config", || async {
        match client.get_locale_config().await {
            Ok(_) => ProbeResult::ok(),
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "get_tools_config", || async {
        match client.get_tools_config().await {
            Ok(_) => ProbeResult::ok(),
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "get_usage_stats", || async {
        match client.get_usage_stats().await {
            Ok(_) => ProbeResult::ok(),
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    run_named_call(&client, &state, "get_scheduled_prompts", || async {
        match client.get_scheduled_prompts().await {
            Ok(_) => ProbeResult::ok(),
            Err(e) => ProbeResult::err(e),
        }
    })
    .await;

    // Conversation actions: create a turn, then regenerate, rate, delete.
    run_named_call(&client, &state, "conversation_actions", || async {
        let response =
            match client.chat().send_message("Say a one-sentence hello in English.").await {
                Ok(r) => r,
                Err(e) => return ProbeResult::err(e),
            };
        let conversation_id = match response.conversation_id() {
            Some(id) => id.to_string(),
            None => return ProbeResult::err("missing conversation_id in chat response"),
        };
        let response_id = match client.last_response_id().await {
            Some(id) => id,
            None => return ProbeResult::err("missing response_id after chat"),
        };

        if let Err(e) = client.regenerate_turn(&conversation_id, &response_id).await {
            return ProbeResult::err(e);
        }
        if let Err(e) = client.rate_turn(&conversation_id, &response_id, TurnRating::Good).await {
            return ProbeResult::err(e);
        }
        if let Err(e) = client.delete_turn(&conversation_id, &response_id).await {
            return ProbeResult::err(e);
        }
        ProbeResult::ok()
    })
    .await;

    let finished_at = humantime::format_rfc3339(std::time::SystemTime::now()).to_string();
    let calls = Arc::try_unwrap(state)
        .unwrap_or_else(|_| panic!("probe state still shared"))
        .into_calls()
        .await;

    let passed = calls.iter().filter(|c| c.success).count();
    let failed = calls.len() - passed;
    let report = ProbeReport {
        sdk_version: env!("CARGO_PKG_VERSION").to_string(),
        started_at,
        finished_at,
        base_url: base_url.clone(),
        summary: Summary {
            total: calls.len(),
            passed,
            failed,
        },
        calls,
    };

    let json = serde_json::to_string_pretty(&report).unwrap_or_else(|e| {
        eprintln!("failed to serialize report: {e}");
        std::process::exit(1);
    });

    if let Err(e) = tokio::fs::write(&report_path, json).await {
        eprintln!("failed to write report to {report_path}: {e}");
        std::process::exit(1);
    }

    if failed > 0 {
        eprintln!("live_probe failed: {failed}/{} calls failed", report.summary.total);
        for call in &report.calls {
            if !call.success {
                eprintln!("  - {}: {}", call.operation, call.error);
            }
        }
        std::process::exit(1);
    }

    println!(
        "live_probe complete: {}/{} calls passed; report written to {report_path}",
        report.summary.passed, report.summary.total
    );
}

async fn run_named_call<F, Fut>(
    _client: &GeminiClient,
    state: &Arc<ProbeState>,
    name: &str,
    operation: F,
) where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = ProbeResult> + Send,
{
    let call = run_call(state, operation).await;
    state.push_call(call.named(name)).await;
}

async fn run_call<F, Fut>(state: &Arc<ProbeState>, operation: F) -> ProbeCall
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = ProbeResult> + Send,
{
    let state = Arc::clone(state);
    let start = Instant::now();
    let result = operation().await;
    ProbeCall {
        operation: String::new(),
        duration_ms: start.elapsed().as_millis(),
        success: result.success,
        error: result.error,
        retry_count: state.take_retry_count(),
        http_status: result.http_status,
        transient_400_detected: result.transient_400_detected,
        app_diagnostics: result.app_diagnostics,
    }
}

impl ProbeCall {
    fn named(mut self, operation: &str) -> Self {
        self.operation = operation.to_string();
        self
    }
}
