//! Function calling primitives for the Gemini SDK.
//!
//! The SDK exposes a lightweight, object-safe [`Tool`] trait that callers
//! implement to let the model invoke local code. The trait avoids an
//! `async-trait` dependency by returning boxed futures, matching the pattern
//! used by [`crate::auth::CredentialsProvider`].

use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// A single tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    /// Name of the registered tool to invoke.
    pub name: String,
    /// Arguments parsed from the model output, typically matching the tool's
    /// declared JSON Schema.
    pub args: Value,
}

impl ToolCall {
    /// Creates a new tool call.
    #[must_use]
    pub fn new(name: impl Into<String>, args: Value) -> Self {
        Self { name: name.into(), args }
    }
}

/// The result of invoking a tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolResult {
    /// Name of the tool that produced this result.
    pub name: String,
    /// JSON-serializable result returned by the tool.
    pub result: Value,
}

impl ToolResult {
    /// Creates a new tool result.
    #[must_use]
    pub fn new(name: impl Into<String>, result: Value) -> Self {
        Self { name: name.into(), result }
    }
}

/// Errors that can occur while validating or invoking a tool.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// The tool arguments do not match the expected schema or shape.
    #[error("invalid tool arguments: {0}")]
    InvalidArgs(String),

    /// The tool handler failed during invocation.
    #[error("tool invocation failed: {0}")]
    InvokeFailed(String),

    /// A tool name requested by the model is not registered.
    #[error("unknown tool: {0}")]
    NotFound(String),
}

/// An object-safe, asynchronous tool that the model can invoke.
///
/// Implementors provide a JSON Schema describing the tool's parameters and an
/// async handler that receives the parsed arguments. The boxed-future return
/// type keeps the trait object-safe without requiring `async-trait`.
///
/// # Security
///
/// Arguments passed to [`Tool::invoke`] are parsed from untrusted model output.
/// Implementors must validate argument shapes and avoid exposing secrets or
/// executing sensitive operations without additional authorization.
pub trait Tool: Send + Sync {
    /// Returns the tool's registered name.
    fn name(&self) -> &str;

    /// Returns the JSON Schema object describing this tool's parameters.
    fn schema(&self) -> Value;

    /// Invokes the tool with the parsed arguments.
    fn invoke(
        &self,
        args: Value,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<Value, ToolError>> + Send + '_>>;
}

impl Tool for std::sync::Arc<dyn Tool> {
    fn name(&self) -> &str {
        (**self).name()
    }

    fn schema(&self) -> Value {
        (**self).schema()
    }

    fn invoke(
        &self,
        args: Value,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<Value, ToolError>> + Send + '_>> {
        (**self).invoke(args)
    }
}

/// Builds a JSON Schema object from a tool name and parameter spec.
///
/// This is a convenience helper for tools that only need a flat parameter
/// object. The returned value has `type: "object"` and the supplied
/// `properties` and `required` arrays.
#[must_use]
pub fn tool_declaration(name: impl Into<String>, parameters: Value) -> Value {
    serde_json::json!({
        "name": name.into(),
        "parameters": parameters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct Doubler;

    impl Tool for Doubler {
        fn name(&self) -> &str {
            "doubler"
        }

        fn schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": { "n": { "type": "integer" } },
                "required": ["n"]
            })
        }

        fn invoke(
            &self,
            args: Value,
        ) -> Pin<Box<dyn Future<Output = std::result::Result<Value, ToolError>> + Send + '_>>
        {
            Box::pin(async move {
                let n = args
                    .get("n")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| ToolError::InvalidArgs("missing n".to_string()))?;
                Ok(serde_json::json!({ "result": n * 2 }))
            })
        }
    }

    #[tokio::test]
    async fn tool_invokes_with_boxed_future() {
        let tool = Doubler;
        let result = tool
            .invoke(serde_json::json!({ "n": 3 }))
            .await
            .expect("doubler should succeed");
        assert_eq!(result, serde_json::json!({ "result": 6 }));
    }

    #[tokio::test]
    async fn arc_dyn_tool_is_object_safe() {
        let tool: Arc<dyn Tool> = Arc::new(Doubler);
        assert_eq!(tool.name(), "doubler");
        let result = tool
            .invoke(serde_json::json!({ "n": 4 }))
            .await
            .expect("doubler should succeed");
        assert_eq!(result, serde_json::json!({ "result": 8 }));
    }

    #[tokio::test]
    async fn tool_error_display_is_generic() {
        let err = ToolError::NotFound("unknown".to_string());
        let message = err.to_string();
        assert!(message.contains("unknown"));
        assert!(!message.contains("secret"));
    }

    #[test]
    fn tool_call_and_result_serialize() {
        let call = ToolCall::new("doubler", serde_json::json!({ "n": 1 }));
        let serialized = serde_json::to_string(&call).unwrap();
        assert!(serialized.contains("doubler"));

        let result = ToolResult::new("doubler", serde_json::json!({ "result": 2 }));
        let serialized = serde_json::to_string(&result).unwrap();
        assert!(serialized.contains("result"));
    }
}
