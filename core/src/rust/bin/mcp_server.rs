// MCP Server Entry Point - Lightweight HTTP Client Mode
use neurospec::{mcp::run_server, utils::auto_init_logger, log_important};
use neurospec::daemon::{is_daemon_running, DEFAULT_DAEMON_PORT};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging system
    auto_init_logger()?;

    log_important!(info, "Starting NeuroSpec MCP Server (Client Mode)");
    
    // Check if daemon is running
    if !is_daemon_running(None).await {
        log_important!(warn, "NeuroSpec daemon is not running!");
        log_important!(warn, "Please start the NeuroSpec GUI application first.");
        log_important!(warn, "The daemon should be running on http://127.0.0.1:{}", DEFAULT_DAEMON_PORT);
        
        eprintln!("\n⚠️  NeuroSpec Daemon Not Running");
        eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        eprintln!("The NeuroSpec MCP server requires the main application to be running.");
        eprintln!("\nPlease:");
        eprintln!("  1. Start the NeuroSpec GUI application");
        eprintln!("  2. Ensure it's running in the background (system tray)");
        eprintln!("  3. Try your MCP request again");
        eprintln!("\nExpected daemon address: http://127.0.0.1:{}", DEFAULT_DAEMON_PORT);
        eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        
        // Continue anyway - the server will handle connection errors gracefully
    } else {
        log_important!(info, "Daemon health check passed");
    }
    
    run_server().await
}
