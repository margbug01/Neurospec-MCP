use crate::config::AppState;
use crate::app::{setup::setup_application, commands::*};
use crate::log_important;
use tauri::{Builder, Manager};

/// 构建Tauri应用
pub fn build_tauri_app() -> Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log_important!(info, "Another instance attempted to start, focusing existing window");
            // Optionally bring the existing window to front
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))

        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            // 基础应用命令
            get_app_info,
            get_always_on_top,
            set_always_on_top,
            sync_window_state,
            reload_config,

            // 主题和窗口命令
            get_theme,
            set_theme,
            get_window_config,
            set_window_config,
            get_reply_config,
            set_reply_config,
            get_window_settings,
            set_window_settings,
            get_window_settings_for_mode,
            get_window_constraints_cmd,
            get_current_window_size,
            apply_window_constraints,
            update_window_size,
            crate::ui::window::show_window,

            // 字体命令
            get_font_config,
            set_font_family,
            set_font_size,
            set_custom_font_family,
            get_font_family_options,
            get_font_size_options,
            reset_font_config,

            // MCP 命令
            get_mcp_tools_config,
            set_mcp_tool_enabled,
            get_mcp_tools_status,
            reset_mcp_tools_config,
            handle_mcp_popup_response,
            send_mcp_response,
            get_cli_args,
            read_mcp_request,
            select_image_files,
            build_mcp_send_response,
            build_mcp_continue_response,
            create_test_popup,
            
            // 搜索命令（本地引擎）
            crate::mcp::tools::acemcp::commands::clear_acemcp_cache,
            crate::mcp::tools::acemcp::commands::debug_acemcp_search,
            crate::mcp::tools::acemcp::commands::execute_acemcp_tool,

            // 上下文编排器命令
            crate::daemon::commands::set_context_orchestrator_config,

            // 记忆管理命令
            crate::mcp::tools::memory::commands::memory_list,
            crate::mcp::tools::memory::commands::memory_add,
            crate::mcp::tools::memory::commands::memory_update,
            crate::mcp::tools::memory::commands::memory_delete,
            crate::mcp::tools::memory::commands::detect_project_path,
            crate::mcp::tools::memory::commands::analyze_memory_suggestions,

            // 自定义prompt命令
            get_custom_prompt_config,
            add_custom_prompt,
            update_custom_prompt,
            delete_custom_prompt,
            set_custom_prompt_enabled,
            update_custom_prompt_order,
            update_conditional_prompt_state,

            // 快捷键命令
            get_shortcut_config,
            update_shortcut_binding,
            reset_shortcuts_to_default,

            // 配置管理命令
            get_config_file_path,

            // 系统命令
            open_external_url,
            exit_app,
            handle_app_exit_request,
            force_exit_app,
            reset_exit_attempts_cmd,

            // 更新命令
            check_for_updates,
            download_and_install_update,
            get_current_version,
            restart_app,

            // AGENTS.md 编辑器命令
            crate::ui::agents_commands::detect_project_agents,
            crate::ui::agents_commands::load_agents_config,
            crate::ui::agents_commands::save_agents_config,
            crate::ui::agents_commands::set_project_path,
            crate::ui::agents_commands::get_index_status,

            // Interact 历史记录命令
            get_interact_history_cmd,
            search_interact_history_cmd,
            clear_interact_history_cmd,

            // 嵌入配置命令
            get_embedding_config_cmd,
            save_embedding_config_cmd,
            test_embedding_connection_cmd
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // 应用初始化 - 如果失败则阻止应用启动
            tauri::async_runtime::block_on(async {
                setup_application(&app_handle).await
            })
            .map_err(|e| {
                eprintln!("❌ 应用初始化失败: {}", e);
                eprintln!("请检查日志文件以获取详细信息");
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error>
            })?;

            Ok(())
        })
}

/// 运行Tauri应用
pub fn run_tauri_app() {
    build_tauri_app()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
