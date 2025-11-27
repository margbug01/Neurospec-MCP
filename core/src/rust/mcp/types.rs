use chrono;
use serde::{Deserialize, Serialize};

// Interaction tool request (interactive dialogue with user)
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct InteractRequest {
    #[schemars(description = "Message to display to the user")]
    pub message: String,
    #[schemars(description = "List of predefined options (optional)")]
    #[serde(default)]
    pub predefined_options: Vec<String>,
    #[schemars(description = "Whether the message is in Markdown format, defaults to true")]
    #[serde(default = "default_is_markdown")]
    pub is_markdown: bool,
}


fn default_is_markdown() -> bool {
    true
}

// Memory management tool request
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MemoryRequest {
    #[schemars(description = "Action type: 'remember' (add), 'recall' (retrieve), 'update' (modify), 'delete' (remove), 'list' (paginated list)")]
    pub action: String,
    #[schemars(description = "Project path (optional, auto-detects from current working directory or Git root if omitted)")]
    #[serde(default)]
    pub project_path: String,
    #[schemars(description = "Memory content (required for 'remember'/'update' action)")]
    #[serde(default)]
    pub content: String,
    #[schemars(description = "Memory category: rule, preference, pattern, context")]
    #[serde(default = "default_category")]
    pub category: String,
    #[schemars(description = "Memory ID (required for 'update'/'delete' action)")]
    #[serde(default)]
    pub id: Option<String>,
    #[schemars(description = "Page number for 'list' action (default: 1)")]
    #[serde(default = "default_page")]
    pub page: usize,
    #[schemars(description = "Page size for 'list' action (default: 20)")]
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    #[schemars(description = "Context for smart recall (optional, improves relevance)")]
    #[serde(default)]
    pub context: Option<String>,
}


fn default_category() -> String {
    "context".to_string()
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupRequest {
    pub id: String,
    pub message: String,
    pub predefined_options: Option<Vec<String>>,
    pub is_markdown: bool,
}

/// 新的结构化响应数据格式
#[derive(Debug, Deserialize)]
pub struct McpResponse {
    pub user_input: Option<String>,
    pub selected_options: Vec<String>,
    pub images: Vec<ImageAttachment>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageAttachment {
    pub data: String,
    pub media_type: String,
    pub filename: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMetadata {
    pub timestamp: Option<String>,
    pub request_id: Option<String>,
    pub source: Option<String>,
}

/// 旧格式兼容性支持
#[derive(Debug, Deserialize)]
pub struct McpResponseContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub source: Option<ImageSource>,
}

#[derive(Debug, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// 统一的响应构建函数
///
/// 用于生成标准的JSON响应格式，确保无GUI和有GUI模式输出一致
pub fn build_mcp_response(
    user_input: Option<String>,
    selected_options: Vec<String>,
    images: Vec<ImageAttachment>,
    request_id: Option<String>,
    source: &str,
) -> serde_json::Value {
    serde_json::json!({
        "user_input": user_input,
        "selected_options": selected_options,
        "images": images,
        "metadata": {
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "request_id": request_id,
            "source": source
        }
    })
}

/// 构建发送操作的响应
pub fn build_send_response(
    user_input: Option<String>,
    selected_options: Vec<String>,
    images: Vec<ImageAttachment>,
    request_id: Option<String>,
    source: &str,
) -> String {
    let response = build_mcp_response(user_input, selected_options, images, request_id, source);
    response.to_string()
}

/// 构建继续操作的响应
pub fn build_continue_response(request_id: Option<String>, source: &str) -> String {
    // 动态获取继续提示词
    let continue_prompt = if let Ok(config) = crate::config::load_standalone_config() {
        config.reply_config.continue_prompt
    } else {
        "请按照最佳实践继续".to_string()
    };

    let response = build_mcp_response(Some(continue_prompt), vec![], vec![], request_id, source);
    response.to_string()
}
