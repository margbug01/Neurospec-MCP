//! WebSocket 客户端 - MCP Server 与 Daemon 的持久连接
//!
//! 提供自动重连、心跳和请求/响应匹配

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_tungstenite::{connect_async_with_config, tungstenite::{Message, protocol::WebSocketConfig}};

use crate::daemon::types::{DaemonRequest, DaemonResponse};
use crate::daemon::server::DEFAULT_DAEMON_PORT;
use crate::{log_important, log_debug};

/// WebSocket 消息格式（与服务端一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    #[serde(rename = "request")]
    Request {
        id: String,
        payload: DaemonRequest,
    },
    #[serde(rename = "response")]
    Response {
        id: String,
        payload: DaemonResponse,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "connected")]
    Connected { message: String },
    #[serde(rename = "error")]
    Error {
        id: Option<String>,
        message: String,
    },
}

/// 待处理的请求
struct PendingRequest {
    sender: oneshot::Sender<DaemonResponse>,
}

/// WebSocket 客户端状态
struct WsClientState {
    pending: HashMap<String, PendingRequest>,
    connected: bool,
    sender: Option<mpsc::Sender<String>>,
    last_message_time: std::time::Instant,
}

/// 心跳间隔（秒）
const HEARTBEAT_INTERVAL_SECS: u64 = 10;
/// 连接超时（秒）- 如果这么长时间没有收到任何消息，认为连接断开
const CONNECTION_TIMEOUT_SECS: u64 = 35;
/// 最大消息大小（10MB）- 支持大图片响应
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

lazy_static::lazy_static! {
    /// 全局 WebSocket 客户端
    static ref WS_CLIENT: Arc<RwLock<WsClientState>> = Arc::new(RwLock::new(WsClientState {
        pending: HashMap::new(),
        connected: false,
        sender: None,
        last_message_time: std::time::Instant::now(),
    }));
}

/// 初始化标记
static INIT_STARTED: std::sync::atomic::AtomicBool = 
    std::sync::atomic::AtomicBool::new(false);

