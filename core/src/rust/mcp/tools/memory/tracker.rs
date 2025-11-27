//! 代码修改轨迹追踪器
//!
//! 自动记录 AI 的代码修改，并在相似场景时召回相关记忆

use anyhow::Result;
use std::path::PathBuf;

use super::storage::SqliteStorage;
use super::types::{CodeChangeMemory, ChangeType};

/// 代码修改追踪器
/// 
/// 负责：
/// - 记录代码修改
/// - 搜索相关修改历史
/// - 管理记忆衰减
pub struct ChangeTracker {
    storage: SqliteStorage,
    project_path: String,
}

impl ChangeTracker {
    /// 创建新的追踪器
    pub fn new(project_path: &str) -> Result<Self> {
        let normalized = Self::normalize_path(project_path);
        let memory_dir = PathBuf::from(&normalized).join(".neurospec-memory");
        
        std::fs::create_dir_all(&memory_dir)?;
        
        let storage = SqliteStorage::new(&memory_dir, &normalized)?;
        
        Ok(Self {
            storage,
            project_path: normalized,
        })
    }

    /// 规范化路径
    fn normalize_path(path: &str) -> String {
        let p = PathBuf::from(path);
        if let Ok(canonical) = p.canonicalize() {
            canonical.to_string_lossy().to_string()
        } else {
            path.to_string()
        }
    }

    // ========================================================================
    // 记录修改
    // ========================================================================

    /// 记录一次代码修改
    /// 
    /// # Arguments
    /// * `change_type` - 修改类型
    /// * `file_paths` - 修改的文件列表
    /// * `symbols` - 涉及的符号
    /// * `summary` - 修改摘要
    /// * `user_intent` - 用户原始请求
    pub fn record_change(
        &self,
        change_type: ChangeType,
        file_paths: Vec<String>,
        symbols: Vec<String>,
        summary: String,
        user_intent: String,
    ) -> Result<String> {
        let memory = CodeChangeMemory::new(
            change_type,
            file_paths,
            symbols,
            summary,
            user_intent,
        );
        
        self.storage.add_change_memory(&memory)
    }

    /// 记录修改并附加代码片段
    pub fn record_change_with_diff(
        &self,
        change_type: ChangeType,
        file_paths: Vec<String>,
        symbols: Vec<String>,
        summary: String,
        user_intent: String,
        diff_snippet: String,
    ) -> Result<String> {
        let mut memory = CodeChangeMemory::new(
            change_type,
            file_paths,
            symbols,
            summary,
            user_intent,
        );
        memory.diff_snippet = Some(diff_snippet);
        
        self.storage.add_change_memory(&memory)
    }

    // ========================================================================
    // 搜索相关记忆
    // ========================================================================

    /// 根据当前上下文搜索相关的修改记忆
    /// 
    /// # Arguments
    /// * `file_paths` - 当前正在修改的文件
    /// * `user_intent` - 用户当前的请求
    /// * `limit` - 返回数量限制
    pub fn find_relevant_changes(
        &self,
        file_paths: &[String],
        user_intent: &str,
        limit: usize,
    ) -> Result<Vec<CodeChangeMemory>> {
        let mut all_results = Vec::new();
        
        // 1. 按文件路径搜索
        for path in file_paths {
            if let Ok(memories) = self.storage.search_by_file_path(path, limit) {
                for mem in memories {
                    if !all_results.iter().any(|m: &CodeChangeMemory| m.id == mem.id) {
                        all_results.push(mem);
                    }
                }
            }
        }
        
        // 2. 按关键词搜索
        let keywords = Self::extract_keywords_from_intent(user_intent);
        if !keywords.is_empty() {
            if let Ok(memories) = self.storage.search_change_memories(&keywords, limit) {
                for mem in memories {
                    if !all_results.iter().any(|m: &CodeChangeMemory| m.id == mem.id) {
                        all_results.push(mem);
                    }
                }
            }
        }
        
        // 3. 按相关性排序
        all_results.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // 4. 限制数量
        all_results.truncate(limit);
        
        // 5. 记录召回
        for mem in &all_results {
            let _ = self.storage.record_change_recall(&mem.id);
        }
        
        Ok(all_results)
    }

    /// 从用户意图中提取关键词
    fn extract_keywords_from_intent(intent: &str) -> Vec<String> {
        intent
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
            .filter(|s| s.len() > 2)
            .collect()
    }

    /// 获取所有修改记忆
    pub fn get_all_changes(&self) -> Result<Vec<CodeChangeMemory>> {
        self.storage.get_all_change_memories()
    }

    // ========================================================================
    // 记忆管理
    // ========================================================================

    /// 应用记忆衰减
    /// 
    /// 默认每 30 天衰减 10%
    pub fn apply_decay(&self) -> Result<usize> {
        self.storage.apply_memory_decay(0.1)
    }

    /// 清理低分记忆
    /// 
    /// 删除相关性分数低于阈值的记忆
    pub fn cleanup(&self, threshold: f32) -> Result<usize> {
        self.storage.cleanup_low_score_memories(threshold)
    }

    /// 执行完整的维护（衰减 + 清理）
    pub fn maintenance(&self) -> Result<(usize, usize)> {
        let decayed = self.apply_decay()?;
        let cleaned = self.cleanup(0.1)?; // 清理分数低于 0.1 的记忆
        Ok((decayed, cleaned))
    }
}

// ============================================================================
// 便捷函数
// ============================================================================

/// 从修改摘要自动推断修改类型
pub fn infer_change_type(summary: &str, user_intent: &str) -> ChangeType {
    let text = format!("{} {}", summary, user_intent).to_lowercase();
    
    if text.contains("fix") || text.contains("bug") || text.contains("修复") || text.contains("错误") {
        ChangeType::BugFix
    } else if text.contains("refactor") || text.contains("重构") || text.contains("优化代码") {
        ChangeType::Refactor
    } else if text.contains("optimize") || text.contains("性能") || text.contains("优化") {
        ChangeType::Optimization
    } else if text.contains("doc") || text.contains("文档") || text.contains("注释") {
        ChangeType::Documentation
    } else if text.contains("add") || text.contains("feature") || text.contains("新增") || text.contains("添加") {
        ChangeType::Feature
    } else {
        ChangeType::Other
    }
}

/// 格式化修改记忆为可读文本
pub fn format_change_memory(memory: &CodeChangeMemory) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("### {} ({})\n", memory.summary, memory.change_type));
    output.push_str(&format!("📅 {}\n", memory.created_at.format("%Y-%m-%d %H:%M")));
    output.push_str(&format!("📁 Files: {}\n", memory.file_paths.join(", ")));
    
    if !memory.symbols.is_empty() {
        output.push_str(&format!("🔤 Symbols: {}\n", memory.symbols.join(", ")));
    }
    
    output.push_str(&format!("💬 Intent: {}\n", memory.user_intent));
    
    if let Some(ref diff) = memory.diff_snippet {
        output.push_str("```\n");
        output.push_str(diff);
        output.push_str("\n```\n");
    }
    
    output.push_str(&format!("📊 Score: {:.2} | Recalls: {}\n", 
        memory.relevance_score, memory.recall_count));
    
    output
}
