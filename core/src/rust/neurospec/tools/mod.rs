//! NeuroSpec 高级工具（重构辅助）
//!
//! 提供依赖影响分析和跨文件重命名功能

use rmcp::{
    model::CallToolResult,
    ErrorData as McpError,
};

pub mod graph_tools;
pub mod refactor_tools;

pub use graph_tools::ImpactAnalysisArgs;
pub use refactor_tools::RenameArgs;

/// 处理 NeuroSpec 工具调用
pub async fn handle_neurospec_tool(
    name: &str,
    arguments: Option<serde_json::Map<String, serde_json::Value>>,
) -> Result<CallToolResult, McpError> {
    let args = arguments.unwrap_or_default();

    let content = match name {
        "neurospec_graph_impact_analysis" => {
            let args: ImpactAnalysisArgs = serde_json::from_value(serde_json::Value::Object(args))
                .map_err(|e| {
                    McpError::invalid_params(format!("Invalid parameters: {}", e), None)
                })?;

            graph_tools::handle_impact_analysis(args)?
        }
        "neurospec_refactor_rename" => {
            let args: RenameArgs = serde_json::from_value(serde_json::Value::Object(args))
                .map_err(|e| {
                    McpError::invalid_params(format!("Invalid parameters: {}", e), None)
                })?;

            refactor_tools::handle_rename(args)?
        }
        _ => {
            return Err(McpError::invalid_request(
                format!("Unknown tool: {}", name),
                None,
            ))
        }
    };

    Ok(CallToolResult {
        content,
        is_error: None,
        meta: None,
        structured_content: None,
    })
}
