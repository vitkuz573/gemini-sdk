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
            _ => Err(crate::errors::Error::bad_request(format!("unknown model category: {s}"))),
        }
    }
}

/// Metadata about a model available in the Gemini web frontend picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModelInfo {
    /// Google's internal hex mode ID.
    pub(crate) id: String,
    /// Short title shown in the UI (e.g. "3.6 Flash").
    pub(crate) title: String,
    /// Longer description.
    pub(crate) description: String,
    /// Versioned name if available.
    pub(crate) versioned_name: Option<String>,
    /// Category reported by the model picker.
    pub(crate) category: ModelCategory,
    /// Numeric enum value used in `StreamGenerate` slot 30.
    pub(crate) category_enum: u64,
}

impl ModelInfo {
    /// Returns the model's display name.
    ///
    /// Prefers the versioned name (e.g. "Gemini 3.6 Flash") when available,
    /// otherwise falls back to the short title.
    #[must_use]
    pub fn display_name(&self) -> String {
        self.versioned_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .map_or_else(|| self.title.clone(), |s| s.to_string())
    }

    /// Returns the model's internal hex mode ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the short title shown in the UI (e.g. "3.6 Flash").
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the longer model description, if any.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the versioned name (e.g. "Gemini 3.6 Flash") if available.
    #[must_use]
    pub fn versioned_name(&self) -> Option<&str> {
        self.versioned_name.as_deref()
    }

    /// Returns the model category reported by the model picker.
    #[must_use]
    pub fn category(&self) -> ModelCategory {
        self.category
    }

    /// Returns the numeric enum value used in `StreamGenerate` slot 30.
    #[must_use]
    pub fn category_enum(&self) -> u64 {
        self.category_enum
    }
}

/// Derives a category from the model title or hex id as a fallback when the
/// picker does not report one.
///
/// Precedence (first match wins):
/// 1. `lite` → `FlashLite`
/// 2. `thinking` / `deep` → `Thinking`
/// 3. `pro` → `Pro`
/// 4. `auto` → `Auto`
/// 5. otherwise → `Fast`
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
    fn display_name_prefers_versioned_name() {
        let info = ModelInfo {
            id: "abc".to_string(),
            title: "Flash".to_string(),
            description: String::new(),
            versioned_name: Some("Gemini 3.6 Flash".to_string()),
            category: ModelCategory::Fast,
            category_enum: 1,
        };
        assert_eq!(info.display_name(), "Gemini 3.6 Flash");
    }

    #[test]
    fn display_name_falls_back_to_title() {
        let info = ModelInfo {
            id: "abc".to_string(),
            title: "Flash".to_string(),
            description: String::new(),
            versioned_name: None,
            category: ModelCategory::Fast,
            category_enum: 1,
        };
        assert_eq!(info.display_name(), "Flash");
    }

    #[test]
    fn derive_category_from_strings() {
        assert_eq!(derive_category("x", "3.6 Flash"), ModelCategory::Fast);
        assert_eq!(derive_category("x", "3.1 Pro"), ModelCategory::Pro);
        assert_eq!(derive_category("x", "3.5 Flash-Lite"), ModelCategory::FlashLite);
        assert_eq!(derive_category("x", "Thinking"), ModelCategory::Thinking);
    }
}
