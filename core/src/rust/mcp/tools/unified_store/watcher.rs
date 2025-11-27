//! 文件监听器
//!
//! 使用 notify crate 监听文件变化，触发增量更新
//! 包含防抖处理避免频繁更新

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

/// 防抖时间（毫秒）
const DEBOUNCE_MS: u64 = 500;

/// 文件变化事件
#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
}

/// 文件监听器（带防抖）
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
    watched_paths: Arc<RwLock<Vec<PathBuf>>>,
    /// 防抖缓存：文件路径 -> 最后变化时间
    pending_changes: Arc<RwLock<HashMap<PathBuf, Instant>>>,
}

impl FileWatcher {
    /// 创建新的文件监听器
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel();
        
        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )?;

        Ok(Self {
            watcher,
            receiver: rx,
            watched_paths: Arc::new(RwLock::new(Vec::new())),
            pending_changes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 监听目录
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        self.watcher.watch(path, RecursiveMode::Recursive)?;
        
        let mut paths = self.watched_paths.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        paths.push(path.to_path_buf());
        
        Ok(())
    }

    /// 停止监听目录
    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        self.watcher.unwatch(path)?;
        
        let mut paths = self.watched_paths.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        paths.retain(|p| p != path);
        
        Ok(())
    }

    /// 获取待处理的变化事件（非阻塞，带防抖）
    /// 
    /// 只返回超过防抖时间的事件，避免频繁更新
    pub fn poll_events(&self) -> Vec<FileChangeEvent> {
        let now = Instant::now();
        let debounce_duration = Duration::from_millis(DEBOUNCE_MS);
        
        // 1. 收集新事件到 pending_changes
        while let Ok(result) = self.receiver.try_recv() {
            if let Ok(event) = result {
                for path in event.paths {
                    // 只处理代码文件
                    if !is_code_file(&path) {
                        continue;
                    }
                    
                    if let Ok(mut pending) = self.pending_changes.write() {
                        pending.insert(path, now);
                    }
                }
            }
        }
        
        // 2. 提取超过防抖时间的事件
        let mut events = Vec::new();
        let mut to_remove = Vec::new();
        
        if let Ok(pending) = self.pending_changes.read() {
            for (path, last_change) in pending.iter() {
                if now.duration_since(*last_change) >= debounce_duration {
                    // 根据文件是否存在判断事件类型
                    let event = if path.exists() {
                        FileChangeEvent::Modified(path.clone())
                    } else {
                        FileChangeEvent::Removed(path.clone())
                    };
                    events.push(event);
                    to_remove.push(path.clone());
                }
            }
        }
        
        // 3. 清理已处理的事件
        if !to_remove.is_empty() {
            if let Ok(mut pending) = self.pending_changes.write() {
                for path in to_remove {
                    pending.remove(&path);
                }
            }
        }
        
        events
    }

    /// 获取当前监听的路径
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths
            .read()
            .map(|paths| paths.clone())
            .unwrap_or_default()
    }
}

/// 检查是否为代码文件
fn is_code_file(path: &Path) -> bool {
    let code_extensions = ["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h", "hpp", "vue", "svelte"];
    
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| code_extensions.contains(&ext))
        .unwrap_or(false)
}
