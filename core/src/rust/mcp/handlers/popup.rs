use anyhow::Result;

use crate::mcp::types::{PopupRequest, InteractRequest};
use crate::daemon::{DaemonClient, DaemonRequest};
use crate::{log_important, log_debug};
use crate::mcp::utils::errors::daemon_connection_error;

/// 创建 Tauri 弹窗
///
/// 通过 HTTP 与 NeuroSpec daemon 通信，显示弹窗并等待用户响应
pub async fn create_tauri_popup(request: &PopupRequest) -> Result<String> {
    log_debug!("Creating popup via daemon HTTP client");
    
    // 创建 daemon 客户端
    let client = DaemonClient::new(None);
    
    // 构造交互请求
    let interact_request = InteractRequest {
        message: request.message.clone(),
        predefined_options: request.predefined_options.clone().unwrap_or_default(),
        is_markdown: request.is_markdown,
        // TODO: Add timeout parameter when updating InteractRequest struct
    };
    
    let daemon_request = DaemonRequest::Interact(interact_request);
    
    // 直接异步执行请求
    let response = client.execute_tool(daemon_request).await.map_err(|e| {
        // 检查是否为 reqwest 连接错误
        if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
             if reqwest_err.is_connect() || reqwest_err.is_timeout() {
                 log_important!(error, "Daemon connection failed: {}", reqwest_err);
                 return anyhow::Error::new(daemon_connection_error("NeuroSpec Daemon not running or unreachable"));
             }
        }

        // Fallback: 尝试检查 IO 错误 (std::io::Error)
        // 有些 reqwest 错误可能被封装，或者如果是其他网络库
        let error_msg = e.to_string();
        if error_msg.contains("connect") || error_msg.contains("refused") || error_msg.contains("timeout") {
             log_important!(error, "Daemon connection failed (msg match): {}", error_msg);
             return anyhow::Error::new(daemon_connection_error("NeuroSpec Daemon not running or unreachable"));
        }

        log_important!(error, "Daemon request failed: {}", e);
        e
    })?;
    
    // 从响应中提取结果
    if let Some(result) = response.data {
        // 结果是 CallToolResult 的 JSON 表示
        // 需要提取其中的文本内容
        if let Some(contents) = result.get("content").and_then(|v| v.as_array()) {
            let mut text_parts = Vec::new();
            for content in contents {
                if let Some(text) = content.get("text").and_then(|v| v.as_str()) {
                    text_parts.push(text.to_string());
                }
            }
            if !text_parts.is_empty() {
                return Ok(text_parts.join("\n"));
            }
        }
        
        // 如果无法解析内容，返回整个 JSON
        Ok(result.to_string())
    } else {
        Ok("用户取消了操作".to_string())
    }
}