//! High-level chat types for building requests and reading responses.

use std::sync::Arc;

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::{Error, Result};
use crate::models::ModelCategory;
use crate::tool::{Tool, ToolCall, ToolResult};

/// Current snapshot format version for forward compatibility.
pub(crate) const CONVERSATION_FORMAT_VERSION: u32 = 1;

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatMessage {
    /// Role of the message author: `user` or `model`.
    pub role: String,
    /// Content parts that make up this message.
    ///
    /// This field is public to allow low-level construction, but callers are
    /// responsible for keeping roles and part types consistent with what the
    /// Gemini web frontend expects. Malformed messages will fail at send time.
    pub parts: Vec<ContentPart>,
}

impl ChatMessage {
    /// Creates a plain text message.
    pub fn text(role: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            parts: vec![ContentPart::Text(text.into())],
        }
    }

    /// Creates a user message with the given text.
    pub fn user(text: impl Into<String>) -> Self {
        Self::text(crate::constants::roles::USER, text)
    }

    /// Creates a model message with the given text.
    pub fn model(text: impl Into<String>) -> Self {
        Self::text(crate::constants::roles::MODEL, text)
    }

    /// Creates a user message containing an image.
    pub fn with_image(role: impl Into<String>, source: ImageSource) -> Self {
        Self {
            role: role.into(),
            parts: vec![ContentPart::Image(source)],
        }
    }

    /// Creates a message containing audio.
    pub fn with_audio(role: impl Into<String>, source: AudioSource) -> Self {
        Self {
            role: role.into(),
            parts: vec![ContentPart::Audio(source)],
        }
    }

    /// Creates a message containing video.
    pub fn with_video(role: impl Into<String>, source: VideoSource) -> Self {
        Self {
            role: role.into(),
            parts: vec![ContentPart::Video(source)],
        }
    }

    /// Appends a content part to this message.
    pub fn with_part(mut self, part: ContentPart) -> Self {
        self.parts.push(part);
        self
    }
}

/// A source for image content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageSource {
    /// Base64-encoded inline image data together with its MIME type.
    InlineData {
        /// MIME type of the image, e.g. `image/png`.
        mime_type: String,
        /// Base64-encoded bytes.
        data: String,
    },
    /// A publicly reachable image URL.
    Url {
        /// URL of the image.
        url: String,
    },
}

/// A source for audio content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioSource {
    /// Base64-encoded inline audio data together with its MIME type.
    InlineData {
        /// MIME type of the audio, e.g. `audio/mp3`.
        mime_type: String,
        /// Base64-encoded bytes.
        data: String,
    },
    /// A publicly reachable audio URL.
    Url {
        /// URL of the audio.
        url: String,
    },
}

impl AudioSource {
    /// Creates an inline audio source from raw bytes, base64-encoding them.
    pub fn from_bytes(mime_type: impl Into<String>, bytes: &[u8]) -> Self {
        Self::InlineData {
            mime_type: mime_type.into(),
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
        }
    }

    /// Returns the MIME type of this audio source.
    #[must_use]
    pub fn mime_type(&self) -> Option<&str> {
        match self {
            Self::InlineData { mime_type, .. } => Some(mime_type.as_str()),
            Self::Url { .. } => None,
        }
    }
}

/// A source for video content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoSource {
    /// Base64-encoded inline image data together with its MIME type.
    InlineData {
        /// MIME type of the image, e.g. `image/png`.
        mime_type: String,
        /// Base64-encoded bytes.
        data: String,
    },
    /// A publicly reachable image URL.
    Url {
        /// URL of the image.
        url: String,
    },
}

impl ImageSource {
    /// Creates an inline image source from raw bytes, base64-encoding them.
    pub fn from_bytes(mime_type: impl Into<String>, bytes: &[u8]) -> Self {
        Self::InlineData {
            mime_type: mime_type.into(),
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
        }
    }

    /// Returns the MIME type of this image source.
    #[must_use]
    pub fn mime_type(&self) -> Option<&str> {
        match self {
            Self::InlineData { mime_type, .. } => Some(mime_type.as_str()),
            Self::Url { .. } => None,
        }
    }
}

impl VideoSource {
    /// Creates an inline video source from raw bytes, base64-encoding them.
    pub fn from_bytes(mime_type: impl Into<String>, bytes: &[u8]) -> Self {
        Self::InlineData {
            mime_type: mime_type.into(),
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
        }
    }

