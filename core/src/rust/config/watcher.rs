use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use std::sync::mpsc::channel;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use crate::{log_important, log_debug};
use super::storage::{get_config_path, load_config};
use super::settings::AppState;

/// 启动配置文件监听器
pub fn start_config_watcher(app_handle: AppHandle) -> Result<()> {
    let config_path = get_config_path(&app_handle)?;
    
    log_important!(info, "Starting config file watcher for: {:?}", config_path);
    
    // 创建通道
    let (tx, rx) = channel();
    
    // 创建监听器
    let config = Config::default().with_poll_interval(Duration::from_secs(2));
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        config,
    )?;
    
    // 监听配置文件所在目录
    if let Some(parent) = config_path.parent() {
        watcher.watch(parent, RecursiveMode::NonRecursive)?;
        log_debug!("Watching directory: {:?}", parent);
    }
    
    // 在后台线程处理文件变化事件
    std::thread::spawn(move || {
        // 保持 watcher 存活
        let _watcher = watcher;
        
        while let Ok(event) = rx.recv() {
            // 只处理修改事件
            if let EventKind::Modify(_) = event.kind {
                // 检查是否是配置文件
                if event.paths.iter().any(|p| p.ends_with("config.json")) {
                    log_important!(info, "Config file changed, reloading...");
                    
                    // 短暂延迟，确保文件写入完成
                    std::thread::sleep(Duration::from_millis(100));
                    
                    // 重新加载配置
                    let app_clone = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_clone.state::<AppState>();
                        match load_config(&state, &app_clone).await {
                            Ok(_) => {
                                log_important!(info, "Config reloaded successfully");
                                
                                // 发送事件通知前端
                                if let Err(e) = app_clone.emit("config-reloaded", ()) {
                                    log_debug!("Failed to emit config-reloaded event: {}", e);
                                }
                            }
                            Err(e) => {
                                log_important!(warn, "Failed to reload config: {}", e);
                            }
                        }
                    });
                }
            }
        }
    });
    
    Ok(())
}
