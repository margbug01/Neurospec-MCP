use anyhow::Result;
use tauri::{AppHandle, Manager, Emitter};
use tokio::sync::oneshot;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::mcp::types::PopupRequest;
use crate::{log_important, log_debug};
use super::context_orchestrator::enhance_message_with_context;

// Response size limit (10MB) matching image limit
const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

// Default popup timeout in seconds
pub const POPUP_TIMEOUT_SECS: u64 = 300; // Default popup timeout in seconds

// Maximum number of pending requests to prevent memory leaks/DoS
const MAX_PENDING_REQUESTS: usize = 100; // Maximum number of pending requests to prevent memory leaks/DoS

// Global storage for pending popup responses
lazy_static::lazy_static! {
    static ref PENDING_RESPONSES: Arc<Mutex<HashMap<String, oneshot::Sender<String>>>> = 
        Arc::new(Mutex::new(HashMap::new())); // Global storage for pending popup responses
}

// Show popup via Tauri window and wait for response
pub async fn show_popup_and_wait(app_handle: &AppHandle, request: &PopupRequest) -> Result<String> {
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
    
    // Create oneshot channel for response
    let (tx, rx) = oneshot::channel();
    
    // Store the sender with capacity check and deduplication
    {
        let mut pending = PENDING_RESPONSES.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock pending responses: {}", e))?;
            
        if pending.len() >= MAX_PENDING_REQUESTS {
            return Err(anyhow::anyhow!("Too many pending requests (max {})", MAX_PENDING_REQUESTS));
        }
        
        if pending.contains_key(&request_id) {
            return Err(anyhow::anyhow!("Duplicate request ID: {}", request_id));
        }
        
        pending.insert(request_id.clone(), tx);
    }
    
    // Get or create the main window
    let window = match app_handle.get_webview_window("main") {
        Some(w) => w,
        None => {
            // Cleanup if window not found
            let mut pending = PENDING_RESPONSES.lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock pending responses: {}", e))?;
            pending.remove(&request_id);
            
            log_important!(warn, "Main window not found, creating new window");
            return Err(anyhow::anyhow!("Main window not available"));
        }
    };
    
    // Show the window if hidden - Fail fast if error
    if let Err(e) = window.show() {
        log_important!(error, "Failed to show window: {}", e);
        // Cleanup and fail
        let mut pending = PENDING_RESPONSES.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock pending responses: {}", e))?;
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
        let mut pending = PENDING_RESPONSES.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock pending responses: {}", e))?;
        pending.remove(&request_id);
        return Err(anyhow::anyhow!("Failed to send popup request to frontend: {}", e));
    }
    
    log_debug!("Popup request sent to frontend, waiting for response...");
    
    // Wait for response with timeout
    match tokio::time::timeout(std::time::Duration::from_secs(POPUP_TIMEOUT_SECS), rx).await {
        Ok(Ok(response)) => {
            log_important!(info, "Received popup response");
            Ok(response)
        }
        Ok(Err(_)) => {
            // Channel closed without value (sender dropped)
            log_important!(warn, "Response channel closed");
            let mut pending = PENDING_RESPONSES.lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock pending responses: {}", e))?;
            pending.remove(&request_id);
            Err(anyhow::anyhow!("Response channel closed unexpectedly"))
        }
        Err(_) => {
            log_important!(warn, "Popup response timeout");
            // Clean up pending response
            let mut pending = PENDING_RESPONSES.lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock pending responses: {}", e))?;
            pending.remove(&request_id);
            Err(anyhow::anyhow!("Popup response timeout ({} seconds)", POPUP_TIMEOUT_SECS))
        }
    }
}

/// Handle popup response from frontend
pub fn handle_popup_response(request_id: String, response: String) -> Result<()> {
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
    
    let mut pending = PENDING_RESPONSES.lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock pending responses: {}", e))?;
    
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