//! 统一符号存储模块
//!
//! 为 Search、X-Ray、Graph 提供共享的底层索引基础设施
//! - 符号提取（一次扫描，多方共享）
//! - 增量更新（文件变化时自动更新）
//! - 文件监听（统一的变化检测）
//! - 全局单例（应用生命周期内共享）

pub mod store;
pub mod watcher;
pub mod global;

pub use store::{UnifiedSymbolStore, UnifiedSymbol, IndexStats};
pub use watcher::{FileWatcher, FileChangeEvent};
pub use global::{
    init_global_store,
    get_global_store,
    with_global_store,
    init_global_watcher,
    watch_project,
    process_file_changes,
    // 搜索引擎相关
    init_global_search_config,
    get_global_search_config,
    create_searcher_for_project,
    is_search_initialized,
    // 索引状态管理
    ProjectIndexState,
    is_project_indexed,
    is_project_indexing,
    mark_indexing_started,
    mark_indexing_complete,
    get_index_state,
    get_indexed_file_count,
};
