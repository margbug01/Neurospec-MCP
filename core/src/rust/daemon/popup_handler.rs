use anyhow::Result;
use tauri::{AppHandle, Manager, Emitter};
use tokio::sync::{oneshot, broadcast, Mutex};
use std::sync::Arc;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::time::Instant;

use crate::mcp::types::PopupRequest;
use crate::{log_important, log_debug};
use super::context_orchestrator::enhance_message_with_context;

// Response size limit (10MB) matching image limit
const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

// Maximum number of pending requests to prevent memory leaks/DoS
const MAX_PENDING_REQUESTS: usize = 100;

// Broadcast channel capacity for shared responses
const BROADCAST_CAPACITY: usize = 16;

// 已完成响应的缓存保留时间（秒）
const RESPONSE_CACHE_TTL_SECS: u64 = 30;

/// 获取弹窗超时时间（秒）
/// 优先从配置文件读取，失败时使用默认值
fn get_popup_timeout_secs() -> u64 {
    match crate::config::load_standalone_config() {
        Ok(config) => {
            let timeout = config.daemon_config.popup_timeout_secs;
            // 确保在合理范围内
            timeout.clamp(
                crate::constants::mcp::MIN_POPUP_TIMEOUT_SECS,
                crate::constants::mcp::MAX_POPUP_TIMEOUT_SECS,
            )
        }
        Err(_) => crate::constants::mcp::DEFAULT_POPUP_TIMEOUT_SECS,
    }
}

// Global storage for pending popup responses (使用 tokio::sync::Mutex 避免异步上下文问题)
lazy_static::lazy_static! {
    static ref PENDING_RESPONSES: Arc<Mutex<HashMap<String, oneshot::Sender<String>>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    
    // 进行中的请求缓存：基于消息内容 hash，允许多个请求者共享同一个弹窗响应
    // key: 消息内容 hash, value: (request_id, broadcast::Sender)
    static ref ONGOING_REQUESTS: Arc<Mutex<HashMap<u64, (String, broadcast::Sender<String>)>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    
    // 已完成响应的缓存：防止降级请求导致重复弹窗
    // key: 消息内容 hash, value: (响应内容, 过期时间)
    static ref COMPLETED_RESPONSES: Arc<Mutex<HashMap<u64, (String, Instant)>>> = 
        Arc::new(Mutex::new(HashMap::new()));
}

/// 计算消息内容的 hash 值
fn compute_message_hash(message: &str, options: &Option<Vec<String>>) -> u64 {
    let mut hasher = DefaultHasher::new();
    message.hash(&mut hasher);
    if let Some(opts) = options {
        for opt in opts {
            opt.hash(&mut hasher);
        }
    }
    hasher.finish()
}

