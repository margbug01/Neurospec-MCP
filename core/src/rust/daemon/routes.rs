use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use super::ws_handler::ws_upgrade_handler;
use std::sync::Arc;
use std::time::Instant;
use tauri::AppHandle;

use super::types::{DaemonRequest, DaemonResponse, HealthResponse};
use super::context_orchestrator::enhance_message_with_context;
use crate::mcp::tools::{MemoryTool, AcemcpTool};
use crate::log_debug;

// Validation constants for DoS protection
const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB
const MAX_OPTIONS: usize = 20;

/// Application state shared across handlers
#[derive(Clone)]
pub struct DaemonAppState {
    pub start_time: Instant,
    pub app_handle: Option<AppHandle>,
}

impl DaemonAppState {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            app_handle: None,
        }
    }
    
    pub fn with_app_handle(app_handle: AppHandle) -> Self {
        Self {
            start_time: Instant::now(),
            app_handle: Some(app_handle),
        }
    }
}

/// Create the main router with all routes
pub fn create_router() -> Router {
    let state = Arc::new(DaemonAppState::new());
    
    Router::new()
        .route("/health", get(health_check))
        .route("/mcp/execute", post(execute_tool))
        .route("/ws", get(ws_upgrade_handler))  // WebSocket endpoint
        .with_state(state)
}

/// Create router with Tauri app handle for GUI integration
pub fn create_router_with_app(app_handle: AppHandle) -> Router {
    let state = Arc::new(DaemonAppState::with_app_handle(app_handle));
    
    Router::new()
        .route("/health", get(health_check))
        .route("/mcp/execute", post(execute_tool))
        .route("/ws", get(ws_upgrade_handler))  // WebSocket endpoint
        .with_state(state)
}

/// Health check endpoint
async fn health_check(
    State(state): State<Arc<DaemonAppState>>,
) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
    })
}

/// Execute MCP tool endpoint
async fn execute_tool(
    State(state): State<Arc<DaemonAppState>>,
    Json(request): Json<DaemonRequest>,
) -> impl IntoResponse {
    log_debug!("Daemon: Received tool request: {:?}", request);
    
    let result = match request {
        DaemonRequest::Interact(interact_req) => {
            // Validate message size to prevent DoS
            if interact_req.message.len() > MAX_MESSAGE_SIZE {
                return (
                    StatusCode::OK,
                    Json(DaemonResponse::error(format!(
                        "Message size exceeds maximum allowed size of {} bytes",
                        MAX_MESSAGE_SIZE
                    )))
                );
            }
            
            // Validate options count to prevent DoS
            if interact_req.predefined_options.len() > MAX_OPTIONS {
                return (
                    StatusCode::OK,
                    Json(DaemonResponse::error(format!(
                        "Number of options ({}) exceeds maximum allowed ({})",
                        interact_req.predefined_options.len(),
                        MAX_OPTIONS
                    )))
                );
            }
            
            // Use app handle if available for GUI popup
            if let Some(app_handle) = &state.app_handle {
                use crate::mcp::types::PopupRequest;
                use crate::daemon::show_popup_and_wait;
                use crate::mcp::handlers::parse_mcp_response;
                
                let popup_request = PopupRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    message: interact_req.message,
                    predefined_options: if interact_req.predefined_options.is_empty() {
                        None
                    } else {
                        Some(interact_req.predefined_options)
                    },
                    is_markdown: interact_req.is_markdown,
                };
                
                match show_popup_and_wait(app_handle, &popup_request).await {
                    Ok(response_str) => {
                        match parse_mcp_response(&response_str) {
                            Ok(content) => {
                                let result = crate::mcp::create_success_result(content);
                                match serde_json::to_value(&result) {
                                    Ok(json) => DaemonResponse::success(json),
                                    Err(e) => DaemonResponse::error(format!("Failed to serialize result: {}", e)),
                                }
                            }
                            Err(e) => DaemonResponse::error(format!("Failed to parse response: {}", e)),
                        }
                    }
                    Err(e) => DaemonResponse::error(format!("Popup failed: {}", e)),
                }
            } else {
                // Fail explicitly if no app handle (Headless Daemon)
                // Do NOT call InteractionTool::interact here as it would cause infinite recursion
                DaemonResponse::error(
                    "Cannot show popup: Daemon running in headless mode or AppHandle missing. \
                    GUI interaction requires the main application window."
                )
            }
        }
        DaemonRequest::Memory(memory_req) => {
            match MemoryTool::manage_memory(memory_req).await {
                Ok(result) => {
                    match serde_json::to_value(&result) {
                        Ok(json) => DaemonResponse::success(json),
                        Err(e) => DaemonResponse::error(format!("Failed to serialize result: {}", e)),
                    }
                }
                Err(e) => DaemonResponse::error(format!("Tool execution failed: {}", e)),
            }
        }
        DaemonRequest::Search(search_req) => {
            match AcemcpTool::search_context(search_req).await {
                Ok(result) => {
                    match serde_json::to_value(&result) {
                        Ok(json) => DaemonResponse::success(json),
                        Err(e) => DaemonResponse::error(format!("Failed to serialize result: {}", e)),
                    }
                }
                Err(e) => DaemonResponse::error(format!("Tool execution failed: {}", e)),
            }
        }
        DaemonRequest::EnhanceContext(enhance_req) => {
            // 使用 context_orchestrator 增强消息
            let enhanced = enhance_message_with_context(&enhance_req.message);
            DaemonResponse::success(serde_json::json!({
                "original": enhance_req.message,
                "enhanced": enhanced,
            }))
        }
    };
    
    (StatusCode::OK, Json(result))
}

