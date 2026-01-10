use anyhow::Result;
use tauri::{AppHandle, Manager, Emitter};
use tokio::sync::oneshot;

use crate::mcp::types::PopupRequest;
use crate::daemon::types::PendingResponseState;
use crate::{log_important, log_debug};
use super::context_orchestrator::enhance_message_with_context;

// Response size limit (10MB) matching image limit
const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

// Maximum number of pending requests to prevent memory leaks/DoS
const MAX_PENDING_REQUESTS: usize = 100;

// Show popup via Tauri window and wait for response
pub async fn show_popup_and_wait(
    app_handle: &AppHandle, 
    pending_state: &PendingResponseState,
    request: &PopupRequest, 
    timeout_secs: u64
) -> Result<String> {
    // 上下文增强：自动注入项目信息和相关记忆
    let enhanced_message = enhance_message_with_context(&request.message);
    let enhanced_request = PopupRequest {
        id: request.id.clone(),
        message: enhanced_message,
        predefined_options: request.predefined_options.clone(),
        is_markdown: request.is_markdown,
    };
    
    let request_id = enhanced_request.id.clone();
    
    // Create oneshot channel for response
    let (tx, rx) = oneshot::channel();
    
    // Store the sender with capacity check
    {
        let mut pending = pending_state.map.lock().await;
            
        if pending.len() >= MAX_PENDING_REQUESTS {
            return Err(anyhow::anyhow!("Too many pending requests (max {})", MAX_PENDING_REQUESTS));
        }
        
        if pending.contains_key(&request_id) {
            return Err(anyhow::anyhow!("Duplicate request ID: {}", request_id));
        }
        
        pending.insert(request_id.clone(), tx);
        log_important!(info, "[Popup] Registered pending request: {}", request_id);
    }
    
    // Get or create the main window
    let window = match app_handle.get_webview_window("main") {
        Some(w) => w,
        None => {
            // Cleanup if window not found
            let mut pending = pending_state.map.lock().await;
            pending.remove(&request_id);
            
            log_important!(warn, "Main window not found");
            return Err(anyhow::anyhow!("Main window not available"));
        }
    };
    
    // Show the window if hidden - Fail fast if error
    if let Err(e) = window.show() {
        // Cleanup and fail
        let mut pending = pending_state.map.lock().await;
        pending.remove(&request_id);
        return Err(anyhow::anyhow!("Failed to show popup window: {}", e));
    }
    
    // Focus the window - Log error but continue (not fatal)
    if let Err(e) = window.set_focus() {
        log_important!(warn, "Failed to focus window: {}", e);
    }
    
    // Emit event to frontend with popup request
    if let Err(e) = window.emit("mcp-popup-request", &enhanced_request) {
        // Clean up pending response
        let mut pending = pending_state.map.lock().await;
        pending.remove(&request_id);
        return Err(anyhow::anyhow!("Failed to send popup request to frontend: {}", e));
    }
    
    log_debug!("Popup request sent to frontend, waiting for response (timeout: {}s)...", timeout_secs);
    
    // Wait for response with timeout
    let result = match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await {
        Ok(Ok(response)) => {
            log_important!(info, "Received popup response");
            Ok(response)
        }
        Ok(Err(_)) => {
            // Channel closed without value (sender dropped)
            let mut pending = pending_state.map.lock().await;
            pending.remove(&request_id);
            Err(anyhow::anyhow!("Response channel closed unexpectedly"))
        }
        Err(_) => {
            log_important!(warn, "Popup response timeout after {} seconds", timeout_secs);
            
            // Clean up pending response
            let mut pending = pending_state.map.lock().await;
            pending.remove(&request_id);
            
            // Fix Zombie Popup: Emit cancel event to frontend
            if let Some(w) = app_handle.get_webview_window("main") {
                if let Err(e) = w.emit("mcp-popup-cancel", &request_id) {
                    log_important!(warn, "Failed to emit cancel event: {}", e);
                } else {
                    log_important!(info, "Emitted mcp-popup-cancel event for {}", request_id);
                }
            }
            
            Err(anyhow::anyhow!("Popup response timeout ({} seconds)", timeout_secs))
        }
    };
    
    result
}

/// Handle popup response from frontend
pub async fn handle_popup_response(
    pending_state: &PendingResponseState,
    request_id: String, 
    response: String
) -> Result<()> {
    log_important!(info, "[Popup] Received response for request_id: {}", request_id);
    
    // Validate response size to prevent DoS
    if response.len() > MAX_RESPONSE_SIZE {
        return Err(anyhow::anyhow!(
            "Response size ({} bytes) exceeds maximum allowed size of {} bytes",
            response.len(),
            MAX_RESPONSE_SIZE
        ));
    }
    
    let mut pending = pending_state.map.lock().await;
    
    if let Some(tx) = pending.remove(&request_id) {
        if tx.send(response).is_err() {
            log_important!(warn, "[Popup] Failed to send response through channel (receiver dropped)");
        } else {
            log_important!(info, "[Popup] Response sent successfully");
        }
        Ok(())
    } else {
        log_important!(warn, "[Popup] No pending request found for ID: {} (possibly timed out)", request_id);
        Err(anyhow::anyhow!("No pending request found for ID: {}", request_id))
    }
}