/// 初始化 WebSocket 连接
pub async fn init_ws_connection() -> Result<()> {
    // 避免重复初始化
    if INIT_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Ok(());
    }
    
    tokio::spawn(async {
        ws_connection_loop().await;
    });
    
    // 等待连接建立（最多 5 秒）
    for _ in 0..50 {
        {
            let state = WS_CLIENT.read().await;
            if state.connected {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    log_important!(warn, "[WsClient] Connection not established within timeout, will retry in background");
    Ok(())
}

/// WebSocket 连接循环（自动重连）
async fn ws_connection_loop() {
    let mut retry_delay = Duration::from_secs(1);
    let max_retry_delay = Duration::from_secs(30);
    
    loop {
        let url = format!("ws://127.0.0.1:{}/ws", DEFAULT_DAEMON_PORT);
        log_important!(info, "[WsClient] Connecting to {}", url);
        
        // 配置 WebSocket 允许大消息
        let ws_config = WebSocketConfig {
            max_message_size: Some(MAX_MESSAGE_SIZE),
            max_frame_size: Some(MAX_MESSAGE_SIZE),
            ..Default::default()
        };
        
        match connect_async_with_config(&url, Some(ws_config), false).await {
            Ok((ws_stream, _)) => {
                log_important!(info, "[WsClient] Connected successfully");
                retry_delay = Duration::from_secs(1); // 重置重试延迟
                
                let (write, read) = ws_stream.split();
                let (tx, rx) = mpsc::channel::<String>(100);
                
                // 更新状态
                {
                    let mut state = WS_CLIENT.write().await;
                    state.connected = true;
                    state.sender = Some(tx);
                    state.last_message_time = std::time::Instant::now(); // 重置超时计时器
                }
                
                // 运行连接处理
                handle_connection(write, read, rx).await;
                
                // 连接断开，清理状态
                log_important!(warn, "[WsClient] Connection closed, will reconnect");
                {
                    let mut state = WS_CLIENT.write().await;
                    state.connected = false;
                    state.sender = None;
                    // 清理所有 pending 请求，发送错误响应
                    let pending_count = state.pending.len();
                    if pending_count > 0 {
                        log_important!(warn, "[WsClient] Cleaning up {} pending requests due to disconnection", pending_count);
                        for (id, pending) in state.pending.drain() {
                            log_important!(warn, "[WsClient] Canceling pending request: {}", id);
                            let _ = pending.sender.send(DaemonResponse::error("WebSocket disconnected".to_string()));
                        }
                    }
                }
            }
            Err(e) => {
                log_important!(error, "[WsClient] Connection failed: {}", e);
            }
        }
        
        // 等待后重试
        log_important!(info, "[WsClient] Reconnecting in {:?}", retry_delay);
        tokio::time::sleep(retry_delay).await;
        retry_delay = std::cmp::min(retry_delay * 2, max_retry_delay);
    }
}

/// 处理 WebSocket 连接
async fn handle_connection<S, R>(
    mut write: S,
    mut read: R,
    mut rx: mpsc::Receiver<String>,
) where
    S: SinkExt<Message> + Unpin,
    R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    // 心跳定时器 - 更频繁的心跳
    let mut heartbeat = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
    // 连接健康检查定时器
    let mut health_check = tokio::time::interval(Duration::from_secs(5));
    
    log_important!(info, "[WsClient] Connection handler started, heartbeat={}s, timeout={}s", 
        HEARTBEAT_INTERVAL_SECS, CONNECTION_TIMEOUT_SECS);
    
    loop {
        tokio::select! {
            // 发送请求
            Some(msg) = rx.recv() => {
                log_important!(info, "[WsClient] Sending message to server, length={}", msg.len());
                if write.send(Message::Text(msg)).await.is_err() {
                    log_important!(error, "[WsClient] Failed to send message, connection broken");
                    break;
                }
            }
            
            // 接收响应
            Some(msg) = read.next() => {
                // 更新最后消息时间
                {
                    let mut state = WS_CLIENT.write().await;
                    state.last_message_time = std::time::Instant::now();
                }
                
                log_important!(info, "[WsClient] Received raw message from server");
                match msg {
                    Ok(Message::Text(text)) => {
                        log_important!(info, "[WsClient] Text message received, length={}", text.len());
                        handle_message(&text).await;
                    }
                    Ok(Message::Binary(data)) => {
                        // 处理二进制消息（可能是大消息被转为 binary）
                        log_important!(info, "[WsClient] Binary message received, length={}", data.len());
                        if let Ok(text) = String::from_utf8(data) {
                            log_important!(info, "[WsClient] Converted binary to text, processing...");
                            handle_message(&text).await;
                        } else {
                            log_important!(error, "[WsClient] Failed to convert binary to text");
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        log_debug!("[WsClient] Ping received, sending pong");
                        let _ = write.send(Message::Pong(data)).await;
                    }
                    Ok(Message::Pong(_)) => {
                        log_debug!("[WsClient] Pong received, connection healthy");
                    }
                    Ok(Message::Close(_)) => {
                        log_important!(info, "[WsClient] Server closed connection gracefully");
                        break;
                    }
                    Ok(Message::Frame(_)) => {
                        log_important!(info, "[WsClient] Raw frame received");
                    }
                    Err(e) => {
                        log_important!(error, "[WsClient] Read error: {}", e);
                        break;
                    }
                }
            }
            
            // 心跳 - 发送 ping
            _ = heartbeat.tick() => {
                let ping = serde_json::json!({"type": "ping"});
                log_debug!("[WsClient] Sending heartbeat ping");
                if write.send(Message::Text(ping.to_string())).await.is_err() {
                    log_important!(error, "[WsClient] Failed to send heartbeat, connection broken");
                    break;
                }
            }
            
            // 健康检查 - 检测连接是否超时
            _ = health_check.tick() => {
                let (elapsed, pending_count) = {
                    let state = WS_CLIENT.read().await;
                    (state.last_message_time.elapsed(), state.pending.len())
                };
                
                // 定期输出连接状态（便于调试）
                if pending_count > 0 {
                    log_debug!("[WsClient] Health check: last_msg={:?} ago, pending_requests={}", elapsed, pending_count);
                }
                
                if elapsed > Duration::from_secs(CONNECTION_TIMEOUT_SECS) {
                    log_important!(error, "[WsClient] Connection timeout! No message received for {:?}, pending_requests={}, breaking connection", elapsed, pending_count);
                    break;
                }
            }
        }
    }
    
    log_important!(warn, "[WsClient] Connection handler exiting");
}

/// 处理接收到的消息
async fn handle_message(text: &str) {
    log_important!(info, "[WsClient] handle_message called, text length={}", text.len());
    log_important!(info, "[WsClient] >>> BEFORE JSON PARSE <<<");
    
    let parse_result = serde_json::from_str::<WsMessage>(text);
    log_important!(info, "[WsClient] >>> AFTER JSON PARSE, success={} <<<", parse_result.is_ok());
    
    match parse_result {
        Ok(WsMessage::Response { id, payload }) => {
            log_important!(info, "[WsClient] Parsed response for request: {}", id);
            
            log_important!(info, "[WsClient] Acquiring write lock...");
            let mut state = WS_CLIENT.write().await;
            log_important!(info, "[WsClient] Write lock acquired");
            
            log_important!(info, "[WsClient] Pending requests count: {}", state.pending.len());
            for key in state.pending.keys() {
                log_important!(info, "[WsClient] Pending request_id: {}", key);
            }
            
            log_important!(info, "[WsClient] Attempting to remove pending request with id: {}", id);
            let removed = state.pending.remove(&id);
            log_important!(info, "[WsClient] Remove result: {}", removed.is_some());
            
            if let Some(pending) = removed {
                log_important!(info, "[WsClient] Found matching pending request, sending to channel");
                drop(state); // 释放锁后再发送
                log_important!(info, "[WsClient] Lock released, sending payload...");
                if pending.sender.send(payload).is_err() {
                    log_important!(error, "[WsClient] Failed to send response through channel (receiver dropped)");
                } else {
                    log_important!(info, "[WsClient] Response sent to waiting task successfully");
                }
            } else {
                log_important!(error, "[WsClient] No pending request found for id: {}", id);
                drop(state);
            }
            log_important!(info, "[WsClient] Response handling completed");
        }
        Ok(WsMessage::Ping) => {
            // 正确处理服务端发来的 Ping，回复 Pong
            log_debug!("[WsClient] Received ping from server, connection healthy");
            // 注意：Pong 响应在 handle_connection 的 WebSocket 层已处理
        }
        Ok(WsMessage::Pong) => {
            log_debug!("[WsClient] Received pong");
        }
        Ok(WsMessage::Connected { message }) => {
            log_important!(info, "[WsClient] Server says: {}", message);
        }
        Ok(WsMessage::Error { id, message }) => {
            log_important!(error, "[WsClient] Server error: {}", message);
            if let Some(id) = id {
                let mut state = WS_CLIENT.write().await;
                if let Some(pending) = state.pending.remove(&id) {
                    let _ = pending.sender.send(DaemonResponse::error(message));
                }
            }
        }
        Ok(other) => {
            log_important!(warn, "[WsClient] Received unexpected message type: {:?}", other);
        }
        Err(e) => {
            log_important!(error, "[WsClient] Failed to parse message: {}", e);
            log_important!(error, "[WsClient] Raw message: {}", &text[..text.len().min(200)]);
        }
    }
}

/// 请求超时时间（秒）- 弹窗响应可能需要较长时间
const REQUEST_TIMEOUT_SECS: u64 = 600; // 10 分钟

/// 通过 WebSocket 执行请求
pub async fn execute_via_ws(request: DaemonRequest) -> Result<DaemonResponse> {
    log_important!(info, "[WsClient] execute_via_ws called");
    
    // 确保连接已初始化
    init_ws_connection().await?;
    
    let request_id = uuid::Uuid::new_v4().to_string();
    log_important!(info, "[WsClient] Request ID: {}", request_id);
    
    let (tx, rx) = oneshot::channel();
    
    // 注册待处理请求并检查连接健康状态
    let sender = {
        let mut state = WS_CLIENT.write().await;
        log_important!(info, "[WsClient] State: connected={}, has_sender={}, pending_count={}", 
            state.connected, state.sender.is_some(), state.pending.len());
        
        if !state.connected || state.sender.is_none() {
            log_important!(error, "[WsClient] Not connected, cannot send");
            return Err(anyhow::anyhow!("WebSocket not connected"));
        }
        
        // 检查连接是否健康（最近有消息）
        let elapsed = state.last_message_time.elapsed();
        if elapsed > Duration::from_secs(CONNECTION_TIMEOUT_SECS) {
            log_important!(error, "[WsClient] Connection appears stale (no message for {:?}), marking as disconnected", elapsed);
            state.connected = false;
            return Err(anyhow::anyhow!("WebSocket connection stale"));
        }
        
        state.pending.insert(request_id.clone(), PendingRequest { sender: tx });
        state.sender.clone().unwrap()
    };
    
    // 构造消息
    let msg = WsMessage::Request {
        id: request_id.clone(),
        payload: request,
    };
    let msg_text = serde_json::to_string(&msg)?;
    log_important!(info, "[WsClient] Sending message, length={}", msg_text.len());
    
    // 发送请求
    if let Err(e) = sender.send(msg_text).await {
        log_important!(error, "[WsClient] Failed to send message: {}", e);
        // 清理 pending 请求
        let mut state = WS_CLIENT.write().await;
        state.pending.remove(&request_id);
        return Err(anyhow::anyhow!("Failed to send: {}", e));
    }
    
    log_important!(info, "[WsClient] Message sent, waiting for response (timeout={}s)...", REQUEST_TIMEOUT_SECS);
    
    // 等待响应
    match tokio::time::timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS), rx).await {
        Ok(Ok(response)) => {
            log_important!(info, "[WsClient] Response received successfully");
            // 检查是否是错误响应（连接断开导致的）
            if response.success == false && response.error.as_ref().map(|e| e.contains("disconnected")).unwrap_or(false) {
                log_important!(warn, "[WsClient] Received disconnect error response");
                return Err(anyhow::anyhow!("WebSocket disconnected during request"));
            }
            Ok(response)
        }
        Ok(Err(_)) => {
            log_important!(error, "[WsClient] Response channel closed unexpectedly (connection likely dropped)");
            let mut state = WS_CLIENT.write().await;
            state.pending.remove(&request_id);
            Err(anyhow::anyhow!("Response channel closed - connection dropped"))
        }
        Err(_) => {
            log_important!(error, "[WsClient] Request timeout after {}s", REQUEST_TIMEOUT_SECS);
            let mut state = WS_CLIENT.write().await;
            state.pending.remove(&request_id);
            Err(anyhow::anyhow!("Request timeout"))
        }
    }
}

/// 检查 WebSocket 是否已连接
pub async fn is_ws_connected() -> bool {
    let state = WS_CLIENT.read().await;
    let connected = state.connected;
    log_debug!("[WsClient] is_ws_connected check: {}", connected);
    connected
}
