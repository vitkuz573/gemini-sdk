//! Model discovery and metadata for the Gemini web frontend.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A category reported by the Gemini model picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ModelCategory {
    /// Fast models (e.g. Flash family).
    Fast,
    /// Thinking / reasoning models.
    Thinking,
    /// Pro models.
    Pro,
    /// Auto / fallback category.
    Auto,
    /// Fast-Dynamic-Thinking (experimental).
    FastDynamicThinking,
    /// Flash-Lite models.
    FlashLite,
}

impl ModelCategory {
    /// Returns the numeric enum value used in `StreamGenerate` slot 30.
    #[must_use]
    pub fn as_enum_value(self) -> u64 {
        match self {
            Self::Fast => 1,
            Self::Thinking => 2,
            Self::Pro => 3,
            Self::Auto => 4,
            Self::FastDynamicThinking => 5,
            Self::FlashLite => 6,
        }
    }

    /// Parses a category from its numeric enum value.
    pub fn from_enum_value(value: u64) -> Option<Self> {
        match value {
            1 => Some(Self::Fast),
            2 => Some(Self::Thinking),
            3 => Some(Self::Pro),
            4 => Some(Self::Auto),
            5 => Some(Self::FastDynamicThinking),
            6 => Some(Self::FlashLite),
            _ => None,
        }
    }
}

impl fmt::Display for ModelCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Fast => "FAST",
            Self::Thinking => "THINKING",
            Self::Pro => "PRO",
            Self::Auto => "AUTO",
            Self::FastDynamicThinking => "FAST_DYNAMIC_THINKING",
            Self::FlashLite => "FLASH_LITE",
        };
        write!(f, "{s}")
    }
}

impl FromStr for ModelCategory {
    type Err = crate::errors::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "FAST" => Ok(Self::Fast),
            "THINKING" => Ok(Self::Thinking),
            "PRO" => Ok(Self::Pro),
            "AUTO" => Ok(Self::Auto),
            "FAST_DYNAMIC_THINKING" => Ok(Self::FastDynamicThinking),
            "FLASH_LITE" | "FLASHLITE" => Ok(Self::FlashLite),
            _ => Err(crate::errors::Error::bad_request(format!(
                "unknown model category: {s}"
            ))),
        }
    }
}

/// Metadata about a model available in the Gemini web frontend picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Google's internal hex mode ID.
    pub id: String,
    /// Short title shown in the UI (e.g. "3.6 Flash").
    pub title: String,
    /// Longer description.
    pub description: String,
    /// Versioned name if available.
    pub versioned_name: Option<String>,
    /// Category reported by the model picker.
    pub category: ModelCategory,
    /// Numeric enum value used in `StreamGenerate` slot 30.
    pub category_enum: u64,
}

impl ModelInfo {
    /// Returns a stable, human-readable OpenAI-style model ID.
    ///
    /// Examples:
    /// - "3.6 Flash" -> "gemini-3.6-flash"
    /// - "3.1 Pro" -> "gemini-3.1-pro"
    #[must_use]
    pub fn human_id(&self) -> String {
        let source = self
            .versioned_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.title);
        let lower = source.to_lowercase();
        let mut parts: Vec<&str> = lower.split_whitespace().collect();
        parts.retain(|p| *p != "gemini");
        if parts.is_empty() {
            return "gemini-unknown".to_string();
        }
        format!("gemini-{}", parts.join("-"))
    }

    /// Returns the `models/<hex>` root identifier used by the proxy.
    #[must_use]
    pub fn root_id(&self) -> String {
        format!("models/{}", self.id)
    }
}

/// Derives a category from the model title or hex id as a fallback when the
/// picker does not report one.
#[must_use]
pub(crate) fn derive_category(id: &str, title: &str) -> ModelCategory {
    let combined = format!("{id} {title}").to_lowercase();
    if combined.contains("lite") {
        ModelCategory::FlashLite
    } else if combined.contains("thinking") || combined.contains("deep") {
        ModelCategory::Thinking
    } else if combined.contains("pro") {
        ModelCategory::Pro
    } else if combined.contains("auto") {
        ModelCategory::Auto
    } else {
        ModelCategory::Fast
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_enum_round_trip() {
        for category in [
            ModelCategory::Fast,
            ModelCategory::Thinking,
            ModelCategory::Pro,
            ModelCategory::Auto,
            ModelCategory::FastDynamicThinking,
            ModelCategory::FlashLite,
        ] {
            assert_eq!(ModelCategory::from_enum_value(category.as_enum_value()), Some(category));
        }
    }

    #[test]
    fn human_id_generation() {
        let info = ModelInfo {
            id: "abc".to_string(),
            title: "Flash".to_string(),
            description: String::new(),
            versioned_name: Some("3.6 Flash".to_string()),
            category: ModelCategory::Fast,
            category_enum: 1,
        };
        assert_eq!(info.human_id(), "gemini-3.6-flash");
    }

    #[test]
    fn derive_category_from_strings() {
        assert_eq!(derive_category("x", "3.6 Flash"), ModelCategory::Fast);
        assert_eq!(derive_category("x", "3.1 Pro"), ModelCategory::Pro);
        assert_eq!(derive_category("x", "3.5 Flash-Lite"), ModelCategory::FlashLite);
        assert_eq!(derive_category("x", "Thinking"), ModelCategory::Thinking);
    }
}