/// Process daemon request - shared logic for HTTP and WebSocket handlers
/// This is the core request processing function, extracted for reuse
pub async fn process_daemon_request(
    request: DaemonRequest,
    state: &Arc<DaemonAppState>,
) -> DaemonResponse {
    match request {
        DaemonRequest::Interact(interact_req) => {
            // Validate message size
            if interact_req.message.len() > MAX_MESSAGE_SIZE {
                return DaemonResponse::error(format!(
                    "Message size exceeds maximum allowed size of {} bytes",
                    MAX_MESSAGE_SIZE
                ));
            }
            
            // Validate options count
            if interact_req.predefined_options.len() > MAX_OPTIONS {
                return DaemonResponse::error(format!(
                    "Number of options ({}) exceeds maximum allowed ({})",
                    interact_req.predefined_options.len(),
                    MAX_OPTIONS
                ));
            }
            
            // Use app handle if available for GUI popup
            if let Some(app_handle) = &state.app_handle {
                use crate::mcp::types::PopupRequest;
                use crate::daemon::show_popup_and_wait;
                use crate::mcp::handlers::parse_mcp_response;
                
                let popup_request = PopupRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    message: interact_req.message,
                    predefined_options: if interact_req.predefined_options.is_empty() {
                        None
                    } else {
                        Some(interact_req.predefined_options)
                    },
                    is_markdown: interact_req.is_markdown,
                };
                
                match show_popup_and_wait(app_handle, &popup_request).await {
                    Ok(response_str) => {
                        match parse_mcp_response(&response_str) {
                            Ok(content) => {
                                let result = crate::mcp::create_success_result(content);
                                match serde_json::to_value(&result) {
                                    Ok(json) => DaemonResponse::success(json),
                                    Err(e) => DaemonResponse::error(format!("Failed to serialize result: {}", e)),
                                }
                            }
                            Err(e) => DaemonResponse::error(format!("Failed to parse response: {}", e)),
                        }
                    }
                    Err(e) => DaemonResponse::error(format!("Popup failed: {}", e)),
                }
            } else {
                DaemonResponse::error(
                    "Cannot show popup: Daemon running in headless mode or AppHandle missing."
                )
            }
        }
        DaemonRequest::Memory(memory_req) => {
            match MemoryTool::manage_memory(memory_req).await {
                Ok(result) => {
                    match serde_json::to_value(&result) {
                        Ok(json) => DaemonResponse::success(json),
                        Err(e) => DaemonResponse::error(format!("Failed to serialize result: {}", e)),
                    }
                }
                Err(e) => DaemonResponse::error(format!("Tool execution failed: {}", e)),
            }
        }
        DaemonRequest::Search(search_req) => {
            match AcemcpTool::search_context(search_req).await {
                Ok(result) => {
                    match serde_json::to_value(&result) {
                        Ok(json) => DaemonResponse::success(json),
                        Err(e) => DaemonResponse::error(format!("Failed to serialize result: {}", e)),
                    }
                }
                Err(e) => DaemonResponse::error(format!("Tool execution failed: {}", e)),
            }
        }
        DaemonRequest::EnhanceContext(enhance_req) => {
            let enhanced = enhance_message_with_context(&enhance_req.message);
            DaemonResponse::success(serde_json::json!({
                "original": enhance_req.message,
                "enhanced": enhanced,
            }))
        }
    }
}
