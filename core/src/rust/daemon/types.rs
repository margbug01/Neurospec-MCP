use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};

/// Shared state for pending popup responses
#[derive(Clone)]
pub struct PendingResponseState {
    pub map: Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>,
}

impl Default for PendingResponseState {
    fn default() -> Self {
        Self {
            map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Generic daemon request wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", content = "params")]
pub enum DaemonRequest {
    #[serde(rename = "interact")]
    Interact(crate::mcp::InteractRequest),
    
    #[serde(rename = "memory")]
    Memory(crate::mcp::MemoryRequest),
    
    #[serde(rename = "enhance_context")]
    EnhanceContext(EnhanceContextRequest),
}

/// Request to enhance a message with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhanceContextRequest {
    pub message: String,
}

/// Daemon response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl DaemonResponse {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}
