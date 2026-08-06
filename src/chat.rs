//! High-level chat types for building requests and reading responses.

use std::fmt;

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::errors::{Error, Result};
use crate::models::ModelCategory;

/// A single message in a conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Role of the message author: `user`, `model`, or `system`.
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

    /// Creates a system message with the given text.
    pub fn system(text: impl Into<String>) -> Self {
        Self::text("system", text)
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
    /// A function call produced by the model.
    FunctionCall {
        /// Function name.
        name: String,
        /// Function arguments as a JSON value.
        args: serde_json::Value,
    },
    /// A function response provided by the caller.
    FunctionResponse {
        /// Function name.
        name: String,
        /// Response payload as a JSON value.
        response: serde_json::Value,
    },
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
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// Text content returned by the model.
    pub text: String,
    /// Function calls emitted by the model.
    pub function_calls: Vec<FunctionCall>,
    /// Whether the response includes reasoning / thinking content.
    pub has_thoughts: bool,
    /// Reasoning content, if any.
    pub thoughts: Vec<String>,
}

impl ChatResponse {
    /// Returns a reference to the response text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns `true` if the response contains at least one function call.
    #[must_use]
    pub fn has_function_calls(&self) -> bool {
        !self.function_calls.is_empty()
    }
}

/// A function call parsed from a model response.
#[derive(Debug, Clone)]
pub struct FunctionCall {
    /// Function name.
    pub name: String,
    /// Function arguments.
    pub args: serde_json::Value,
}

impl fmt::Display for FunctionCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.name, self.args)
    }
}

/// Internal helper: serialises a list of chat messages into the flattened prompt
/// string used by the Gemini web frontend.
pub(crate) fn serialize_messages_to_prompt(
    messages: &[ChatMessage],
    system: Option<&ChatMessage>,
) -> String {
    let mut sections = Vec::new();

    if let Some(sys) = system {
        let text = collect_text_parts(sys);
        if !text.is_empty() {
            sections.push(format!("<system>\n{}\n</system>", xml_escape(&text)));
        }
    }

    for message in messages {
        let role = normalize_role(&message.role);
        let body = message
            .parts
            .iter()
            .map(|part| match part {
                ContentPart::Text(t) => xml_escape(t),
                ContentPart::Image(_) => String::new(),
                ContentPart::FunctionCall { name, args } => {
                    format!(
                        "<function_call name=\"{}\">{}</function_call>",
                        xml_escape(name),
                        args
                    )
                }
                ContentPart::FunctionResponse { name, response } => {
                    format!(
                        "<function_response name=\"{}\">{}</function_response>",
                        xml_escape(name),
                        response
                    )
                }
            })
            .collect::<String>();

        if !body.is_empty() {
            sections.push(format!("<{role}>\n{body}\n</{role}>"));
        }
    }

    if sections.is_empty() {
        return "Hello".to_string();
    }
    sections.join("\n\n")
}

fn collect_text_parts(message: &ChatMessage) -> String {
    message
        .parts
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_role(role: &str) -> &str {
    match role {
        "user" => "user",
        "model" | "assistant" => "assistant",
        "system" => "system",
        other => other,
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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
/// Internal type used when preparing a generation request.
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
    conversation: Option<&Conversation>,
    new_message: &ChatMessage,
    config: Option<GenerationConfig>,
    default_category: ModelCategory,
) -> Result<PreparedRequest> {
    let category = conversation
        .and_then(|c| c.model_category)
        .unwrap_or(default_category);

    let mut messages: Vec<ChatMessage> = conversation
        .map(|c| c.messages().to_vec())
        .unwrap_or_default();
    messages.push(new_message.clone());

    let system = messages
        .iter()
        .position(|m| m.role == "system")
        .map(|idx| messages.remove(idx));

    let mut inline_images = Vec::new();
    let mut sanitized_messages = Vec::with_capacity(messages.len());
    for message in messages {
        let mut parts = Vec::new();
        for part in message.parts {
            match part {
                ContentPart::Image(ImageSource::InlineData { mime_type, data }) => {
                    inline_images.push((mime_type, data));
                }
                ContentPart::Image(ImageSource::Url { url }) => {
                    return Err(Error::bad_request(format!(
                        "image URLs are not supported directly by the web frontend: {url}"
                    )));
                }
                other => parts.push(other),
            }
        }
        sanitized_messages.push(ChatMessage {
            role: message.role,
            parts,
        });
    }

    let prompt = serialize_messages_to_prompt(&sanitized_messages, system.as_ref());

    Ok(PreparedRequest {
        prompt,
        inline_images,
        config,
        category,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_simple_turn() {
        let messages = vec![ChatMessage::user("Hello")];
        let prompt = serialize_messages_to_prompt(&messages, None);
        assert_eq!(prompt, "<user>\nHello\n</user>");
    }

    #[test]
    fn xml_escape_works() {
        assert_eq!(xml_escape("a & b"), "a &amp; b");
        assert_eq!(xml_escape("<tag>"), "&lt;tag&gt;");
        assert_eq!(xml_escape("\"x\""), "&quot;x&quot;");
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
        assert!(prepared.prompt.contains("Look at this"));
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
