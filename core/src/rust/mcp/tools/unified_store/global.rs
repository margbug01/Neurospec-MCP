//! 全局单例管理
//!
//! 提供 UnifiedSymbolStore 和 LocalSearcher 的全局访问点

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use lazy_static::lazy_static;

use super::store::UnifiedSymbolStore;
use super::watcher::{FileWatcher, FileChangeEvent};
use crate::mcp::tools::acemcp::local_engine::{LocalSearcher, LocalEngineConfig};

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

/// 索引过期时间（秒）- 默认 24 小时
const INDEX_EXPIRY_SECS: u64 = 86400;

/// 索引状态文件名
const INDEX_STATE_FILE: &str = "index_state.json";

/// 项目索引状态（可持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIndexState {
    /// 索引是否就绪
    pub ready: bool,
    /// 索引是否正在进行
    #[serde(default)]
    pub indexing: bool,
    /// 上次索引完成时间戳 (Unix timestamp)
    pub last_indexed_ts: Option<u64>,
    /// 索引文件数
    pub file_count: usize,
}

impl Default for ProjectIndexState {
    fn default() -> Self {
        Self {
            ready: false,
            indexing: false,
            last_indexed_ts: None,
            file_count: 0,
        }
    }
}

impl ProjectIndexState {
    /// 获取当前 Unix 时间戳
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
    
    /// 检查索引是否过期
    pub fn is_expired(&self) -> bool {
        match self.last_indexed_ts {
            Some(ts) => {
                let now = Self::current_timestamp();
                now.saturating_sub(ts) > INDEX_EXPIRY_SECS
            }
            None => true,
        }
    }
}

/// 持久化的索引状态存储
#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedIndexState {
    projects: HashMap<String, ProjectIndexState>,
}

lazy_static! {
    /// 全局统一符号存储
    static ref GLOBAL_STORE: Arc<RwLock<Option<UnifiedSymbolStore>>> = Arc::new(RwLock::new(None));
    
    /// 全局文件监听器（使用 Mutex 因为 Receiver 不是 Sync）
    static ref GLOBAL_WATCHER: Arc<std::sync::Mutex<Option<FileWatcher>>> = Arc::new(std::sync::Mutex::new(None));
    
    /// 全局搜索引擎配置
    static ref GLOBAL_SEARCH_CONFIG: Arc<RwLock<Option<LocalEngineConfig>>> = Arc::new(RwLock::new(None));
    
    /// 项目索引状态（项目路径 -> 状态）
    static ref PROJECT_INDEX_STATE: Arc<RwLock<HashMap<String, ProjectIndexState>>> = {
        // 尝试从文件加载持久化状态
        let state = load_persisted_state().unwrap_or_default();
        Arc::new(RwLock::new(state))
    };
}

/// 初始化全局存储
///
/// 应在应用启动时调用一次
pub fn init_global_store(cache_dir: &std::path::Path) -> Result<()> {
    let store = UnifiedSymbolStore::new(cache_dir)?;
    
    let mut global = GLOBAL_STORE.write().map_err(|e| anyhow::anyhow!("{}", e))?;
    *global = Some(store);
    
    Ok(())
}

/// 获取全局存储的引用
///
/// 如果未初始化，返回错误
pub fn get_global_store() -> Result<Arc<RwLock<Option<UnifiedSymbolStore>>>> {
    Ok(GLOBAL_STORE.clone())
}

/// 使用全局存储执行操作
pub fn with_global_store<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&UnifiedSymbolStore) -> Result<R>,
{
    let guard = GLOBAL_STORE.read().map_err(|e| anyhow::anyhow!("{}", e))?;
    let store = guard.as_ref().ok_or_else(|| anyhow::anyhow!("Global store not initialized"))?;
    f(store)
}


/// 初始化全局文件监听器
pub fn init_global_watcher() -> Result<()> {
    let watcher = FileWatcher::new()?;
    
    let mut global = GLOBAL_WATCHER.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
    *global = Some(watcher);
    
    Ok(())
}