// Show popup via Tauri window and wait for response
pub async fn show_popup_and_wait(app_handle: &AppHandle, request: &PopupRequest) -> Result<String> {
    // 计算消息 hash，用于去重
    let message_hash = compute_message_hash(&request.message, &request.predefined_options);
    
    // 首先检查是否有已完成的缓存响应（防止降级请求导致重复弹窗）
    {
        let mut completed = COMPLETED_RESPONSES.lock().await;
        
        // 惰性清理过期缓存
        let now = Instant::now();
        completed.retain(|_, (_, expires_at)| *expires_at > now);
        
        if let Some((cached_response, expires_at)) = completed.get(&message_hash) {
            if *expires_at > now {
                log_important!(info, "[Popup] Found cached response for message hash: {}, returning cached result", message_hash);
                return Ok(cached_response.clone());
            }
        }
    }
    
    // 检查是否有相同消息的请求正在进行中
    {
        let ongoing = ONGOING_REQUESTS.lock().await;
        if let Some((existing_id, sender)) = ongoing.get(&message_hash) {
            log_important!(info, "[Popup] Found ongoing request with same message hash: {}, subscribing...", existing_id);
            let mut receiver = sender.subscribe();
            drop(ongoing); // 释放锁
            
            // 等待已有请求的响应
            match receiver.recv().await {
                Ok(response) => {
                    log_important!(info, "[Popup] Received shared response from ongoing request");
                    return Ok(response);
                }
                Err(e) => {
                    log_important!(warn, "[Popup] Failed to receive shared response: {}, will create new popup", e);
                    // 继续创建新弹窗
                }
            }
        }
    }
    
    // 上下文增强：自动注入项目信息和相关记忆
    let enhanced_message = enhance_message_with_context(&request.message);
    let enhanced_request = PopupRequest {
        id: request.id.clone(),
        message: enhanced_message,
        predefined_options: request.predefined_options.clone(),
        is_markdown: request.is_markdown,
    };
    log_debug!("Popup request with context enhancement");
    
    let request_id = enhanced_request.id.clone();
    
    // 创建 broadcast channel 用于共享响应
    let (broadcast_tx, _) = broadcast::channel::<String>(BROADCAST_CAPACITY);
    let broadcast_tx_clone = broadcast_tx.clone();
    
    // Create oneshot channel for response
    let (tx, rx) = oneshot::channel();
    
    // Store the sender with capacity check and deduplication
    {
        let mut pending = PENDING_RESPONSES.lock().await;
            
        if pending.len() >= MAX_PENDING_REQUESTS {
            return Err(anyhow::anyhow!("Too many pending requests (max {})", MAX_PENDING_REQUESTS));
        }
        
        if pending.contains_key(&request_id) {
            return Err(anyhow::anyhow!("Duplicate request ID: {}", request_id));
        }
        
        pending.insert(request_id.clone(), tx);
        log_important!(info, "[Popup] Registered pending request: {}, total pending: {}", request_id, pending.len());
    }
    
    // 注册到进行中请求缓存
    {
        let mut ongoing = ONGOING_REQUESTS.lock().await;
        ongoing.insert(message_hash, (request_id.clone(), broadcast_tx_clone));
        log_important!(info, "[Popup] Registered ongoing request with hash: {}", message_hash);
    }
    
    // Get or create the main window
    let window = match app_handle.get_webview_window("main") {
        Some(w) => w,
        None => {
            // Cleanup if window not found
            let mut pending = PENDING_RESPONSES.lock().await;
            pending.remove(&request_id);
            
            log_important!(warn, "Main window not found, creating new window");
            return Err(anyhow::anyhow!("Main window not available"));
        }
    };
    
    // Show the window if hidden - Fail fast if error
    if let Err(e) = window.show() {
        log_important!(error, "Failed to show window: {}", e);
        // Cleanup and fail
        let mut pending = PENDING_RESPONSES.lock().await;
        pending.remove(&request_id);
        return Err(anyhow::anyhow!("Failed to show popup window: {}", e));
    }
    
    // Focus the window - Log error but continue (not fatal)
    if let Err(e) = window.set_focus() {
        log_important!(warn, "Failed to focus window: {}", e);
    }
    
    // Emit event to frontend with popup request (using enhanced request)
    if let Err(e) = window.emit("mcp-popup-request", &enhanced_request) {
        log_important!(error, "Failed to emit popup request: {}", e);
        // Clean up pending response
        let mut pending = PENDING_RESPONSES.lock().await;
        pending.remove(&request_id);
        return Err(anyhow::anyhow!("Failed to send popup request to frontend: {}", e));
    }
    
    log_debug!("Popup request sent to frontend, waiting for response...");
    
    // 从配置获取超时时间
    let timeout_secs = get_popup_timeout_secs();
    log_debug!("Using popup timeout: {} seconds", timeout_secs);
    
    // Wait for response with timeout
    let result = match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await {
        Ok(Ok(response)) => {
            log_important!(info, "Received popup response");
            // 广播响应给所有等待者
            let _ = broadcast_tx.send(response.clone());
            
            // 将响应存入已完成缓存，保留 30 秒
            {
                let mut completed = COMPLETED_RESPONSES.lock().await;
                let expires_at = Instant::now() + std::time::Duration::from_secs(RESPONSE_CACHE_TTL_SECS);
                completed.insert(message_hash, (response.clone(), expires_at));
                log_important!(info, "[Popup] Cached response for hash: {}, expires in {} secs", message_hash, RESPONSE_CACHE_TTL_SECS);
            }
            
            Ok(response)
        }
        Ok(Err(_)) => {
            // Channel closed without value (sender dropped)
            log_important!(warn, "Response channel closed");
            let mut pending = PENDING_RESPONSES.lock().await;
            pending.remove(&request_id);
            Err(anyhow::anyhow!("Response channel closed unexpectedly"))
        }
        Err(_) => {
            log_important!(warn, "Popup response timeout after {} seconds", timeout_secs);
            // Clean up pending response
            let mut pending = PENDING_RESPONSES.lock().await;
            pending.remove(&request_id);
            Err(anyhow::anyhow!("Popup response timeout ({} seconds)", timeout_secs))
        }
    };
    
    // 清理进行中请求缓存
    {
        let mut ongoing = ONGOING_REQUESTS.lock().await;
        ongoing.remove(&message_hash);
        log_important!(info, "[Popup] Removed ongoing request with hash: {}", message_hash);
    }
    
    result
}

/// Handle popup response from frontend (异步版本，配合 tokio::sync::Mutex)
pub async fn handle_popup_response(request_id: String, response: String) -> Result<()> {
    log_important!(info, "[Popup] Received response for request_id: {}", request_id);
    log_important!(info, "[Popup] Response length: {} bytes", response.len());
    
    // Validate response size to prevent DoS
    if response.len() > MAX_RESPONSE_SIZE {
        log_important!(error, "[Popup] Response size exceeds limit");
        return Err(anyhow::anyhow!(
            "Response size ({} bytes) exceeds maximum allowed size of {} bytes",
            response.len(),
            MAX_RESPONSE_SIZE
        ));
    }
    
    let mut pending = PENDING_RESPONSES.lock().await;
    
    // 调试：打印所有 pending 的 request_id
    log_important!(info, "[Popup] Pending requests count: {}", pending.len());
    for key in pending.keys() {
        log_important!(info, "[Popup] Pending request_id: {}", key);
    }
    
    if let Some(tx) = pending.remove(&request_id) {
        log_important!(info, "[Popup] Found pending request, sending response...");
        if tx.send(response).is_err() {
            log_important!(warn, "[Popup] Failed to send response through channel (receiver dropped)");
        } else {
            log_important!(info, "[Popup] Response sent successfully through channel");
        }
        Ok(())
    } else {
        log_important!(error, "[Popup] No pending request found for ID: {}", request_id);
        Err(anyhow::anyhow!("No pending request found for ID: {}", request_id))
    }
}