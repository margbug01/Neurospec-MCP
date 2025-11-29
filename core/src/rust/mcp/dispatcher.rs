use rmcp::{model::CallToolResult, ErrorData as McpError};
use std::sync::Once;

use crate::mcp::tools::{AcemcpTool, InteractionTool, MemoryTool};
use crate::mcp::types::{InteractRequest, MemoryRequest};
use crate::mcp::utils::errors::invalid_params_error;
use crate::mcp::tools::unified_store::{init_global_search_config, init_global_store, init_global_watcher, is_search_initialized};

/// 确保搜索系统只初始化一次
static SEARCH_INIT: Once = Once::new();

/// 初始化 MCP 搜索系统
/// 
/// 在 MCP stdio 模式下，daemon 服务器可能未启动，
/// 因此需要在 dispatcher 中也进行初始化。
fn ensure_search_system_initialized() {
    SEARCH_INIT.call_once(|| {
        if is_search_initialized() {
            return; // 已由 daemon 初始化
        }
        
        // 使用与 LocalEngineConfig::default() 一致的路径，复用已有索引
        // 索引路径: ~/.acemcp/local_index
        // 存储路径: %LOCALAPPDATA%/neurospec/unified_store
        let default_config = crate::mcp::tools::acemcp::local_engine::LocalEngineConfig::default();
        let index_cache_dir = default_config.index_path;
        
        let store_cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("neurospec")
            .join("unified_store");
        
        // 初始化全局存储
        if let Err(e) = init_global_store(&store_cache_dir) {
            crate::log_important!(warn, "[MCP] Failed to initialize global store: {}", e);
        }
        
        // 初始化全局搜索配置（使用默认路径）
        if let Err(e) = init_global_search_config(&index_cache_dir) {
            crate::log_important!(warn, "[MCP] Failed to initialize search config: {}", e);
        } else {
            crate::log_important!(info, "[MCP] Search system initialized (index_path: {:?})", index_cache_dir);
        }
        
        // 初始化文件监听器
        if let Err(e) = init_global_watcher() {
            crate::log_important!(warn, "[MCP] Failed to initialize file watcher: {}", e);
        }
    });
}

/// Tool dispatcher - provides O(1) tool name validation and routing
///
/// Uses the unified tool registry for validation and routing.
/// This ensures tool definitions are in a single place.
pub struct ToolDispatcher {
    /// Set of registered tool names for O(1) lookup
    registered_tools: std::collections::HashSet<String>,
}

impl ToolDispatcher {
    /// Create a new dispatcher using the unified tool registry
    pub fn new() -> Self {
        // 确保搜索系统已初始化（MCP stdio 模式下可能未启动 daemon）
        ensure_search_system_initialized();
        
        // 从统一注册表获取所有工具名
        let tool_names = crate::mcp::tool_registry::get_all_tool_names();
        let registered_tools: std::collections::HashSet<String> = 
            tool_names.into_iter().map(String::from).collect();

        Self { registered_tools }
    }