    /// Returns the MIME type of this video source.
    #[must_use]
    pub fn mime_type(&self) -> Option<&str> {
        match self {
            Self::InlineData { mime_type, .. } => Some(mime_type.as_str()),
            Self::Url { .. } => None,
        }
    }
}

/// A part of a chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentPart {
    /// Plain text.
    Text(String),
    /// Model reasoning / thinking text (only present in responses).
    Thinking(String),
    /// An image.
    Image(ImageSource),
    /// An audio attachment.
    Audio(AudioSource),
    /// A video attachment.
    Video(VideoSource),
    /// A function-call request produced by the model.
    ToolCall(ToolCall),
    /// A function-call result returned to the model.
    ToolResult(ToolResult),
}

/// Configuration for a generation request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    /// Sampling temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Nucleus sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Desired thinking level (when supported).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<ThinkingLevel>,
    /// Optional system instruction prepended to the user prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<String>,
    /// Optional tool declarations registered for function calling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    /// Maximum number of tool-call turns before returning the last response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tool_turns: Option<usize>,
}

impl GenerationConfig {
    /// Sets the system instruction for this generation config.
    pub fn with_system_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.system_instruction = Some(instruction.into());
        self
    }

    /// Sets the tool declarations for this generation config.
    ///
    /// Each entry should be a JSON object with a `name` and `parameters` field,
    /// typically produced by [`crate::tool::tool_declaration`].
    pub fn with_tools(mut self, tools: Vec<Value>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Sets the maximum number of tool-call turns allowed.
    pub fn with_max_tool_turns(mut self, max: usize) -> Self {
        self.max_tool_turns = Some(max);
        self
    }
}

/// Thinking level requested for a generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ThinkingLevel {
    /// No explicit thinking level.
    None,
    /// Standard thinking.
    Standard,
    /// Extended thinking.
    Extended,
    /// Deep think.
    DeepThink,
}

impl ThinkingLevel {
    /// Returns the numeric enum value used in `StreamGenerate` slot 80.
    #[must_use]
    pub fn as_enum_value(self) -> Option<u64> {
        match self {
            Self::None => None,
            Self::Standard => Some(1),
            Self::Extended => Some(2),
            Self::DeepThink => Some(3),
        }
    }
}

/// A structured response from a chat completion.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ChatResponse {
    /// Text content returned by the model.
    pub(crate) text: String,
    /// Model reasoning / thinking content (empty when the model does not
    /// expose its reasoning, e.g. for models without thinking enabled).
    pub(crate) thinking: String,
    /// Conversation id extracted from the response state, if available.
    pub(crate) conversation_id: Option<String>,
}

impl ChatResponse {
    /// Creates a response from a single text string.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// Returns a reference to the response text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns a reference to the model's reasoning / thinking content.
    #[must_use]
    pub fn thinking(&self) -> &str {
        &self.thinking
    }

    /// Sets the model's reasoning / thinking content.
    pub fn with_thinking(mut self, thinking: impl Into<String>) -> Self {
        self.thinking = thinking.into();
        self
    }

    /// Sets the conversation id extracted from the response state.
    pub fn with_conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self
    }

    /// Sets an optional conversation id.
    pub(crate) fn with_conversation_id_opt(mut self, conversation_id: Option<String>) -> Self {
        self.conversation_id = conversation_id;
        self
    }

    /// Returns the conversation id extracted from the response state, if any.
    ///
    /// This is a best-effort accessor intended for integration tests and the
    /// live probe example. It returns `None` if the response did not contain
    /// parseable conversation state.
    #[must_use]
    pub fn conversation_id(&self) -> Option<&str> {
        self.conversation_id.as_deref()
    }
}

/// Internal helper: extracts the latest user prompt from a message.
///
/// The SDK does not flatten multi-turn history; callers that need history in
/// the prompt should build it themselves or use a higher-level wrapper.
pub(crate) fn extract_prompt(message: &ChatMessage) -> Result<String> {
    let mut text_parts = Vec::new();
    for part in &message.parts {
        match part {
            ContentPart::Text(t) => text_parts.push(t.as_str()),
            ContentPart::Thinking(_)
            | ContentPart::Image(_)
            | ContentPart::Audio(_)
            | ContentPart::Video(_)
            | ContentPart::ToolCall(_)
            | ContentPart::ToolResult(_) => {}
        }
    }
    let prompt = text_parts.join("\n");
    if prompt.is_empty() {
        return Err(Error::bad_request("prompt is empty"));
    }
    Ok(prompt)
}

