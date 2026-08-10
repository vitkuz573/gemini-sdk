//! Integration tests for the function calling tool API.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use gemini_sdk::tool::{Tool, ToolCall, ToolError, ToolResult};
use serde_json::Value;

struct MockTool {
    name: String,
    schema: Value,
    handler: Box<dyn Fn(Value) -> Result<Value, ToolError> + Send + Sync>,
}

impl MockTool {
    fn new<F>(name: impl Into<String>, schema: Value, handler: F) -> Self
    where
        F: Fn(Value) -> Result<Value, ToolError> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            schema,
            handler: Box::new(handler),
        }
    }
}

impl Tool for MockTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn schema(&self) -> Value {
        self.schema.clone()
    }

    fn invoke(
        &self,
        args: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value, ToolError>> + Send + '_>> {
        let result = (self.handler)(args);
        Box::pin(async move { result })
    }
}

#[test]
fn mock_tool_name_and_schema_passthrough() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "n": { "type": "integer" } },
        "required": ["n"]
    });
    let tool = MockTool::new("doubler", schema.clone(), |args| {
        let n = args["n"].as_i64().unwrap_or(0);
        Ok(serde_json::json!({ "result": n * 2 }))
    });

    assert_eq!(tool.name(), "doubler");
    assert_eq!(tool.schema(), schema);
}

#[tokio::test]
async fn mock_tool_invokes_with_boxed_future() {
    let tool = MockTool::new("doubler", serde_json::json!({}), |args| {
        let n = args["n"].as_i64().unwrap_or(0);
        Ok(serde_json::json!({ "result": n * 2 }))
    });

    let result = tool.invoke(serde_json::json!({ "n": 3 })).await.unwrap();
    assert_eq!(result, serde_json::json!({ "result": 6 }));
}

#[test]
fn box_dyn_tool_is_object_safe() {
    let tool: Box<dyn Tool> = Box::new(MockTool::new("x", serde_json::json!({}), |_| {
        Ok(Value::Null)
    }));
    assert_eq!(tool.name(), "x");
}

#[test]
fn arc_dyn_tool_is_object_safe() {
    let tool: Arc<dyn Tool> = Arc::new(MockTool::new("x", serde_json::json!({}), |_| {
        Ok(Value::Null)
    }));
    assert_eq!(tool.name(), "x");
}

#[test]
fn tool_error_messages_are_generic() {
    let not_found = ToolError::NotFound("unknown".to_string());
    let message = not_found.to_string();
    assert!(message.contains("unknown"));
    assert!(!message.contains("password"));

    let invalid = ToolError::InvalidArgs("bad shape".to_string());
    assert!(invalid.to_string().contains("bad shape"));

    let invoke = ToolError::InvokeFailed("downstream".to_string());
    assert!(invoke.to_string().contains("downstream"));
}

#[test]
fn tool_call_serializes_round_trip() {
    let call = ToolCall::new("doubler", serde_json::json!({ "n": 5 }));
    let serialized = serde_json::to_string(&call).unwrap();
    let deserialized: ToolCall = serde_json::from_str(&serialized).unwrap();
    assert_eq!(call, deserialized);
}

#[test]
fn tool_result_serializes_round_trip() {
    let result = ToolResult::new("doubler", serde_json::json!({ "result": 10 }));
    let serialized = serde_json::to_string(&result).unwrap();
    let deserialized: ToolResult = serde_json::from_str(&serialized).unwrap();
    assert_eq!(result, deserialized);
}
