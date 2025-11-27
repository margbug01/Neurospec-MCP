use crate::config::{AppState, load_config_and_apply_window_settings};
use crate::ui::setup_window_event_listeners;
use crate::ui::exit_handler::setup_exit_handlers;
use crate::ui::tray::create_tray;
use crate::daemon::start_daemon_server_with_app;
use crate::mcp::tools::interaction::init_interact_history;
use crate::log_important;
use tauri::{AppHandle, Manager};

/// 应用设置和初始化
pub async fn setup_application(app_handle: &AppHandle) -> Result<(), String> {
    let state = app_handle.state::<AppState>();

    // 加载配置并应用窗口设置
    if let Err(e) = load_config_and_apply_window_settings(&state, app_handle).await {
        log_important!(warn, "加载配置失败: {}", e);
    }

    // 初始化交互历史记录系统
    if let Err(e) = init_interact_history() {
        log_important!(warn, "初始化交互历史失败: {}", e);
    }

    // 启动 daemon HTTP server with app handle
    let app_handle_clone = app_handle.clone();
    match start_daemon_server_with_app(app_handle_clone, None).await {
        Ok(addr) => {
            log_important!(info, "Daemon server started successfully on {}", addr);
        }
        Err(e) => {
            log_important!(error, "Failed to start daemon server: {}", e);
            return Err(format!("Failed to start daemon server: {}", e));
        }
    }

    // 设置窗口事件监听器
    setup_window_event_listeners(app_handle);

    // 设置系统托盘
    if let Err(e) = create_tray(app_handle) {
        log_important!(warn, "创建系统托盘失败: {}", e);
    }

    // 设置退出处理器
    if let Err(e) = setup_exit_handlers(app_handle) {
        log_important!(warn, "设置退出处理器失败: {}", e);
    }

    // 启动配置文件监听器
    let app_handle_clone = app_handle.clone();
    if let Err(e) = crate::config::start_config_watcher(app_handle_clone) {
        log_important!(warn, "启动配置监听器失败: {}", e);
    } else {
        log_important!(info, "Config watcher started successfully");
    }

    // Explicitly show main window to ensure it appears in taskbar
    if let Some(window) = app_handle.get_webview_window("main") {
        if let Err(e) = window.show() {
             log_important!(warn, "Failed to show main window: {}", e);
        }
        if let Err(e) = window.set_focus() {
             log_important!(warn, "Failed to focus main window: {}", e);
        }
    }

    Ok(())
}