/// 开始监听项目目录
pub fn watch_project(project_root: &std::path::Path) -> Result<()> {
    let mut guard = GLOBAL_WATCHER.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
    
    if let Some(ref mut watcher) = *guard {
        watcher.watch(project_root)?;
    } else {
        return Err(anyhow::anyhow!("Global watcher not initialized"));
    }
    
    Ok(())
}

/// 处理文件变化事件
///
/// 应定期调用以处理待处理的文件变化
pub fn process_file_changes() -> Result<usize> {
    let events = {
        let guard = GLOBAL_WATCHER.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        if let Some(ref watcher) = *guard {
            watcher.poll_events()
        } else {
            return Ok(0);
        }
    };

    if events.is_empty() {
        return Ok(0);
    }

    let mut store_guard = GLOBAL_STORE.write().map_err(|e| anyhow::anyhow!("{}", e))?;
    let store = store_guard.as_mut().ok_or_else(|| anyhow::anyhow!("Global store not initialized"))?;

    let mut processed = 0;
    for event in events {
        match event {
            FileChangeEvent::Created(path) | FileChangeEvent::Modified(path) => {
                // 找到项目根目录并使文件失效
                if let Some(project_root) = find_project_root(&path) {
                    let rel_path = path.strip_prefix(&project_root)
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();
                    let _ = store.invalidate_file(&project_root, &rel_path);
                    processed += 1;
                }
            }
            FileChangeEvent::Removed(path) => {
                if let Some(project_root) = find_project_root(&path) {
                    let rel_path = path.strip_prefix(&project_root)
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();
                    let _ = store.invalidate_file(&project_root, &rel_path);
                    processed += 1;
                }
            }
        }
    }

    Ok(processed)
}

/// 查找文件所属的项目根目录（通过 .git 目录）
fn find_project_root(path: &std::path::Path) -> Option<PathBuf> {
    let mut current = path.parent()?;
    
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        
        current = current.parent()?;
    }
}

// ============================================================================
// 全局搜索引擎相关
// ============================================================================

/// 初始化全局搜索配置
/// 
/// 应在应用启动时与 init_global_store 一起调用
pub fn init_global_search_config(index_dir: &std::path::Path) -> Result<()> {
    let config = LocalEngineConfig {
        index_path: index_dir.to_path_buf(),
        max_results: 10,
        snippet_context: 3,
    };
    
    let mut global = GLOBAL_SEARCH_CONFIG.write().map_err(|e| anyhow::anyhow!("{}", e))?;
    *global = Some(config);
    
    Ok(())
}

/// 获取全局搜索配置
pub fn get_global_search_config() -> Result<LocalEngineConfig> {
    let guard = GLOBAL_SEARCH_CONFIG.read().map_err(|e| anyhow::anyhow!("{}", e))?;
    guard.clone().ok_or_else(|| anyhow::anyhow!("Global search config not initialized"))
}

/// 为项目创建 Searcher
/// 
/// 使用全局配置创建针对特定项目的 Searcher 实例
pub fn create_searcher_for_project(project_root: &std::path::Path) -> Result<LocalSearcher> {
    let config = get_global_search_config()?;
    LocalSearcher::new(config, project_root.to_path_buf())
}

