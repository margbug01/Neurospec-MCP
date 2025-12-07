// HTTP Daemon Server for MCP tools
// This module provides a lightweight HTTP API for MCP tool execution
// Running on localhost:15177 for fast IPC communication

pub mod server;
pub mod routes;
pub mod types;
pub mod client;
pub mod popup_handler;
pub mod context_orchestrator;
pub mod commands;
pub mod ws_handler;

pub use server::{start_daemon_server, start_daemon_server_with_app, is_daemon_running, DEFAULT_DAEMON_PORT};
pub use types::{DaemonRequest, DaemonResponse};
pub use client::DaemonClient;
pub use popup_handler::{show_popup_and_wait, handle_popup_response};
pub use context_orchestrator::{enhance_message_with_context, set_orchestrator_config, OrchestratorConfig};
pub use ws_handler::ws_upgrade_handler;