/// Serializable representation of a [`Conversation`] for snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConversationSnapshot {
    format_version: u32,
    messages: Vec<ChatMessage>,
    model_category: Option<ModelCategory>,
}

/// An in-progress conversation that carries multi-turn state.
///
/// `messages` is public as a low-level escape hatch, but callers that mutate it
/// directly are responsible for keeping roles and part types valid. Malformed
/// conversations will fail at send time when `extract_prompt` validates the
/// outgoing message.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Conversation {
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) model_category: Option<ModelCategory>,
}

impl Conversation {
    /// Creates a new empty conversation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the model category used for all turns in this conversation.
    pub fn with_model_category(mut self, category: ModelCategory) -> Self {
        self.model_category = Some(category);
        self
    }

    /// Adds a message to the conversation history.
    pub fn add_message(&mut self, message: ChatMessage) -> &mut Self {
        self.messages.push(message);
        self
    }

    /// Adds a user text turn.
    pub fn add_user_text(&mut self, text: impl Into<String>) -> &mut Self {
        self.messages.push(ChatMessage::user(text));
        self
    }

    /// Adds a model text turn.
    pub fn add_model_text(&mut self, text: impl Into<String>) -> &mut Self {
        self.messages.push(ChatMessage::model(text));
        self
    }

    /// Returns an immutable view of the messages.
    #[must_use]
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Returns the model category selected for this conversation, if any.
    #[must_use]
    pub fn model_category(&self) -> Option<ModelCategory> {
        self.model_category
    }

    /// Serialises this conversation to a JSON snapshot.
    ///
    /// The snapshot includes the message history and model category. It does not
    /// contain credentials or other client state.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversation cannot be serialised to JSON.
    pub fn save(&self) -> Result<String> {
        let snapshot = ConversationSnapshot {
            format_version: CONVERSATION_FORMAT_VERSION,
            messages: self.messages.clone(),
            model_category: self.model_category,
        };
        serde_json::to_string(&snapshot).map_err(Error::Json)
    }

    /// Restores a conversation from a JSON snapshot created by [`save`][Self::save].
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot is not valid JSON or does not match the
    /// expected conversation shape.
    pub fn restore(snapshot: &str) -> Result<Self> {
        let parsed: ConversationSnapshot = serde_json::from_str(snapshot).map_err(Error::Json)?;
        Ok(Self {
            messages: parsed.messages,
            model_category: parsed.model_category,
        })
    }
}

/// Internal type used when preparing a generation request.
///
/// This type is exposed for benchmarks, hooks, and advanced use. Fields are
/// public but not covered by the semver stability guarantees of the primary
/// public API.
#[derive(Clone)]
#[doc(hidden)]
pub struct PreparedRequest {
    /// Flattened prompt text.
    pub prompt: String,
    /// Inline images as `(mime_type, base64_data)` pairs.
    pub inline_images: Vec<(String, String)>,
    /// Inline audio as `(mime_type, base64_data)` pairs.
    pub inline_audio: Vec<(String, String)>,
    /// Inline video as `(mime_type, base64_data)` pairs.
    pub inline_video: Vec<(String, String)>,
    /// Generation configuration.
    pub config: Option<GenerationConfig>,
    /// Selected model category.
    pub category: ModelCategory,
    /// Registered tool declarations for function calling.
    pub tools: Option<Vec<Arc<dyn Tool>>>,
    /// Whether to retry once on `NotSignedIn` errors (Plan 03 wiring).
    pub refresh_on_auth_error: bool,
}

impl std::fmt::Debug for PreparedRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedRequest")
            .field("prompt", &self.prompt)
            .field("inline_images", &self.inline_images.len())
            .field("inline_audio", &self.inline_audio.len())
            .field("inline_video", &self.inline_video.len())
            .field("config", &self.config)
            .field("category", &self.category)
            .field("tools", &self.tools.as_ref().map(|t| t.len()))
            .field("refresh_on_auth_error", &self.refresh_on_auth_error)
            .finish()
    }
}

