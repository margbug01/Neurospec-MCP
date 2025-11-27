use serde::{Deserialize, Serialize};

/// Generic daemon request wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", content = "params")]
pub enum DaemonRequest {
    #[serde(rename = "interact")]
    Interact(crate::mcp::InteractRequest),
    
    #[serde(rename = "memory")]
    Memory(crate::mcp::MemoryRequest),
    
    #[serde(rename = "search")]
    Search(crate::mcp::tools::acemcp::types::SearchRequest),
    
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
