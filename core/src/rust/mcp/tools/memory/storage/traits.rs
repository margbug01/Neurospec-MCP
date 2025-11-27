//! 存储后端 trait 定义

use anyhow::Result;
use crate::mcp::tools::memory::types::{MemoryEntry, MemoryCategory, MemoryListResult, MemoryMetadata};

/// 记忆使用统计
#[derive(Debug, Clone)]
pub struct MemoryUsageStat {
    pub memory_id: String,
    pub usage_count: u32,
    pub last_used_at: Option<i64>,
    pub contributed_count: u32,
}

/// 记忆存储后端 trait
/// 
/// 所有存储实现（文件、SQLite等）都需要实现此 trait
pub trait MemoryStorage: Send + Sync {
    /// 添加记忆
    fn add(&self, entry: &MemoryEntry) -> Result<String>;
    
    /// 删除记忆
    fn delete(&self, id: &str) -> Result<bool>;
    
    /// 更新记忆
    fn update(&self, id: &str, new_content: &str) -> Result<bool>;
    
    /// 根据ID获取记忆
    fn get_by_id(&self, id: &str) -> Result<Option<MemoryEntry>>;
    
    /// 获取所有记忆
    fn get_all(&self) -> Result<Vec<MemoryEntry>>;
    
    /// 按分类获取记忆
    fn get_by_category(&self, category: MemoryCategory) -> Result<Vec<MemoryEntry>>;
    
    /// 分页获取记忆
    fn list(&self, category: Option<MemoryCategory>, page: usize, page_size: usize) -> Result<MemoryListResult>;
    
    /// 获取记忆总数
    fn count(&self, category: Option<MemoryCategory>) -> Result<usize>;
    
    /// 记录记忆使用
    fn record_usage(&self, memory_id: &str) -> Result<()>;
    
    /// 获取使用统计
    fn get_usage_stats(&self, memory_id: &str) -> Result<Option<MemoryUsageStat>>;
    
    /// 获取元数据
    fn get_metadata(&self) -> Result<MemoryMetadata>;
    
    /// 更新元数据
    fn update_metadata(&self) -> Result<()>;
}
