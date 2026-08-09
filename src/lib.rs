//! # Gemini SDK
//!
//! A clean, well-structured, production-ready Rust SDK for interacting with the
//! Google Gemini / Bard web frontend (`gemini.google.com`).
//!
//! The SDK provides:
//!
//! - Cookie-based authentication.
//! - Text-only and image (inline data / URL) chat completions.
//! - Streaming and non-streaming response handling.
//! - Model reasoning / thinking content extraction.
//! - Multi-turn conversation state.
//! - Model listing via `batchexecute` (`GetUserStatus` / `Fd0Qje`).
//! - File upload to `push.clients6.google.com`.
//! - Optional browser attestation using headless Chrome CDP (`browser-attestation`
//!   feature).
//! - Consent / `SOCS` cookie auto-acquisition.
//! - Proper error types, retry logic, and rate-limit handling.
//!
//! ## Quick start
//!
//! ```no_run
//! use gemini_sdk::GeminiClient;
//!
//! # async fn run() -> gemini_sdk::Result<()> {
//! let cookies = "__Secure-1PSID=YOUR_PSID; __Secure-1PSIDCC=YOUR_PSIDCC";
//!
//! let client = GeminiClient::from_cookie_header(cookies)?;
//!
//! let response = client
//!     .chat()
//!     .send_message("What is Rust?")
//!     .await?;
//!
//! println!("{}", response.text());
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//!
//! - `browser-attestation` — enables the headless-Chrome CDP attestation module
//!   required for image uploads and true multi-turn state.

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::return_self_not_must_use,
    clippy::doc_markdown,
    clippy::unused_async,
    clippy::unused_self,
    clippy::too_many_lines,
    clippy::manual_let_else,
    clippy::needless_continue,
    clippy::redundant_closure_for_method_calls,
    clippy::if_same_then_else,
    clippy::match_same_arms,
    clippy::implicit_hasher,
    clippy::elidable_lifetime_names,
    dead_code
)]

pub mod auth;
pub mod chat;
pub mod client;
pub mod errors;
pub mod models;
pub mod proto;
pub mod upload;

#[cfg(feature = "browser-attestation")]
pub mod attestation;

// Internal helpers kept private.
mod retry;
mod session;

pub use auth::{Cookies, CookieHeaderProvider, Credentials, CredentialsError, CredentialsProvider};
pub use chat::{
    ChatMessage, ChatResponse, ContentPart, Conversation, GenerationConfig, ImageSource,
    ThinkingLevel,
};
// PreparedRequest is intentionally public for benchmarks and advanced use.
pub use client::GeminiClient;
pub use errors::{Error, Result};
pub use models::{ModelCategory, ModelInfo};
// Re-export parsing helpers so consumers can convert streaming responses.
pub use proto::{
    extract_bard_error_code, extract_text_from_parsed_response,
    extract_thinking_from_parsed_response, parse_chat_response, parse_response_parts,
};