/// 检查全局搜索系统是否已初始化
pub fn is_search_initialized() -> bool {
    GLOBAL_SEARCH_CONFIG.read()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

// ============================================================================
// 索引状态管理
// ============================================================================

/// 检查项目索引是否就绪
/// 
/// 索引就绪条件：
/// 1. 已完成至少一次完整索引
/// 2. 索引未过期（24小时内）
pub fn is_project_indexed(project_root: &std::path::Path) -> bool {
    let key = project_root.to_string_lossy().to_string();
    
    let guard = match PROJECT_INDEX_STATE.read() {
        Ok(g) => g,
        Err(_) => return false,
    };
    
    if let Some(state) = guard.get(&key) {
        if !state.ready {
            return false;
        }
        // 使用新的过期检查方法
        return !state.is_expired();
    }
    
    false
}

/// 检查项目是否正在索引中
pub fn is_project_indexing(project_root: &std::path::Path) -> bool {
    let key = project_root.to_string_lossy().to_string();
    
    PROJECT_INDEX_STATE.read()
        .map(|guard| guard.get(&key).map(|s| s.indexing).unwrap_or(false))
        .unwrap_or(false)
}

/// 标记项目开始索引
pub fn mark_indexing_started(project_root: &std::path::Path) {
    let key = project_root.to_string_lossy().to_string();
    
    if let Ok(mut guard) = PROJECT_INDEX_STATE.write() {
        let state = guard.entry(key).or_default();
        state.indexing = true;
    }
}

/// 标记项目索引完成
/// 
/// 同时启动文件监听（如果全局 watcher 已初始化）
pub fn mark_indexing_complete(project_root: &std::path::Path, file_count: usize) {
    let key = project_root.to_string_lossy().to_string();
    
    if let Ok(mut guard) = PROJECT_INDEX_STATE.write() {
        let state = guard.entry(key).or_default();
        state.ready = true;
        state.indexing = false;
        state.last_indexed_ts = Some(ProjectIndexState::current_timestamp());
        state.file_count = file_count;
        
        // 持久化保存
        let _ = save_persisted_state(&guard);
    }
    
    // 自动启动文件监听
    if let Err(e) = start_watching_project(project_root) {
        crate::log_important!(warn, "Failed to start file watching: {}", e);
    }
}

/// 启动项目文件监听
fn start_watching_project(project_root: &std::path::Path) -> Result<()> {
    let mut guard = GLOBAL_WATCHER.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
    
    if let Some(ref mut watcher) = *guard {
        // 检查是否已在监听
        let watched = watcher.watched_paths();
        if !watched.iter().any(|p| p == project_root) {
            watcher.watch(project_root)?;
            crate::log_important!(info, "Started watching project: {}", project_root.display());
        }
    }
    
    Ok(())
}

/// 获取项目索引状态
pub fn get_index_state(project_root: &std::path::Path) -> Option<ProjectIndexState> {
    let key = project_root.to_string_lossy().to_string();
    
    PROJECT_INDEX_STATE.read()
        .ok()
        .and_then(|guard| guard.get(&key).cloned())
}

/// 获取项目已索引的文件数量
pub fn get_indexed_file_count(project_root: &std::path::Path) -> Option<usize> {
    get_index_state(project_root).map(|s| s.file_count)
}

// ============================================================================
// 持久化相关
// ============================================================================

/// 获取索引状态文件路径
fn get_state_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("neurospec").join(INDEX_STATE_FILE))
}

/// 从文件加载持久化的索引状态
fn load_persisted_state() -> Option<HashMap<String, ProjectIndexState>> {
    let path = get_state_file_path()?;
    
    if !path.exists() {
        return None;
    }
    
    let content = std::fs::read_to_string(&path).ok()?;
    let persisted: PersistedIndexState = serde_json::from_str(&content).ok()?;
    
    // 重置所有项目的 indexing 状态（重启后不可能还在索引）
    let mut projects = persisted.projects;
    for state in projects.values_mut() {
        state.indexing = false;
    }
    
    crate::log_important!(info, "Loaded {} persisted index states", projects.len());
    Some(projects)
}

/// 保存索引状态到文件
fn save_persisted_state(state: &HashMap<String, ProjectIndexState>) -> Result<()> {
    let path = get_state_file_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"))?;
    
    // 确保目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let persisted = PersistedIndexState {
        projects: state.clone(),
    };
    
    let content = serde_json::to_string_pretty(&persisted)?;
    std::fs::write(&path, content)?;
    
    crate::log_important!(info, "Saved {} index states to {:?}", state.len(), path);
    Ok(())
}
