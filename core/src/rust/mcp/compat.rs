// Compatibility helpers for rmcp API changes
use rmcp::model::*;
use std::borrow::Cow;
use std::sync::Arc;

/// Create a Tool with default values for new fields
pub fn create_tool(
    name: &'static str,
    description: &'static str,
    input_schema: serde_json::Map<String, serde_json::Value>,
) -> Tool {
    Tool {
        name: Cow::Borrowed(name),
        title: None,
        description: Some(Cow::Borrowed(description)),
        input_schema: Arc::new(input_schema),
        annotations: None,
        icons: None,
        meta: None,
        output_schema: None,
    }
}

/// Create a successful CallToolResult with default values for new fields
pub fn create_success_result(content: Vec<Content>) -> CallToolResult {
    CallToolResult {
        content,
        is_error: None,
        meta: None,
        structured_content: None,
    }
}

/// Create an error CallToolResult
pub fn create_error_result(error_message: String) -> CallToolResult {
    CallToolResult {
        content: vec![Content::text(error_message)],
        is_error: Some(true),
        meta: None,
        structured_content: None,
    }
}

/// Create Implementation info with default values for new fields
pub fn create_implementation(name: String, version: String) -> Implementation {
    Implementation {
        name,
        version,
        icons: None,
        title: None,
        website_url: None,
    }
}
