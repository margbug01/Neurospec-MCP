use crate::config::AppState;
use crate::constants::app::{EXIT_CONFIRMATION_WINDOW_SECS, REQUIRED_EXIT_ATTEMPTS};
use crate::log_important;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State, Emitter};

/// 检查是否应该允许退出
/// 返回 (should_exit, show_warning)
pub fn should_allow_exit(state: &State<AppState>) -> Result<(bool, bool), String> {
    let now = Instant::now();

    // 获取当前退出尝试计数和上次尝试时间
    let (current_count, last_attempt) = {
        let count_guard = state.exit_attempt_count.lock()
            .map_err(|e| format!("获取退出计数失败: {}", e))?;
        let time_guard = state.last_exit_attempt.lock()
            .map_err(|e| format!("获取退出时间失败: {}", e))?;

        (*count_guard, *time_guard)
    };

    log_important!(info, "🔍 退出检查 - 当前计数: {}, 要求计数: {}", current_count, REQUIRED_EXIT_ATTEMPTS);

    // 检查时间窗口
    let within_time_window = if let Some(last_time) = last_attempt {
        let elapsed = now.duration_since(last_time);
        let within_window = elapsed <= Duration::from_secs(EXIT_CONFIRMATION_WINDOW_SECS);
        log_important!(info, "🔍 时间窗口检查 - 距离上次: {:?}, 窗口期: {}秒, 在窗口内: {}",
                elapsed, EXIT_CONFIRMATION_WINDOW_SECS, within_window);
        within_window
    } else {
        log_important!(info, "🔍 首次退出尝试");
        false
    };
    
    // 如果超出时间窗口，重置计数器并开始新的计数
    if !within_time_window {
        reset_exit_attempts(state)?;
        increment_exit_attempts(state, now)?;
        return Ok((false, true)); // 不退出，显示警告
    }

    // 在时间窗口内，先增加计数，然后检查是否达到要求
    increment_exit_attempts(state, now)?;
    let new_count = {
        let count_guard = state.exit_attempt_count.lock()
            .map_err(|e| format!("获取退出计数失败: {}", e))?;
        *count_guard
    };

    if new_count >= REQUIRED_EXIT_ATTEMPTS {
        // 达到要求的尝试次数，允许退出
        reset_exit_attempts(state)?;
        Ok((true, false))
    } else {
        // 还未达到要求次数，显示警告
        Ok((false, true))
    }
}

/// 重置退出尝试计数器
fn reset_exit_attempts(state: &State<AppState>) -> Result<(), String> {
    {
        let mut count_guard = state.exit_attempt_count.lock()
            .map_err(|e| format!("重置退出计数失败: {}", e))?;
        *count_guard = 0;
    }
    
    {
        let mut time_guard = state.last_exit_attempt.lock()
            .map_err(|e| format!("重置退出时间失败: {}", e))?;
        *time_guard = None;
    }
    
    Ok(())
}

/// 增加退出尝试计数
fn increment_exit_attempts(state: &State<AppState>, now: Instant) -> Result<(), String> {
    {
        let mut count_guard = state.exit_attempt_count.lock()
            .map_err(|e| format!("增加退出计数失败: {}", e))?;
        *count_guard += 1;
    }
    
    {
        let mut time_guard = state.last_exit_attempt.lock()
            .map_err(|e| format!("更新退出时间失败: {}", e))?;
        *time_guard = Some(now);
    }
    
    Ok(())
}

/// 处理系统退出请求（来自快捷键或窗口关闭按钮）
pub async fn handle_system_exit_request(
    state: State<'_, AppState>,
    app: &AppHandle,
    is_manual_close: bool,
) -> Result<bool, String> {
    // 如果是手动点击关闭按钮，直接退出
    if is_manual_close {
        perform_exit(app.clone()).await?;
        return Ok(true);
    }
    
    // 检查是否应该允许退出
    let (should_exit, show_warning) = should_allow_exit(&state)?;
    
    if should_exit {
        perform_exit(app.clone()).await?;
        Ok(true)
    } else if show_warning {
        // 发送警告消息到前端
        let warning_message = format!(
            "再次按下退出快捷键以确认退出 ({}秒内有效)",
            EXIT_CONFIRMATION_WINDOW_SECS
        );

        if let Some(window) = app.get_webview_window("main") {
            match window.emit("exit-warning", &warning_message) {
                Ok(_) => {
                    log_important!(info, "✅ 退出警告事件已发送: {}", warning_message);
                }
                Err(e) => {
                    log_important!(error, "❌ 发送退出警告事件失败: {}", e);
                }
            }
        } else {
            log_important!(error, "❌ 无法获取主窗口，无法发送退出警告");
        }
        Ok(false)
    } else {
        Ok(false)
    }
}

/// 执行实际的退出操作
async fn perform_exit(app: AppHandle) -> Result<(), String> {
    // 直接退出应用，不关闭窗口（避免触发CloseRequested事件循环）
    app.exit(0);
    Ok(())
}

/// Tauri命令：强制退出应用（用于程序内部调用）
#[tauri::command]
pub async fn force_exit_app(app: AppHandle) -> Result<(), String> {
    crate::log_important!(warn, "⚠️ force_exit_app called! Application will exit.");
    perform_exit(app).await
}

/// Tauri命令：重置退出尝试计数器
#[tauri::command]
pub async fn reset_exit_attempts_cmd(state: State<'_, AppState>) -> Result<(), String> {
    reset_exit_attempts(&state)
}
