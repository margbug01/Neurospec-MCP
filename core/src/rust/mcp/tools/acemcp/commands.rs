use tauri::AppHandle;

use super::AcemcpTool;
use super::types::AcemcpRequest;

#[derive(Debug, serde::Serialize)]
pub struct DebugSearchResult {
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<String>,
}

/// 调试命令：执行本地搜索
#[tauri::command]
pub async fn debug_acemcp_search(
    project_root_path: String,
    query: String,
    _app: AppHandle,
) -> Result<DebugSearchResult, String> {
    let req = AcemcpRequest {
        project_root_path: Some(project_root_path),
        query,
        mode: None,
        profile: None,
    };

    let search_result = AcemcpTool::search_context(req).await;

    match search_result {
        Ok(result) => {
            let mut result_text = String::new();
            if let Ok(val) = serde_json::to_value(&result) {
                if let Some(arr) = val.get("content").and_then(|v| v.as_array()) {
                    for item in arr {
                        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                            if let Some(txt) = item.get("text").and_then(|t| t.as_str()) {
                                result_text.push_str(txt);
                            }
                        }
                    }
                }
            }

            Ok(DebugSearchResult {
                success: true,
                result: Some(result_text),
                error: None,
            })
        }
        Err(e) => Ok(DebugSearchResult {
            success: false,
            result: None,
            error: Some(format!("执行失败: {}", e)),
        }),
    }
}

/// 执行搜索工具
#[tauri::command]
pub async fn execute_acemcp_tool(
    tool_name: String,
    arguments: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match tool_name.as_str() {
        "search_context" | "search" => {
            let project_root_path = arguments
                .get("project_root_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let query = arguments
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "缺少 query 参数".to_string())?
                .to_string();

            let req = AcemcpRequest {
                project_root_path,
                query,
                mode: None,
                profile: None,
            };

            match AcemcpTool::search_context(req).await {
                Ok(result) => {
                    if let Ok(val) = serde_json::to_value(&result) {
                        Ok(serde_json::json!({
                            "status": "success",
                            "result": val
                        }))
                    } else {
                        Err("结果序列化失败".to_string())
                    }
                }
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "error": e.to_string()
                })),
            }
        }
        _ => Err(format!("未知的工具: {}", tool_name)),
    }
}

/// 清除本地索引缓存
#[tauri::command]
pub async fn clear_acemcp_cache() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取 HOME 目录")?;
    let cache_dir = home.join(".acemcp").join("local_index");
    
    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    
    log::info!("本地索引缓存已清除: {:?}", cache_dir);
    Ok(cache_dir.to_string_lossy().to_string())
}
