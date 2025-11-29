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

/// 统一索引状态机
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IndexState {
    /// 未索引
    NotIndexed,
    /// 正在索引
    Indexing {
        started_at: u64,
        #[serde(default)]
        progress: f32,
    },
    /// 索引就绪
    Ready {
        file_count: usize,
        indexed_at: u64,
        #[serde(default)]
        embedding_status: EmbeddingStatus,
    },
    /// 索引损坏
    Corrupted {
        reason: String,
    },
    /// 索引过期（需要重建）
    Stale {
        file_count: usize,
        last_indexed_at: u64,
    },
}

impl Default for IndexState {
    fn default() -> Self {
        Self::NotIndexed
    }
}

/// 嵌入/向量存储状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingStatus {
    #[default]
    NotAvailable,
    Available {
        files_with_vectors: usize,
    },
    Failed {
        reason: String,
    },
}

/// 项目索引状态（可持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIndexState {
    /// 统一状态机
    #[serde(default)]
    pub state: IndexState,
    /// 兼容旧版：索引是否就绪
    #[serde(default)]
    pub ready: bool,
    /// 兼容旧版：索引是否正在进行
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
            state: IndexState::NotIndexed,
            ready: false,
            indexing: false,
            last_indexed_ts: None,
            file_count: 0,
        }
    }
}

impl ProjectIndexState {
    /// 获取当前 Unix 时间戳
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
    
    /// 检查索引是否过期
    pub fn is_expired(&self) -> bool {
        match &self.state {
            IndexState::Ready { indexed_at, .. } => {
                let now = Self::current_timestamp();
                now.saturating_sub(*indexed_at) > INDEX_EXPIRY_SECS
            }
            IndexState::Stale { .. } => true,
            _ => match self.last_indexed_ts {
                Some(ts) => {
                    let now = Self::current_timestamp();
                    now.saturating_sub(ts) > INDEX_EXPIRY_SECS
                }
                None => true,
            }
        }
    }
    
    /// 检查是否正在索引
    pub fn is_indexing(&self) -> bool {
        matches!(self.state, IndexState::Indexing { .. }) || self.indexing
    }
    
    /// 检查索引是否就绪可用
    pub fn is_ready(&self) -> bool {
        matches!(self.state, IndexState::Ready { .. }) || (self.ready && !self.is_expired())
    }
    
