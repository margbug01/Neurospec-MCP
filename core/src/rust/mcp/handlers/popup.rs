use anyhow::Result;

use crate::mcp::types::{PopupRequest, InteractRequest};
use crate::daemon::{DaemonClient, DaemonRequest, DaemonResponse};
use super::ws_client;
use crate::{log_important, log_debug};
use crate::mcp::utils::errors::daemon_connection_error;

/// 检查是否启用 WebSocket
fn is_websocket_enabled() -> bool {
    match crate::config::load_standalone_config() {
        Ok(config) => config.daemon_config.enable_websocket,
        Err(_) => true, // 默认启用
    }
}

/// 创建 Tauri 弹窗
///
/// 优先通过 WebSocket 与 Daemon 通信，失败后降级到 HTTP
pub async fn create_tauri_popup(request: &PopupRequest) -> Result<String> {
    // 构造交互请求
    let interact_request = InteractRequest {
        message: request.message.clone(),
        predefined_options: request.predefined_options.clone().unwrap_or_default(),
        is_markdown: request.is_markdown,
    };
    
    let daemon_request = DaemonRequest::Interact(interact_request);
    
    // 策略：WS 优先，HTTP 降级
    let ws_enabled = is_websocket_enabled();
    log_important!(info, "[Popup] WebSocket enabled: {}", ws_enabled);
    
    let response = if ws_enabled {
        // 尝试 WebSocket
        log_important!(info, "[Popup] Attempting WebSocket connection to daemon...");
        match ws_client::execute_via_ws(daemon_request.clone()).await {
            Ok(resp) => {
                log_important!(info, "[Popup] WebSocket request successful!");
                resp
            }
            Err(ws_err) => {
                // WS 失败，降级到 HTTP
                log_important!(warn, "[Popup] WebSocket failed: {}, falling back to HTTP", ws_err);
                execute_via_http(daemon_request).await?
            }
        }
    } else {
        // WebSocket 禁用，直接使用 HTTP
        log_important!(info, "[Popup] WebSocket disabled, using HTTP");
        execute_via_http(daemon_request).await?
    };
    
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

/// 通过 HTTP 执行 Daemon 请求
async fn execute_via_http(request: DaemonRequest) -> Result<DaemonResponse> {
    log_debug!("Executing request via HTTP client");
    
    let client = DaemonClient::new(None);
    
    client.execute_tool(request).await.map_err(|e| {
        // 检查是否为 reqwest 连接错误
        if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
            if reqwest_err.is_connect() || reqwest_err.is_timeout() {
                log_important!(error, "Daemon HTTP connection failed: {}", reqwest_err);
                return anyhow::Error::new(daemon_connection_error(
                    "NeuroSpec Daemon not running or unreachable"
                ));
            }
        }

        // Fallback: 字符串匹配检查连接错误
        let error_msg = e.to_string();
        if error_msg.contains("connect") || error_msg.contains("refused") || error_msg.contains("timeout") {
            log_important!(error, "Daemon HTTP connection failed (msg match): {}", error_msg);
            return anyhow::Error::new(daemon_connection_error(
                "NeuroSpec Daemon not running or unreachable"
            ));
        }

        log_important!(error, "Daemon HTTP request failed: {}", e);
        e
    })
}