    /// Check if a tool is registered (O(1))
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.registered_tools.contains(tool_name)
    }

    /// Get the list of registered tool names
    pub fn list_tool_names(&self) -> Vec<String> {
        self.registered_tools.iter().cloned().collect()
    }

    /// Dispatch a tool call
    ///
    /// This uses match instead of HashMap<closure> to avoid async lifetime issues
    pub async fn dispatch(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        // Fast O(1) validation
        if !self.has_tool(tool_name) {
            return Err(McpError::invalid_request(
                format!("Unknown tool: {}", tool_name),
                None,
            ));
        }

        // Dispatch to handlers
        match tool_name {
            "interact" => Self::handle_interact(args).await,
            "memory" => Self::handle_memory(args).await,
            "search" => Self::handle_search(args).await,
            "health" => Self::handle_health(args).await,

            #[cfg(feature = "experimental-neurospec")]
            name if name.starts_with("neurospec_") => Self::handle_neurospec(name, args).await,

            _ => Err(McpError::invalid_request(
                format!("Unknown tool: {}", tool_name),
                None,
            )),
        }
    }

    /// Handle interact tool
    async fn handle_interact(args: serde_json::Value) -> Result<CallToolResult, McpError> {
        let req: InteractRequest = serde_json::from_value(args)
            .map_err(|e| invalid_params_error(format!("Failed to parse parameters: {}", e)))?;
        InteractionTool::interact(req).await
    }

    /// Handle memory tool
    async fn handle_memory(args: serde_json::Value) -> Result<CallToolResult, McpError> {
        // 首先尝试解析为 MemoryRequest
        if let Ok(req) = serde_json::from_value::<MemoryRequest>(args.clone()) {
            return Ok(MemoryTool::manage_memory(req).await?);
        }

        // 检查是否是特殊的无参数请求（如计划确认）
        let args_map: serde_json::Map<String, serde_json::Value> = serde_json::from_value(args)
            .map_err(|e| invalid_params_error(format!("Failed to parse parameters: {}", e)))?;

        // 检查 action 字段
        if let Some(action) = args_map.get("action") {
            let action_str = action.as_str().unwrap_or("");

            match action_str {
                // 确认重构计划
                "plan_confirm" => {
                    return Ok(MemoryTool::confirm_refactor_plan().await?);
                }
                // 获取记忆建议
                "suggest_memory" => {
                    let messages: Vec<String> = args_map
                        .get("messages")
                        .cloned()
                        .and_then(|v| serde_json::from_value(v).ok())
                        .unwrap_or_default();
                    let project_path = args_map
                        .get("project_path")
                        .and_then(|v| v.as_str().map(|s| s.to_string()));

                    return Ok(MemoryTool::get_memory_suggestions(messages, project_path).await?);
                }
                // 记录记忆使用
                "record_usage" => {
                    let memory_id: String = args_map
                        .get("memory_id")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .ok_or_else(|| {
                            invalid_params_error("Missing memory_id for record_usage action".to_string())
                        })?;

                    return Ok(MemoryTool::record_memory_usage(memory_id).await?);
                }
                // 获取相关记忆
                "get_related" => {
                    let query: String = args_map
                        .get("query")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .ok_or_else(|| invalid_params_error("Missing query for get_related action".to_string()))?;

                    let existing_memories: Vec<crate::mcp::tools::memory::types::MemoryEntry> = args_map
                        .get("memories")
                        .cloned()
                        .and_then(|v| serde_json::from_value(v).ok())
                        .unwrap_or_default();

                    return Ok(MemoryTool::get_related_memories(query, existing_memories).await?);
                }
                _ => {
                    return Err(invalid_params_error(format!(
                        "Unknown memory action: {}. Supported actions: remember, recall, plan_confirm, suggest_memory, record_usage, get_related",
                        action_str
                    )).into());
                }
            }
        }

        // 如果无法识别，返回错误
        Err(invalid_params_error(
            "Invalid memory tool request. Expected MemoryRequest or valid action".to_string()
        ).into())
    }

    /// Handle search tool
    async fn handle_search(args: serde_json::Value) -> Result<CallToolResult, McpError> {
        // 预处理：如果 profile 字段是字符串，尝试解析为 JSON 对象
        // 这是为了兼容某些 MCP 客户端（如 Cascade）把嵌套对象序列化为字符串的情况
        let args = Self::preprocess_search_args(args);
        
        let req: crate::mcp::tools::acemcp::types::SearchRequest = serde_json::from_value(args)
            .map_err(|e| invalid_params_error(format!("Failed to parse parameters: {}", e)))?;
        Ok(AcemcpTool::search_context(req).await?)
    }
    
    /// Handle health tool
    async fn handle_health(args: serde_json::Value) -> Result<CallToolResult, McpError> {
        let req: crate::mcp::tools::acemcp::health::HealthRequest = serde_json::from_value(args)
            .map_err(|e| invalid_params_error(format!("Failed to parse parameters: {}", e)))?;
        Ok(crate::mcp::tools::acemcp::health::check_health(req).await?)
    }

    /// 预处理 search 参数，修复 profile 字段可能被序列化为字符串的问题
    fn preprocess_search_args(mut args: serde_json::Value) -> serde_json::Value {
        if let serde_json::Value::Object(ref mut map) = args {
            // 处理 profile 字段
            if let Some(profile_val) = map.get("profile").cloned() {
                if let serde_json::Value::String(profile_str) = profile_val {
                    // 尝试把字符串解析为 JSON 对象
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&profile_str) {
                        map.insert("profile".to_string(), parsed);
                        crate::log_important!(info, "[MCP] Preprocessed profile from string to object");
                    }
                }
            }
            
            // 处理 scope 字段（如果 profile.smart_structure.scope 也是字符串）
            if let Some(serde_json::Value::Object(ref mut profile_obj)) = map.get_mut("profile") {
                if let Some(serde_json::Value::Object(ref mut ss_obj)) = profile_obj.get_mut("smart_structure") {
                    if let Some(scope_val) = ss_obj.get("scope").cloned() {
                        if let serde_json::Value::String(scope_str) = scope_val {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&scope_str) {
                                ss_obj.insert("scope".to_string(), parsed);
                            }
                        }
                    }
                }
            }
        }
        args
    }

    /// Handle NeuroSpec tools
    #[cfg(feature = "experimental-neurospec")]
    async fn handle_neurospec(
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let args_map = match args {
            serde_json::Value::Object(map) => Some(map),
            _ => None,
        };
        crate::neurospec::tools::handle_neurospec_tool(tool_name, args_map).await
    }
}

impl Default for ToolDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
