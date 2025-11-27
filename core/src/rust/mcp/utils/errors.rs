/// MCP 错误处理工具模块
///
/// 提供统一的错误处理和转换功能
use rmcp::{model::ErrorCode, ErrorData as McpError};

/// MCP 错误类型枚举
#[derive(Debug, thiserror::Error)]
pub enum McpToolError {
    #[error("项目路径错误: {0}")]
    ProjectPath(String),

    #[error("弹窗创建失败: {0}")]
    PopupCreation(String),

    #[error("响应解析失败: {0}")]
    ResponseParsing(String),

    #[error("记忆管理错误: {0}")]
    Memory(String),

    #[error("Daemon连接错误: {0}")]
    DaemonConnection(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 序列化错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("无效参数: {0}")]
    InvalidParams(String),

    #[error("通用错误: {0}")]
    Generic(#[from] anyhow::Error),
}

impl From<McpToolError> for McpError {
    fn from(error: McpToolError) -> Self {
        match error {
            McpToolError::ProjectPath(msg) | McpToolError::InvalidParams(msg) => {
                McpError::invalid_params(msg, None)
            }
            McpToolError::DaemonConnection(msg) => {
                // 使用 JSON-RPC 错误码 -32000 表示 daemon 未运行
                McpError::new(ErrorCode(-32000), msg, None)
            }
            McpToolError::PopupCreation(msg)
            | McpToolError::ResponseParsing(msg)
            | McpToolError::Memory(msg) => McpError::internal_error(msg, None),
            McpToolError::Io(e) => McpError::internal_error(format!("IO 错误: {}", e), None),
            McpToolError::Json(e) => McpError::internal_error(format!("JSON 错误: {}", e), None),
            McpToolError::Generic(e) => {
                // 检查是否为 daemon 连接错误
                let error_str = e.to_string();
                if error_str.contains("NeuroSpec Daemon not running")
                    || error_str.contains("Failed to connect to NeuroSpec daemon")
                {
                    McpError::new(
                        ErrorCode(-32000),
                        "NeuroSpec Daemon not running".to_string(),
                        None,
                    )
                } else {
                    McpError::internal_error(error_str, None)
                }
            }
        }
    }
}

/// 创建项目路径错误
pub fn project_path_error(msg: impl Into<String>) -> McpToolError {
    McpToolError::ProjectPath(msg.into())
}

/// 创建弹窗错误
pub fn popup_error(msg: impl Into<String>) -> McpToolError {
    McpToolError::PopupCreation(msg.into())
}

/// 创建响应解析错误
pub fn response_error(msg: impl Into<String>) -> McpToolError {
    McpToolError::ResponseParsing(msg.into())
}

/// 创建记忆管理错误
pub fn memory_error(msg: impl Into<String>) -> McpToolError {
    McpToolError::Memory(msg.into())
}

/// 创建 Daemon 连接错误
pub fn daemon_connection_error(msg: impl Into<String>) -> McpToolError {
    McpToolError::DaemonConnection(msg.into())
}

/// 创建无效参数错误
pub fn invalid_params_error(msg: impl Into<String>) -> McpToolError {
    McpToolError::InvalidParams(msg.into())
}
