use crate::config::{AppState, load_config_and_apply_window_settings};
use crate::ui::setup_window_event_listeners;
use crate::ui::exit_handler::setup_exit_handlers;
use crate::ui::tray::create_tray;
use crate::daemon::start_daemon_server_with_app;
use crate::mcp::tools::interaction::init_interact_history;
use crate::log_important;
use crate::daemon::types::PendingResponseState;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "macos")]
use tauri::menu::{Menu, MenuItemBuilder, SubmenuBuilder, PredefinedMenuItem};

/// 应用设置和初始化
pub async fn setup_application(app_handle: &AppHandle) -> Result<(), String> {
    let state = app_handle.state::<AppState>();

    // Initialize shared pending response state
    app_handle.manage(PendingResponseState::default());

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

    // macOS: 创建原生菜单栏
    #[cfg(target_os = "macos")]
    {
        if let Err(e) = create_macos_menu(app_handle) {
            log_important!(warn, "创建 macOS 菜单栏失败: {}", e);
        }
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

/// macOS: 创建原生应用菜单栏
#[cfg(target_os = "macos")]
fn create_macos_menu(app_handle: &AppHandle) -> Result<(), String> {
    // NeuroSpec 应用菜单
    let app_submenu = SubmenuBuilder::new(app_handle, "NeuroSpec")
        .item(&PredefinedMenuItem::about(app_handle, Some("关于 NeuroSpec"), None).map_err(|e| e.to_string())?)
        .separator()
        .item(&PredefinedMenuItem::services(app_handle, Some("服务")).map_err(|e| e.to_string())?)
        .separator()
        .item(&PredefinedMenuItem::hide(app_handle, Some("隐藏 NeuroSpec")).map_err(|e| e.to_string())?)
        .item(&PredefinedMenuItem::hide_others(app_handle, Some("隐藏其他")).map_err(|e| e.to_string())?)
        .item(&PredefinedMenuItem::show_all(app_handle, Some("显示全部")).map_err(|e| e.to_string())?)
        .separator()
        .item(&PredefinedMenuItem::quit(app_handle, Some("退出 NeuroSpec")).map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())?;

    // 编辑菜单
    let edit_submenu = SubmenuBuilder::new(app_handle, "编辑")
        .item(&PredefinedMenuItem::undo(app_handle, Some("撤销")).map_err(|e| e.to_string())?)
        .item(&PredefinedMenuItem::redo(app_handle, Some("重做")).map_err(|e| e.to_string())?)
        .separator()
        .item(&PredefinedMenuItem::cut(app_handle, Some("剪切")).map_err(|e| e.to_string())?)
        .item(&PredefinedMenuItem::copy(app_handle, Some("复制")).map_err(|e| e.to_string())?)
        .item(&PredefinedMenuItem::paste(app_handle, Some("粘贴")).map_err(|e| e.to_string())?)
        .item(&PredefinedMenuItem::select_all(app_handle, Some("全选")).map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())?;

    // 窗口菜单
    let window_submenu = SubmenuBuilder::new(app_handle, "窗口")
        .item(&PredefinedMenuItem::minimize(app_handle, Some("最小化")).map_err(|e| e.to_string())?)
        .item(&PredefinedMenuItem::maximize(app_handle, Some("缩放")).map_err(|e| e.to_string())?)
        .separator()
        .item(&PredefinedMenuItem::close_window(app_handle, Some("关闭窗口")).map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())?;

    // 构建完整菜单栏
    let menu = Menu::with_items(app_handle, &[&app_submenu, &edit_submenu, &window_submenu])
        .map_err(|e| e.to_string())?;

    // 设置为应用菜单
    app_handle.set_menu(menu).map_err(|e| e.to_string())?;

    log_important!(info, "🍎 macOS 原生菜单栏已创建");
    Ok(())
}
