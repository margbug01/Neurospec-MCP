//! 搜索引擎健康检查工具

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use rmcp::model::{CallToolResult, Content};
use crate::mcp::utils::errors::McpToolError;
use crate::mcp::tools::unified_store::{
    is_search_initialized, assess_index_health, IndexHealth,
    get_index_state, is_project_indexing,
};
use super::local_engine::ripgrep::RipgrepSearcher;

/// neurospec.health 工具请求参数
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HealthRequest {
    /// 项目根目录（可选，默认当前目录）
    pub project_root: Option<String>,
}

/// 健康检查响应
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// 索引状态：NotIndexed/Indexing/Ready/Corrupted/Stale
    pub index_state: String,
    /// 已索引文件数
    pub indexed_files: usize,
    /// 嵌入/语义搜索是否可用
    pub embedding_available: bool,
    /// 上次索引时间（ISO 8601）
    pub last_indexed_at: Option<String>,
    /// 索引健康状态
    pub index_health: String,
    /// 是否正在索引
    pub is_indexing: bool,
    /// 可用引擎列表
    pub engines: EngineStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineStatus {
    pub tantivy: bool,
    pub ripgrep: bool,
    pub ctags: bool,
}

/// 执行健康检查
pub async fn check_health(request: HealthRequest) -> Result<CallToolResult, McpToolError> {
    let project_root = if let Some(root) = request.project_root {
        PathBuf::from(root)
    } else {
        std::env::current_dir()?
    };
    
    if !project_root.exists() {
        return Err(McpToolError::InvalidParams(format!(
            "Project root does not exist: {}",
            project_root.display()
        )));
    }
    
    // 收集健康信息
    let index_state_info = get_index_state(&project_root);
    let health = assess_index_health(&project_root);
    let is_indexing = is_project_indexing(&project_root);
    
    let (state_str, file_count, last_indexed) = if let Some(state) = index_state_info {
        let state_name = if state.is_indexing() {
            "Indexing"
        } else if state.is_ready() {
            "Ready"
        } else {
            "NotIndexed"
        };
        
        let timestamp = state.last_indexed_ts.map(|ts| {
            use std::time::{UNIX_EPOCH, Duration};
            let datetime = UNIX_EPOCH + Duration::from_secs(ts);
            format_timestamp(datetime)
        });
        
        (state_name.to_string(), state.get_file_count(), timestamp)
    } else {
        ("NotIndexed".to_string(), 0, None)
    };
    
    let health_str = match health {
        IndexHealth::Healthy => "Healthy",
        IndexHealth::Degraded { .. } => "Degraded",
        IndexHealth::Unhealthy { .. } => "Unhealthy",
    };
    
    let response = HealthResponse {
        index_state: state_str,
        indexed_files: file_count,
        embedding_available: false, // TODO: 检测嵌入服务
        last_indexed_at: last_indexed,
        index_health: health_str.to_string(),
        is_indexing,
        engines: EngineStatus {
            tantivy: is_search_initialized(),
            ripgrep: RipgrepSearcher::is_available(),
            ctags: super::local_engine::ctags::CtagsIndexer::is_available(),
        },
    };
    
    let json = serde_json::to_string_pretty(&response)?;
    
    Ok(crate::mcp::create_success_result(vec![Content::text(json)]))
}

/// 格式化时间戳为 ISO 8601
fn format_timestamp(datetime: std::time::SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    
    if let Ok(duration) = datetime.duration_since(UNIX_EPOCH) {
        let secs = duration.as_secs();
        let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0);
        if let Some(dt) = datetime {
            return dt.to_rfc3339();
        }
    }
    
    "unknown".to_string()
}
