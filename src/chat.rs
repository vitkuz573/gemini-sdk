//! High-level chat types for building requests and reading responses.

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::errors::{Error, Result};
use crate::models::ModelCategory;

/// A single message in a conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Role of the message author: `user` or `model`.
    pub role: String,
    /// Content parts that make up this message.
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
        Self::text("user", text)
    }

    /// Creates a model message with the given text.
    pub fn model(text: impl Into<String>) -> Self {
        Self::text("model", text)
    }

    /// Creates a user message containing an image.
    pub fn with_image(role: impl Into<String>, source: ImageSource) -> Self {
        Self {
            role: role.into(),
            parts: vec![ContentPart::Image(source)],
        }
    }
}

/// A source for image content.
#[derive(Debug, Clone)]
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

/// A part of a chat message.
#[derive(Debug, Clone)]
pub enum ContentPart {
    /// Plain text.
    Text(String),
    /// An image.
    Image(ImageSource),
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
}

/// Thinking level requested for a generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
pub struct ChatResponse {
    /// Text content returned by the model.
    pub text: String,
}

impl ChatResponse {
    /// Creates a response from a single text string.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
        }
    }

    /// Returns a reference to the response text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
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
            ContentPart::Image(_) => {}
        }
    }
    let prompt = text_parts.join("\n");
    if prompt.is_empty() {
        return Err(Error::bad_request("prompt is empty"));
    }
    Ok(prompt)
}

/// An in-progress conversation that carries multi-turn state.
#[derive(Debug, Clone, Default)]
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
}

/// Internal type used when preparing a generation request.
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct PreparedRequest {
    /// Flattened prompt text.
    pub prompt: String,
    /// Inline images as `(mime_type, base64_data)` pairs.
    pub inline_images: Vec<(String, String)>,
    /// Generation configuration.
    pub config: Option<GenerationConfig>,
    /// Selected model category.
    pub category: ModelCategory,
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
            ContentPart::Text(_) => {}
        }
    }

    Ok(PreparedRequest {
        prompt,
        inline_images,
        config,
        category: default_category,
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
        message.parts.push(ContentPart::Image(ImageSource::from_bytes(
            "image/png",
            b"fake",
        )));

        let prepared = prepare_request(None, &message, None, ModelCategory::Auto).unwrap();
        assert_eq!(prepared.inline_images.len(), 1);
        assert_eq!(prepared.prompt, "Look at this");
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
}
