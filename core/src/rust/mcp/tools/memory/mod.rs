//! 记忆管理工具模块
//!
//! 提供全局记忆管理功能，用于存储和管理重要的开发规范、用户偏好和最佳实践

pub mod ai_suggester;
pub mod commands;
pub mod integration;
pub mod manager;
pub mod mcp;
pub mod retrieval;
pub mod storage;
pub mod tracker;
pub mod types;

// 重新导出主要类型和功能
pub use ai_suggester::{MemorySuggester, MemorySuggestion, MemoryUsageStats, ConversationContext};
pub use commands::{memory_list, memory_add, memory_update, memory_delete};
pub use integration::{GitIntegration, GitSuggestion, MemoryExporter, ExportFormat};
pub use manager::{MemoryManager, StorageBackend};
pub use mcp::MemoryTool;
pub use retrieval::{MemoryRanker, ScoredMemory, RankingConfig, TfIdfEngine};
pub use storage::{MemoryStorage, SqliteStorage, FileStorage, MigrationManager};
pub use types::{
    MemoryEntry, MemoryCategory, MemoryMetadata, MemoryListResult,
    // 代码修改轨迹记忆
    CodeChangeMemory, ChangeType, ChangeMemoryListResult,
};
pub use tracker::{ChangeTracker, infer_change_type, format_change_memory};