    /// 获取文件数
    pub fn get_file_count(&self) -> usize {
        match &self.state {
            IndexState::Ready { file_count, .. } => *file_count,
            IndexState::Stale { file_count, .. } => *file_count,
            _ => self.file_count,
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

/// 规范化项目路径键（统一使用正斜杠，用于跨平台兼容）
fn normalize_project_key(project_root: &std::path::Path) -> String {
    project_root.to_string_lossy().replace('\\', "/")
}

/// 索引健康状态
#[derive(Debug, Clone, PartialEq)]
pub enum IndexHealth {
    /// 索引健康可用
    Healthy,
    /// 索引可用但不完整（仍会使用，但建议重建）
    Degraded { reason: String },
    /// 索引不可用（需要回退到 ripgrep）
    Unhealthy { reason: String },
}

/// 评估项目索引健康状态
/// 
/// 判断逻辑：
/// 1. Ready 且 indexed_count / total_count >= 0.7 → Healthy
/// 2. Ready 且 indexed_count >= 3 且 ratio >= 0.3 → Degraded
/// 3. 否则 → Unhealthy
pub fn assess_index_health(project_root: &std::path::Path) -> IndexHealth {
    let key = normalize_project_key(project_root);
    
    let state = match PROJECT_INDEX_STATE.read() {
        Ok(guard) => guard.get(&key).cloned(),
        Err(_) => return IndexHealth::Unhealthy { reason: "State lock error".to_string() },
    };
    
    let Some(project_state) = state else {
        return IndexHealth::Unhealthy { reason: "No index state".to_string() };
    };
    
    if project_state.is_indexing() {
        return IndexHealth::Degraded { reason: "Indexing in progress".to_string() };
    }
    
    if !project_state.is_ready() {
        return IndexHealth::Unhealthy { reason: "Index not ready".to_string() };
    }
    
    if project_state.is_expired() {
        return IndexHealth::Degraded { reason: "Index expired".to_string() };
    }
    
    let indexed_count = project_state.get_file_count();
    
    // 尝试获取项目实际文件数
    let total_count = count_project_files(project_root);
    
    match total_count {
        Some(total) if total > 0 => {
            let ratio = indexed_count as f64 / total as f64;
            if ratio >= 0.7 {
                IndexHealth::Healthy
            } else if indexed_count >= 3 && ratio >= 0.3 {
                IndexHealth::Degraded { 
                    reason: format!("Only {:.0}% indexed ({}/{})", ratio * 100.0, indexed_count, total) 
                }
            } else {
                IndexHealth::Unhealthy { 
                    reason: format!("Too few files indexed ({}/{})", indexed_count, total) 
                }
            }
        }
        _ => {
            // 无法获取总文件数，使用绝对阈值
            if indexed_count >= 10 {
                IndexHealth::Healthy
            } else if indexed_count >= 3 {
                IndexHealth::Degraded { reason: format!("Only {} files indexed", indexed_count) }
            } else {
                IndexHealth::Unhealthy { reason: format!("Only {} files indexed", indexed_count) }
            }
        }
    }
}

/// 统计项目代码文件数（快速估算）
fn count_project_files(project_root: &std::path::Path) -> Option<usize> {
    use ignore::WalkBuilder;
    
    let walker = WalkBuilder::new(project_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .max_depth(Some(10))
        .build();
    
    let mut count = 0;
    let code_extensions = ["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "vue", "c", "cpp", "h", "hpp"];
    
    for entry in walker.filter_map(|e| e.ok()).take(5000) {
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if code_extensions.contains(&ext) {
                    count += 1;
                }
            }
        }
    }
    
    if count > 0 { Some(count) } else { None }
}

/// 统一状态转换入口
/// 
/// 所有索引状态变更都应通过此函数，确保状态一致性和持久化
pub fn transition_index_state(project_root: &std::path::Path, new_state: IndexState) {
    let key = normalize_project_key(project_root);
    
    if let Ok(mut guard) = PROJECT_INDEX_STATE.write() {
        let project_state = guard.entry(key.clone()).or_default();
        
        // 同步更新旧字段（兼容性）
        match &new_state {
            IndexState::NotIndexed => {
                project_state.ready = false;
                project_state.indexing = false;
            }
            IndexState::Indexing { .. } => {
                project_state.indexing = true;
            }
            IndexState::Ready { file_count, indexed_at, .. } => {
                project_state.ready = true;
                project_state.indexing = false;
                project_state.file_count = *file_count;
                project_state.last_indexed_ts = Some(*indexed_at);
            }
            IndexState::Corrupted { .. } => {
                project_state.ready = false;
                project_state.indexing = false;
            }
            IndexState::Stale { file_count, last_indexed_at } => {
                project_state.ready = false;
                project_state.indexing = false;
                project_state.file_count = *file_count;
                project_state.last_indexed_ts = Some(*last_indexed_at);
            }
        }
        
        project_state.state = new_state.clone();
        
        // 持久化
        let _ = save_persisted_state(&guard);
        
        crate::log_important!(info, "Index state transition: {} -> {:?}", key, new_state);
    }
}

/// 检查项目索引是否就绪
/// 
/// 索引就绪条件：
/// 1. 已完成至少一次完整索引
/// 2. 索引未过期（24小时内）
/// 
/// 如果运行时状态没有记录，会尝试从 index_metadata.json 恢复
pub fn is_project_indexed(project_root: &std::path::Path) -> bool {
    let key = normalize_project_key(project_root);
    
    // 先检查运行时状态
    {
        let guard = match PROJECT_INDEX_STATE.read() {
            Ok(g) => g,
            Err(_) => return false,
        };
        
        if let Some(state) = guard.get(&key) {
            return state.is_ready() && !state.is_expired();
        }
    }
    
    // 运行时状态没有记录，尝试从 index_metadata.json 恢复
    if let Some(file_count) = check_index_metadata_exists(&key) {
        // 验证索引完整性
        if verify_index_integrity(project_root) {
            let now = ProjectIndexState::current_timestamp();
            transition_index_state(project_root, IndexState::Ready {
                file_count,
                indexed_at: now,
                embedding_status: EmbeddingStatus::NotAvailable,
            });
            crate::log_important!(info, "Recovered index state from metadata: {} files", file_count);
            return true;
        } else {
            transition_index_state(project_root, IndexState::Corrupted {
                reason: "Index integrity check failed".to_string(),
            });
            return false;
        }
    }
    
    false
}

/// 验证 Tantivy 索引完整性
fn verify_index_integrity(_project_root: &std::path::Path) -> bool {
    let config = match get_global_search_config() {
        Ok(c) => c,
        Err(_) => return false,
    };
    
    let index_dir = &config.index_path;
    
    // 检查索引目录是否存在
    if !index_dir.exists() {
        return false;
    }
    
    // 检查是否有 segment 文件（Tantivy 索引的基本组成）
    let has_meta = index_dir.join("meta.json").exists();
    let has_segments = std::fs::read_dir(index_dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok())
                .any(|e| e.file_name().to_string_lossy().ends_with(".managed.json"))
        })
        .unwrap_or(false);
    
    has_meta || has_segments
}

/// 检查 index_metadata.json 中是否有该项目的记录
fn check_index_metadata_exists(project_key: &str) -> Option<usize> {
    let config = get_global_search_config().ok()?;
    let metadata_path = config.index_path.join("index_metadata.json");
    
    if !metadata_path.exists() {
        return None;
    }
    
    let content = std::fs::read_to_string(&metadata_path).ok()?;
    let metadata: serde_json::Value = serde_json::from_str(&content).ok()?;
    
    // 检查 projects 字段中是否有该项目
    let projects = metadata.get("projects")?.as_object()?;
    let project_files = projects.get(project_key)?.as_object()?;
    
    Some(project_files.len())
}

/// 检查项目是否正在索引中
pub fn is_project_indexing(project_root: &std::path::Path) -> bool {
    let key = normalize_project_key(project_root);
    
    PROJECT_INDEX_STATE.read()
        .map(|guard| guard.get(&key).map(|s| s.is_indexing()).unwrap_or(false))
        .unwrap_or(false)
}

/// 标记项目开始索引
pub fn mark_indexing_started(project_root: &std::path::Path) {
    let now = ProjectIndexState::current_timestamp();
    transition_index_state(project_root, IndexState::Indexing {
        started_at: now,
        progress: 0.0,
    });
}

/// 标记项目索引完成
/// 
/// 同时启动文件监听（如果全局 watcher 已初始化）
pub fn mark_indexing_complete(project_root: &std::path::Path, file_count: usize) {
    let now = ProjectIndexState::current_timestamp();
    transition_index_state(project_root, IndexState::Ready {
        file_count,
        indexed_at: now,
        embedding_status: EmbeddingStatus::NotAvailable,
    });
    
    // 自动启动文件监听
    if let Err(e) = start_watching_project(project_root) {
        crate::log_important!(warn, "Failed to start file watching: {}", e);
    }
}

/// 标记索引为损坏状态
pub fn mark_index_corrupted(project_root: &std::path::Path, reason: &str) {
    transition_index_state(project_root, IndexState::Corrupted {
        reason: reason.to_string(),
    });
}

/// 更新嵌入状态
pub fn update_embedding_status(project_root: &std::path::Path, status: EmbeddingStatus) {
    let key = normalize_project_key(project_root);
    
    if let Ok(mut guard) = PROJECT_INDEX_STATE.write() {
        if let Some(project_state) = guard.get_mut(&key) {
            if let IndexState::Ready { file_count, indexed_at, .. } = &project_state.state {
                project_state.state = IndexState::Ready {
                    file_count: *file_count,
                    indexed_at: *indexed_at,
                    embedding_status: status,
                };
                let _ = save_persisted_state(&guard);
            }
        }
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
    let key = normalize_project_key(project_root);
    
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
