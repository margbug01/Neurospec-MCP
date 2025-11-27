use anyhow::Result;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tauri::AppHandle;

use super::routes::{create_router, create_router_with_app};
use crate::{log_important, log_debug};
use crate::mcp::tools::{init_global_store, init_global_watcher, init_global_search_config};

/// Default daemon server port
pub const DEFAULT_DAEMON_PORT: u16 = 15177;

/// Start the daemon HTTP server with Tauri app handle
/// Returns the actual bound address (useful if port 0 is used for auto-assignment)
pub async fn start_daemon_server_with_app(app_handle: AppHandle, port: Option<u16>) -> Result<SocketAddr> {
    let port = port.unwrap_or(DEFAULT_DAEMON_PORT);
    // Bind to 127.0.0.1 (localhost only) for security
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    // 初始化全局统一存储
    init_unified_store();
    
    log_important!(info, "Starting daemon HTTP server on {}", addr);
    
    // Create router with app handle for GUI integration
    let app = create_router_with_app(app_handle)
        .layer(CorsLayer::permissive());
    
    // Bind TCP listener
    let listener = TcpListener::bind(&addr).await?;
    let actual_addr = listener.local_addr()?;
    
    log_important!(info, "Daemon server listening on http://{}", actual_addr);
    
    // Spawn server in background task
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            log_important!(error, "Daemon server error: {}", e);
        }
    });
    
    Ok(actual_addr)
}

/// Start the daemon HTTP server without app handle (for testing)
/// Returns the actual bound address (useful if port 0 is used for auto-assignment)
pub async fn start_daemon_server(port: Option<u16>) -> Result<SocketAddr> {
    let port = port.unwrap_or(DEFAULT_DAEMON_PORT);
    // Bind to 127.0.0.1 (localhost only) for security
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    log_important!(info, "Starting daemon HTTP server on {}", addr);
    
    // Create router with CORS support
    let app = create_router()
        .layer(CorsLayer::permissive());
    
    // Bind TCP listener
    let listener = TcpListener::bind(&addr).await?;
    let actual_addr = listener.local_addr()?;
    
    log_important!(info, "Daemon server listening on http://{}", actual_addr);
    
    // Spawn server in background task
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            log_important!(error, "Daemon server error: {}", e);
        }
    });
    
    Ok(actual_addr)
}

/// Check if daemon server is running by attempting to connect
pub async fn is_daemon_running(port: Option<u16>) -> bool {
    let port = port.unwrap_or(DEFAULT_DAEMON_PORT);
    let addr = format!("http://127.0.0.1:{}/health", port);
    
    match reqwest::get(&addr).await {
        Ok(response) if response.status().is_success() => {
            log_debug!("Daemon health check passed");
            true
        }
        Ok(response) => {
            log_debug!("Daemon health check failed with status: {}", response.status());
            false
        }
        Err(e) => {
            log_debug!("Daemon not reachable: {}", e);
            false
        }
    }
}

/// Wait for daemon to be ready (with timeout)
pub async fn wait_for_daemon(port: Option<u16>, timeout_secs: u64) -> Result<()> {
    use tokio::time::{timeout, Duration};
    
    let check_interval = Duration::from_millis(100);
    let deadline = Duration::from_secs(timeout_secs);
    
    timeout(deadline, async {
        loop {
            if is_daemon_running(port).await {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(check_interval).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Daemon did not start within {} seconds", timeout_secs))?
}


/// 初始化全局统一存储、搜索引擎和文件监听器
fn init_unified_store() {
    // 获取缓存目录
    let base_cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("neurospec");
    
    let store_cache_dir = base_cache_dir.join("unified_store");
    let index_cache_dir = base_cache_dir.join("search_index");
    
    // 初始化全局存储
    if let Err(e) = init_global_store(&store_cache_dir) {
        log_important!(warn, "Failed to initialize global store: {}", e);
    } else {
        log_important!(info, "Global unified store initialized at {:?}", store_cache_dir);
    }
    
    // 初始化全局搜索配置
    if let Err(e) = init_global_search_config(&index_cache_dir) {
        log_important!(warn, "Failed to initialize global search config: {}", e);
    } else {
        log_important!(info, "Global search config initialized at {:?}", index_cache_dir);
    }
    
    // 初始化文件监听器
    if let Err(e) = init_global_watcher() {
        log_important!(warn, "Failed to initialize global watcher: {}", e);
    } else {
        log_important!(info, "Global file watcher initialized");
    }
}
