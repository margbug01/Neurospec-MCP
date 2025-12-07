// MCP tool-related constants

/// Interactive dialogue tool identifier
pub const TOOL_INTERACT: &str = "interact";

/// Memory management tool identifier
pub const TOOL_MEMORY: &str = "memory";

/// Code search tool identifier
pub const TOOL_SEARCH: &str = "search";

/// NeuroSpec 高级工具标识符（重构辅助）
pub const TOOL_NEUROSPEC_IMPACT_ANALYSIS: &str = "neurospec_graph_impact_analysis";
pub const TOOL_NEUROSPEC_RENAME: &str = "neurospec_refactor_rename";

/// Default enabled tools list
pub const DEFAULT_ENABLED_TOOLS: &[&str] = &[
    TOOL_INTERACT,
    TOOL_MEMORY,
    TOOL_SEARCH,
    TOOL_NEUROSPEC_IMPACT_ANALYSIS,
    TOOL_NEUROSPEC_RENAME,
];

/// 继续回复默认启用状态
pub const DEFAULT_CONTINUE_REPLY_ENABLED: bool = true;

/// 默认自动继续阈值
pub const DEFAULT_AUTO_CONTINUE_THRESHOLD: u32 = 1000;

/// 默认继续提示词
pub const DEFAULT_CONTINUE_PROMPT: &str = "请按照最佳实践继续";

/// MCP 请求超时时间 (ms)
pub const REQUEST_TIMEOUT_MS: u64 = 30000;

// ==================== Daemon 通讯超时配置 ====================

/// 默认弹窗超时时间（秒）- 用户交互等待
/// 设置为 10 分钟，给用户充足的思考时间
pub const DEFAULT_POPUP_TIMEOUT_SECS: u64 = 600;

/// 最小弹窗超时（秒）
pub const MIN_POPUP_TIMEOUT_SECS: u64 = 60;

/// 最大弹窗超时（秒）- 1小时
pub const MAX_POPUP_TIMEOUT_SECS: u64 = 3600;

/// 默认心跳间隔（秒）
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// HTTP 客户端超时（秒）- 应大于弹窗超时
pub const DEFAULT_HTTP_CLIENT_TIMEOUT_SECS: u64 = 660;

/// MCP 重试次数
pub const MAX_RETRY_COUNT: u32 = 3;

// MCP 工具配置结构体
#[derive(Debug, Clone)]
pub struct McpToolConfig {
    pub tool_id: String,
    pub enabled: bool,
    pub can_disable: bool,
}

impl McpToolConfig {
    pub fn new(tool_id: &str, enabled: bool, can_disable: bool) -> Self {
        Self {
            tool_id: tool_id.to_string(),
            enabled,
            can_disable,
        }
    }
}

// MCP 配置结构体
#[derive(Debug, Clone)]
pub struct McpConfig {
    pub tools: Vec<McpToolConfig>,
    pub continue_reply_enabled: bool,
    pub auto_continue_threshold: u32,
    pub continue_prompt: String,
    pub request_timeout_ms: u64,
    pub max_retry_count: u32,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            tools: vec![
                McpToolConfig::new(TOOL_INTERACT, true, false), // Interactive tool cannot be disabled
                McpToolConfig::new(TOOL_MEMORY, true, true),    // Memory tool enabled by default
                McpToolConfig::new(TOOL_SEARCH, true, true),    // Search tool enabled by default
                // NeuroSpec 高级工具（重构辅助）
                McpToolConfig::new(TOOL_NEUROSPEC_IMPACT_ANALYSIS, true, true),
                McpToolConfig::new(TOOL_NEUROSPEC_RENAME, true, true),
            ],
            continue_reply_enabled: DEFAULT_CONTINUE_REPLY_ENABLED,
            auto_continue_threshold: DEFAULT_AUTO_CONTINUE_THRESHOLD,
            continue_prompt: DEFAULT_CONTINUE_PROMPT.to_string(),
            request_timeout_ms: REQUEST_TIMEOUT_MS,
            max_retry_count: MAX_RETRY_COUNT,
        }
    }
}

impl McpConfig {
    /// 获取工具配置
    pub fn get_tool_config(&self, tool_id: &str) -> Option<&McpToolConfig> {
        self.tools.iter().find(|tool| tool.tool_id == tool_id)
    }

    /// 检查工具是否启用
    pub fn is_tool_enabled(&self, tool_id: &str) -> bool {
        self.get_tool_config(tool_id)
            .map(|tool| tool.enabled)
            .unwrap_or(false)
    }

    /// 设置工具启用状态
    pub fn set_tool_enabled(&mut self, tool_id: &str, enabled: bool) -> bool {
        if let Some(tool) = self.tools.iter_mut().find(|tool| tool.tool_id == tool_id) {
            if tool.can_disable || enabled {
                tool.enabled = enabled;
                return true;
            }
        }
        false
    }

    /// 转换为 JSON 格式
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "tools": self.tools.iter().map(|tool| {
                serde_json::json!({
                    "id": tool.tool_id,
                    "enabled": tool.enabled,
                    "can_disable": tool.can_disable
                })
            }).collect::<Vec<_>>(),
            "continue_reply_enabled": self.continue_reply_enabled,
            "auto_continue_threshold": self.auto_continue_threshold,
            "continue_prompt": self.continue_prompt,
            "request_timeout_ms": self.request_timeout_ms,
            "max_retry_count": self.max_retry_count
        })
    }
}

// 便捷函数
/// 获取默认 MCP 配置
pub fn get_default_mcp_config() -> McpConfig {
    McpConfig::default()
}

/// Check if it's a valid tool ID
pub fn is_valid_tool_id(tool_id: &str) -> bool {
    matches!(
        tool_id,
        TOOL_INTERACT | TOOL_MEMORY | TOOL_SEARCH |
        TOOL_NEUROSPEC_IMPACT_ANALYSIS |
        TOOL_NEUROSPEC_RENAME
    )
}
