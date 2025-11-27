// MCP工具注册模块
// 工具实现按各自的模块目录组织

pub mod memory;
pub mod interaction;
pub mod acemcp;
pub mod unified_store;

// 重新导出工具以便访问
pub use memory::MemoryTool;
pub use interaction::InteractionTool;
pub use acemcp::AcemcpTool;
pub use unified_store::{
    UnifiedSymbolStore, 
    UnifiedSymbol,
    IndexStats,
    FileWatcher, 
    FileChangeEvent,
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
};
