//! WebSocket 处理器 - 实现 MCP↔Daemon 的持久通信
//!
//! 提供比 HTTP 更稳定的长连接通信方式

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::types::{DaemonRequest, DaemonResponse};
use super::routes::DaemonAppState;
use crate::{log_important, log_debug};

/// WebSocket 消息格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// 请求消息
    #[serde(rename = "request")]
    Request {
        id: String,
        payload: DaemonRequest,
    },
    /// 响应消息
    #[serde(rename = "response")]
    Response {
        id: String,
        payload: DaemonResponse,
    },
    /// 心跳 ping
    #[serde(rename = "ping")]
    Ping,
    /// 心跳 pong
    #[serde(rename = "pong")]
    Pong,
    /// 错误消息
    #[serde(rename = "error")]
    Error {
        id: Option<String>,
        message: String,
    },
}

/// 最大消息大小（10MB）- 支持大图片响应
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// WebSocket 升级处理
pub async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<DaemonAppState>>,
) -> impl IntoResponse {
    log_important!(info, "[WebSocket] New connection upgrade request, max_message_size={}MB", MAX_MESSAGE_SIZE / 1024 / 1024);
    // 配置大消息支持
    ws.max_message_size(MAX_MESSAGE_SIZE)
        .on_upgrade(move |socket| handle_ws_connection(socket, state))
}

/// 全局连接计数器
static CONNECTION_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 处理 WebSocket 连接
async fn handle_ws_connection(socket: WebSocket, state: Arc<DaemonAppState>) {
    let conn_id = CONNECTION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    log_important!(info, "[WebSocket][Conn#{}] Connection established", conn_id);
    
    let (mut sender, mut receiver) = socket.split();
    
    // 发送欢迎消息
    let welcome = serde_json::json!({
        "type": "connected",
        "message": "NeuroSpec Daemon WebSocket connected"
    });
    if let Err(e) = sender.send(Message::Text(welcome.to_string())).await {
        log_important!(error, "[WebSocket] Failed to send welcome: {}", e);
        return;
    }
    
    // 创建响应发送通道
    let (resp_tx, mut resp_rx) = tokio::sync::mpsc::channel::<String>(100);
    
    // 心跳定时器 - 15秒间隔，与客户端更同步
    let mut heartbeat_interval = tokio::time::interval(std::time::Duration::from_secs(15));
    
    // 主消息处理循环
    loop {
        tokio::select! {
            // 接收客户端消息
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        log_debug!("[WebSocket] Received: {}", &text[..text.len().min(200)]);
                        
                        match serde_json::from_str::<WsMessage>(&text) {
                            Ok(ws_msg) => {
                                // 快速响应（ping/pong）直接处理
                                if matches!(ws_msg, WsMessage::Ping | WsMessage::Pong) {
                                    if let Some(resp) = handle_ws_message(ws_msg, &state).await {
                                        let resp_text = serde_json::to_string(&resp).unwrap_or_default();
                                        if let Err(e) = sender.send(Message::Text(resp_text)).await {
                                            log_important!(error, "[WebSocket] Failed to send response: {}", e);
                                            break;
                                        }
                                    }
                                } else {
                                    // 长时间请求异步处理，不阻塞消息循环
                                    let state_clone = state.clone();
                                    let resp_tx_clone = resp_tx.clone();
                                    let conn_id_clone = conn_id;
                                    tokio::spawn(async move {
                                        log_important!(info, "[WebSocket][Conn#{}] Starting async request processing...", conn_id_clone);
                                        let response = handle_ws_message(ws_msg, &state_clone).await;
                                        if let Some(resp) = response {
                                            let resp_text = serde_json::to_string(&resp).unwrap_or_default();
                                            log_important!(info, "[WebSocket][Conn#{}] Async response ready, length={}, sending to channel...", conn_id_clone, resp_text.len());
                                            match resp_tx_clone.send(resp_text).await {
                                                Ok(_) => log_important!(info, "[WebSocket][Conn#{}] Async response sent to channel successfully", conn_id_clone),
                                                Err(e) => log_important!(error, "[WebSocket][Conn#{}] Failed to send async response to channel: {}", conn_id_clone, e),
                                            }
                                        } else {
                                            log_important!(warn, "[WebSocket][Conn#{}] handle_ws_message returned None for request", conn_id_clone);
                                        }
                                    });
                                }
                            }
                            Err(e) => {
                                log_important!(warn, "[WebSocket] Failed to parse message: {}", e);
                                let error = WsMessage::Error {
                                    id: None,
                                    message: format!("Invalid message format: {}", e),
                                };
                                let _ = sender.send(Message::Text(serde_json::to_string(&error).unwrap_or_default())).await;
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        log_debug!("[WebSocket] Received ping");
                        if let Err(e) = sender.send(Message::Pong(data)).await {
                            log_important!(error, "[WebSocket] Failed to send pong: {}", e);
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        log_debug!("[WebSocket] Received pong");
                    }
                    Some(Ok(Message::Close(_))) => {
                        log_important!(info, "[WebSocket][Conn#{}] Client closed connection", conn_id);
                        break;
                    }
                    Some(Err(e)) => {
                        log_important!(error, "[WebSocket][Conn#{}] Error receiving message: {}", conn_id, e);
                        break;
                    }
                    None => {
                        log_important!(info, "[WebSocket][Conn#{}] Connection closed (stream ended)", conn_id);
                        break;
                    }
                    _ => {}
                }
            }
            
            // 发送异步响应
            Some(resp_text) = resp_rx.recv() => {
                log_important!(info, "[WebSocket][Conn#{}] Received async response from channel, length={}", conn_id, resp_text.len());
                // 打印响应预览以便调试
                let preview = if resp_text.len() > 200 { &resp_text[..200] } else { &resp_text };
                log_important!(info, "[WebSocket][Conn#{}] Response preview: {}", conn_id, preview);
                
                match sender.send(Message::Text(resp_text.clone())).await {
                    Ok(_) => {
                        log_important!(info, "[WebSocket][Conn#{}] Async response sent to client successfully", conn_id);
                    }
                    Err(e) => {
                        log_important!(error, "[WebSocket][Conn#{}] Failed to send async response: {}", conn_id, e);
                        break;
                    }
                }
            }
            
            // 发送心跳
            _ = heartbeat_interval.tick() => {
                let ping = WsMessage::Ping;
                let ping_text = serde_json::to_string(&ping).unwrap_or_default();
                if let Err(e) = sender.send(Message::Text(ping_text)).await {
                    log_important!(error, "[WebSocket] Failed to send heartbeat: {}", e);
                    break;
                }
                log_debug!("[WebSocket] Sent heartbeat ping");
            }
        }
    }
    
    log_important!(info, "[WebSocket][Conn#{}] Connection handler finished", conn_id);
}

/// 处理 WebSocket 消息
async fn handle_ws_message(msg: WsMessage, state: &Arc<DaemonAppState>) -> Option<WsMessage> {
    match msg {
        WsMessage::Request { id, payload } => {
            log_important!(info, "[WebSocket] Processing request: {}", id);
            
            // 使用抽取的公共请求处理逻辑
            let response = super::routes::process_daemon_request(payload, state).await;
            
            Some(WsMessage::Response {
                id,
                payload: response,
            })
        }
        WsMessage::Ping => {
            Some(WsMessage::Pong)
        }
        WsMessage::Pong => {
            // 收到 pong，连接正常
            None
        }
        _ => None,
    }
}
