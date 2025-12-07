
use anyhow::Result;
use rmcp::{
    model::*, service::RequestContext, transport::stdio, ErrorData as McpError, RoleServer,
    ServerHandler, ServiceExt,
};
use std::collections::HashMap;

use super::dispatcher::ToolDispatcher;
use crate::config::load_standalone_config;
use crate::{log_debug, log_important};

pub struct ZhiServer {
    enabled_tools: HashMap<String, bool>,
    dispatcher: std::sync::Arc<ToolDispatcher>,
}

impl Default for ZhiServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ZhiServer {
    pub fn new() -> Self {
        // 尝试加载配置，如果失败则使用默认配置
        let enabled_tools = match load_standalone_config() {
            Ok(config) => config.mcp_config.tools,
            Err(e) => {
                log_important!(warn, "无法加载配置文件，使用默认工具配置: {}", e);
                crate::config::default_mcp_tools()
            }
        };

        Self {
            enabled_tools,
            dispatcher: std::sync::Arc::new(ToolDispatcher::new()),
        }
    }

    /// 检查工具是否启用 - 使用缓存配置
    /// 注意：配置更新通过配置监听器自动重新加载
    fn is_tool_enabled(&self, tool_name: &str) -> bool {
        let enabled = self.enabled_tools.get(tool_name).copied().unwrap_or(true);
        log_debug!("工具 {} 当前状态: {}", tool_name, enabled);
        enabled
    }
}

impl ServerHandler for ZhiServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: crate::mcp::create_implementation(
                "NeuroSpec-MCP".to_string(),
                env!("CARGO_PKG_VERSION").to_string(),
            ),
            instructions: Some("NeuroSpec - AI-powered development assistant.\n\nIMPORTANT WORKFLOW RULE: You MUST use the `search` tool with `mode='symbol'` or `mode='text'` to locate relevant code BEFORE using any modification tools or asking the user for decisions. Do not rely on memory or assumptions. Always ground your actions in the codebase reality.".to_string()),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ServerInfo, McpError> {
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        // 使用统一工具注册表构建工具列表
        let tools = crate::mcp::tool_registry::build_enabled_tools(|name| {
            self.is_tool_enabled(name)
        });

        log_debug!(
            "返回给客户端的工具列表: {:?}",
            tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        log_debug!("收到工具调用请求: {}", request.name);

        // Convert arguments to Value for dispatcher
        let arguments_value = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        // Use dispatcher for O(1) lookup and routing
        self.dispatcher
            .dispatch(&request.name, arguments_value)
            .await
    }
}

/// 启动MCP服务器
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    // 创建并运行服务器
    let service = ZhiServer::new().serve(stdio()).await.inspect_err(|e| {
        log_important!(error, "启动服务器失败: {}", e);
    })?;

    // 等待服务器关闭
    service.waiting().await?;
    Ok(())
}
