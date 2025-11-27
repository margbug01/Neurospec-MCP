/// Tool registration utilities
///
/// Provides macros and helpers to reduce boilerplate when registering MCP tools

/// Register a tool with minimal boilerplate
///
/// # Arguments
/// * `$tools` - The Vec<Tool> to push the tool into
/// * `$self` - The ZhiServer instance (for checking if tool is enabled)
/// * `$tool_id` - Tool identifier string
/// * `$description` - Tool description string
/// * `$args_type` - The type of the tool's arguments (must implement JsonSchema)
///
/// # Example
/// ```
/// register_tool!(
///     tools,
///     self,
///     "neurospec.xray",
///     "扫描项目结构，生成X-Ray摘要",
///     crate::neurospec::tools::XrayArgs
/// );
/// ```
#[macro_export]
macro_rules! register_tool {
    ($tools:expr, $self:expr, $tool_id:expr, $description:expr, $args_type:ty) => {
        if $self.is_tool_enabled($tool_id) {
            let schema = schemars::schema_for!($args_type);
            if let Ok(schema_json) = serde_json::to_value(&schema.schema) {
                if let serde_json::Value::Object(schema_map) = schema_json {
                    $tools.push($crate::mcp::create_tool($tool_id, $description, schema_map));
                }
            }
        }
    };
}

/// Register a tool that doesn't require arguments
///
/// # Example
/// ```
/// register_simple_tool!(
///     tools,
///     self,
///     "neurospec.nsp_schema",
///     "获取NSP JSON Schema"
/// );
/// ```
#[macro_export]
macro_rules! register_simple_tool {
    ($tools:expr, $self:expr, $tool_id:expr, $description:expr) => {
        if $self.is_tool_enabled($tool_id) {
            // MCP 规范要求 input_schema 必须是有效的 JSON Schema
            // 对于无参数工具，需要 type: "object" 和 properties: {}
            let mut schema_map = serde_json::Map::new();
            schema_map.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            schema_map.insert("properties".to_string(), serde_json::Value::Object(serde_json::Map::new()));
            $tools.push($crate::mcp::create_tool(
                $tool_id,
                $description,
                schema_map,
            ));
        }
    };
}