/// Internal helper that prepares a request from a conversation or a single turn.
pub(crate) fn prepare_request(
    _conversation: Option<&Conversation>,
    new_message: &ChatMessage,
    config: Option<GenerationConfig>,
    default_category: ModelCategory,
) -> Result<PreparedRequest> {
    let prompt = extract_prompt(new_message)?;

    let mut inline_images = Vec::new();
    let mut inline_audio = Vec::new();
    let mut inline_video = Vec::new();
    for part in &new_message.parts {
        match part {
            ContentPart::Image(ImageSource::InlineData { mime_type, data }) => {
                inline_images.push((mime_type.clone(), data.clone()));
            }
            ContentPart::Image(ImageSource::Url { url }) => {
                return Err(Error::bad_request(format!(
                    "image URLs are not supported directly by the web frontend: {url}"
                )));
            }
            ContentPart::Audio(AudioSource::InlineData { mime_type, data }) => {
                inline_audio.push((mime_type.clone(), data.clone()));
            }
            ContentPart::Audio(AudioSource::Url { url }) => {
                return Err(Error::bad_request(format!(
                    "audio URLs are not supported directly by the web frontend: {url}"
                )));
            }
            ContentPart::Video(VideoSource::InlineData { mime_type, data }) => {
                inline_video.push((mime_type.clone(), data.clone()));
            }
            ContentPart::Video(VideoSource::Url { url }) => {
                return Err(Error::bad_request(format!(
                    "video URLs are not supported directly by the web frontend: {url}"
                )));
            }
            ContentPart::Text(_) | ContentPart::Thinking(_) => {}
            ContentPart::ToolCall(_) | ContentPart::ToolResult(_) => {}
        }
    }

    Ok(PreparedRequest {
        prompt,
        inline_images,
        inline_audio,
        inline_video,
        config,
        category: default_category,
        tools: None,
        refresh_on_auth_error: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_prompt_from_text_message() {
        let message = ChatMessage::user("Hello");
        assert_eq!(extract_prompt(&message).unwrap(), "Hello");
    }

    #[test]
    fn extract_prompt_rejects_empty() {
        let message = ChatMessage::user("");
        assert!(extract_prompt(&message).is_err());
    }

    #[test]
    fn prepare_request_extracts_inline_images() {
        let mut message = ChatMessage::user("Look at this");
        message
            .parts
            .push(ContentPart::Image(ImageSource::from_bytes("image/png", b"fake")));

        let prepared = prepare_request(None, &message, None, ModelCategory::Auto).unwrap();
        assert_eq!(prepared.inline_images.len(), 1);
        assert_eq!(prepared.prompt, "Look at this");
    }

    #[test]
    fn prepare_request_extracts_inline_audio_and_video() {
        let mut message = ChatMessage::user("Listen and watch");
        message
            .parts
            .push(ContentPart::Audio(AudioSource::from_bytes("audio/mp3", b"fake-audio")));
        message
            .parts
            .push(ContentPart::Video(VideoSource::from_bytes("video/mp4", b"fake-video")));

        let prepared = prepare_request(None, &message, None, ModelCategory::Auto).unwrap();
        assert_eq!(prepared.inline_audio.len(), 1);
        assert_eq!(prepared.inline_video.len(), 1);
        assert_eq!(prepared.prompt, "Listen and watch");
    }

    #[test]
    fn prepare_request_rejects_image_url() {
        let message = ChatMessage::with_image(
            "user",
            ImageSource::Url {
                url: "https://example.com/x.png".to_string(),
            },
        );
        let result = prepare_request(None, &message, None, ModelCategory::Auto);
        assert!(result.is_err());
    }

    #[test]
    fn prepare_request_rejects_audio_and_video_urls() {
        let audio_message = ChatMessage::with_audio(
            "user",
            AudioSource::Url {
                url: "https://example.com/x.mp3".to_string(),
            },
        );
        assert!(prepare_request(None, &audio_message, None, ModelCategory::Auto).is_err());

        let video_message = ChatMessage::with_video(
            "user",
            VideoSource::Url {
                url: "https://example.com/x.mp4".to_string(),
            },
        );
        assert!(prepare_request(None, &video_message, None, ModelCategory::Auto).is_err());
    }

    #[test]
    fn generation_config_with_tools_round_trips() {
        let tools = vec![serde_json::json!({
            "name": "doubler",
            "parameters": { "type": "object" }
        })];
        let config = GenerationConfig::default().with_tools(tools.clone());
        assert_eq!(config.tools, Some(tools));
    }

    #[test]
    fn prepared_request_carries_default_flags() {
        let message = ChatMessage::user("hello");
        let prepared = prepare_request(None, &message, None, ModelCategory::Auto).unwrap();
        assert!(prepared.tools.is_none());
        assert!(!prepared.refresh_on_auth_error);
    }
}
