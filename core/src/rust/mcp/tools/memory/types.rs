use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// 记忆条目结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub category: MemoryCategory,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MemoryEntry {
    /// 生成稳定的记忆ID
    /// 基于内容和创建时间的hash，确保同一记忆在不同读取时ID一致
    pub fn generate_stable_id(content: &str, created_at: &DateTime<Utc>) -> String {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        created_at.timestamp().hash(&mut hasher);
        let hash = hasher.finish();
        format!("mem_{:012x}", hash)
    }

    /// 创建新的记忆条目（自动生成稳定ID）
    pub fn new(content: String, category: MemoryCategory) -> Self {
        let now = Utc::now();
        let id = Self::generate_stable_id(&content, &now);
        Self {
            id,
            content,
            category,
            created_at: now,
            updated_at: now,
        }
    }

    /// 从已有数据创建（用于解析文件时）
    pub fn from_content_with_timestamp(
        content: String, 
        category: MemoryCategory, 
        created_at: DateTime<Utc>
    ) -> Self {
        let id = Self::generate_stable_id(&content, &created_at);
        Self {
            id,
            content,
            category,
            created_at,
            updated_at: created_at,
        }
    }
}

/// 分页列表结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryListResult {
    pub memories: Vec<MemoryEntry>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// 记忆分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MemoryCategory {
    Rule,        // 开发规范和规则
    Preference,  // 用户偏好设置
    Pattern,     // 常用模式和最佳实践
    Context,     // 项目上下文信息
}

/// 记忆元数据
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub project_path: String,
    pub last_organized: DateTime<Utc>,
    pub total_entries: usize,
    pub version: String,
}

// ============================================================================
// 代码修改轨迹记忆 (Code Change Memory)
// ============================================================================

/// 代码修改类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    /// Bug 修复
    BugFix,
    /// 新功能
    Feature,
    /// 重构
    Refactor,
    /// 性能优化
    Optimization,
    /// 文档更新
    Documentation,
    /// 其他
    Other,
}

impl Default for ChangeType {
    fn default() -> Self {
        Self::Other
    }
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeType::BugFix => write!(f, "bug-fix"),
            ChangeType::Feature => write!(f, "feature"),
            ChangeType::Refactor => write!(f, "refactor"),
            ChangeType::Optimization => write!(f, "optimization"),
            ChangeType::Documentation => write!(f, "documentation"),
            ChangeType::Other => write!(f, "other"),
        }
    }
}

/// 代码修改轨迹记忆
/// 
/// 自动记录 AI 的代码修改，用于后续相似场景的召回
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChangeMemory {
    /// 唯一标识符
    pub id: String,
    /// 修改类型
    pub change_type: ChangeType,
    /// 修改的文件路径列表
    pub file_paths: Vec<String>,
    /// 涉及的符号名称（函数、类、模块等）
    pub symbols: Vec<String>,
    /// 修改摘要
    pub summary: String,
    /// 关键代码片段（可选）
    pub diff_snippet: Option<String>,
    /// 用户原始请求/意图
    pub user_intent: String,
    /// 相关关键词（用于匹配）
    pub keywords: Vec<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后被召回的时间
    pub last_recalled: Option<DateTime<Utc>>,
    /// 被召回的次数
    pub recall_count: u32,
    /// 相关性分数 (0.0 - 1.0)，会随时间衰减
    pub relevance_score: f32,
}

impl CodeChangeMemory {
    /// 生成稳定的 ID
    pub fn generate_id(summary: &str, created_at: &DateTime<Utc>) -> String {
        let mut hasher = DefaultHasher::new();
        summary.hash(&mut hasher);
        created_at.timestamp().hash(&mut hasher);
        let hash = hasher.finish();
        format!("chg_{:012x}", hash)
    }

    /// 创建新的代码修改记忆
    pub fn new(
        change_type: ChangeType,
        file_paths: Vec<String>,
        symbols: Vec<String>,
        summary: String,
        user_intent: String,
    ) -> Self {
        let now = Utc::now();
        let id = Self::generate_id(&summary, &now);
        
        // 自动提取关键词
        let keywords = Self::extract_keywords(&summary, &user_intent, &file_paths);
        
        Self {
            id,
            change_type,
            file_paths,
            symbols,
            summary,
            diff_snippet: None,
            user_intent,
            keywords,
            created_at: now,
            last_recalled: None,
            recall_count: 0,
            relevance_score: 1.0, // 新记忆初始分数为 1.0
        }
    }

    /// 从文本中提取关键词
    fn extract_keywords(summary: &str, intent: &str, paths: &[String]) -> Vec<String> {
        let mut keywords = Vec::new();
        
        // 从路径中提取目录名和文件名
        for path in paths {
            if let Some(file_name) = path.rsplit('/').next() {
                // 移除扩展名
                if let Some(name) = file_name.rsplit('.').last() {
                    if !name.is_empty() {
                        keywords.push(name.to_lowercase());
                    }
                }
            }
            // 提取目录名
            for part in path.split('/') {
                if !part.is_empty() && part != "src" && part != "lib" {
                    keywords.push(part.to_lowercase());
                }
            }
        }
        
        // 从摘要和意图中提取关键词（简单分词）
        let text = format!("{} {}", summary, intent);
        for word in text.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if clean.len() > 2 && !keywords.contains(&clean) {
                keywords.push(clean);
            }
        }
        
        // 去重并限制数量
        keywords.sort();
        keywords.dedup();
        keywords.truncate(20);
        keywords
    }

    /// 记录一次召回
    pub fn record_recall(&mut self) {
        self.last_recalled = Some(Utc::now());
        self.recall_count += 1;
        // 被召回时增强相关性
        self.relevance_score = (self.relevance_score + 0.1).min(1.0);
    }

    /// 应用时间衰减
    /// 
    /// 每过 `days` 天，分数降低 `decay_rate`
    pub fn apply_decay(&mut self, days_since_creation: i64, decay_rate: f32) {
        let decay_factor = 1.0 - (decay_rate * (days_since_creation as f32 / 30.0));
        self.relevance_score = (self.relevance_score * decay_factor).max(0.0);
    }

    /// 检查是否应该被遗忘（分数过低）
    pub fn should_forget(&self, threshold: f32) -> bool {
        self.relevance_score < threshold
    }
}

/// 代码修改记忆列表结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMemoryListResult {
    pub memories: Vec<CodeChangeMemory>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}
