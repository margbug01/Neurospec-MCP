use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

use super::types::{DaemonRequest, DaemonResponse};
use super::server::DEFAULT_DAEMON_PORT;
use crate::{log_important, log_debug};

/// 获取 HTTP 客户端超时时间（秒）
fn get_http_client_timeout_secs() -> u64 {
    match crate::config::load_standalone_config() {
        Ok(config) => config.daemon_config.http_client_timeout_secs,
        Err(_) => crate::constants::mcp::DEFAULT_HTTP_CLIENT_TIMEOUT_SECS,
    }
}

/// HTTP client for communicating with the daemon server
pub struct DaemonClient {
    client: Client,
    base_url: String,
}

impl DaemonClient {
    /// Create a new daemon client with configurable timeout
    pub fn new(port: Option<u16>) -> Self {
        let port = port.unwrap_or(DEFAULT_DAEMON_PORT);
        let base_url = format!("http://127.0.0.1:{}", port);
        
        let timeout_secs = get_http_client_timeout_secs();
        log_debug!("Creating HTTP client with timeout: {} seconds", timeout_secs);
        
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to create HTTP client");
        
        Self { client, base_url }
    }
    
    /// Execute a tool via the daemon server
    pub async fn execute_tool(&self, request: DaemonRequest) -> Result<DaemonResponse> {
        let url = format!("{}/mcp/execute", self.base_url);
        
        log_debug!("Sending request to daemon: {:?}", request);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                log_important!(error, "Failed to connect to daemon: {}", e);
                anyhow::anyhow!(
                    "Failed to connect to NeuroSpec daemon at {}. \
                    Please ensure the NeuroSpec GUI application is running. \
                    Error: {}", 
                    self.base_url, e
                )
            })?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Daemon returned error {}: {}", status, error_text);
        }
        
        let daemon_response: DaemonResponse = response.json().await?;
        
        if !daemon_response.success {
            if let Some(error) = daemon_response.error {
                anyhow::bail!("Tool execution failed: {}", error);
            } else {
                anyhow::bail!("Tool execution failed with unknown error");
            }
        }
        
        Ok(daemon_response)
    }
    
    /// Check if daemon is healthy
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => Ok(true),
            Ok(response) => {
                log_debug!("Health check failed with status: {}", response.status());
                Ok(false)
            }
            Err(e) => {
                log_debug!("Health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new(None)
    }
}
