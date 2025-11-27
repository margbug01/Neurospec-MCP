//! 记忆管理 Tauri 命令
//!
//! 提供前端调用的记忆管理接口

use serde::Serialize;
use tauri::command;

use super::{MemoryManager, MemoryCategory, MemoryEntry, MemoryListResult};

/// 记忆列表响应
#[derive(Debug, Serialize)]
pub struct MemoryListResponse {
    pub memories: Vec<MemoryEntryResponse>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// 记忆条目响应
#[derive(Debug, Serialize)]
pub struct MemoryEntryResponse {
    pub id: String,
    pub content: String,
    pub category: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<MemoryEntry> for MemoryEntryResponse {
    fn from(entry: MemoryEntry) -> Self {
        Self {
            id: entry.id,
            content: entry.content,
            category: match entry.category {
                MemoryCategory::Rule => "rule".to_string(),
                MemoryCategory::Preference => "preference".to_string(),
                MemoryCategory::Pattern => "pattern".to_string(),
                MemoryCategory::Context => "context".to_string(),
            },
            created_at: entry.created_at.to_rfc3339(),
            updated_at: entry.updated_at.to_rfc3339(),
        }
    }
}

impl From<MemoryListResult> for MemoryListResponse {
    fn from(result: MemoryListResult) -> Self {
        Self {
            memories: result.memories.into_iter().map(Into::into).collect(),
            total: result.total,
            page: result.page,
            page_size: result.page_size,
            total_pages: result.total_pages,
        }
    }
}

fn parse_category(category: &str) -> Option<MemoryCategory> {
    match category {
        "rule" => Some(MemoryCategory::Rule),
        "preference" => Some(MemoryCategory::Preference),
        "pattern" => Some(MemoryCategory::Pattern),
        "context" => Some(MemoryCategory::Context),
        "" | "all" => None,
        _ => None,
    }
}

/// 获取记忆列表
#[command]
pub async fn memory_list(
    project_path: String,
    category: String,
    page: usize,
    page_size: usize,
) -> Result<MemoryListResponse, String> {
    let manager = MemoryManager::new(&project_path)
        .map_err(|e| format!("创建记忆管理器失败: {}", e))?;

    let cat = parse_category(&category);
    let result = manager
        .list_memories(cat, page, page_size)
        .map_err(|e| format!("获取记忆列表失败: {}", e))?;

    Ok(result.into())
}

/// 添加记忆
#[command]
pub async fn memory_add(
    project_path: String,
    content: String,
    category: String,
) -> Result<serde_json::Value, String> {
    let manager = MemoryManager::new(&project_path)
        .map_err(|e| format!("创建记忆管理器失败: {}", e))?;

    let cat = parse_category(&category).unwrap_or(MemoryCategory::Context);
    let id = manager
        .add_memory(&content, cat)
        .map_err(|e| format!("添加记忆失败: {}", e))?;

    Ok(serde_json::json!({ "id": id }))
}

/// 更新记忆
#[command]
pub async fn memory_update(
    project_path: String,
    id: String,
    content: String,
) -> Result<(), String> {
    let manager = MemoryManager::new(&project_path)
        .map_err(|e| format!("创建记忆管理器失败: {}", e))?;

    let updated = manager
        .update_memory(&id, &content)
        .map_err(|e| format!("更新记忆失败: {}", e))?;

    if updated {
        Ok(())
    } else {
        Err("未找到指定的记忆".to_string())
    }
}

/// 删除记忆
#[command]
pub async fn memory_delete(
    project_path: String,
    id: String,
) -> Result<(), String> {
    let manager = MemoryManager::new(&project_path)
        .map_err(|e| format!("创建记忆管理器失败: {}", e))?;

    let deleted = manager
        .delete_memory(&id)
        .map_err(|e| format!("删除记忆失败: {}", e))?;

    if deleted {
        Ok(())
    } else {
        Err("未找到指定的记忆".to_string())
    }
}

/// 自动检测项目路径
/// 
/// 检测策略（优先级从高到低）：
/// 1. 从配置文件加载已保存的项目路径
/// 2. 从当前工作目录向上查找 .git 目录
#[command]
pub async fn detect_project_path() -> Result<String, String> {
    // 1. 尝试从配置文件加载
    if let Some(saved_path) = load_saved_project_path() {
        let root = std::path::PathBuf::from(&saved_path);
        if root.exists() {
            return Ok(saved_path);
        }
    }
    
    // 2. 从当前工作目录查找
    let cwd = std::env::current_dir()
        .map_err(|e| format!("无法获取当前工作目录: {}", e))?;

    let mut current = cwd.as_path();
    loop {
        if current.join(".git").exists() {
            return Ok(current.to_string_lossy().to_string());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    // 无法检测，返回空字符串（前端会提示用户手动输入）
    Ok(String::new())
}

/// 从配置文件加载已保存的项目路径
fn load_saved_project_path() -> Option<String> {
    let config_path = dirs::data_dir()?.join("neurospec").join("project_config.json");
    
    if !config_path.exists() {
        return None;
    }
    
    let content = std::fs::read_to_string(&config_path).ok()?;
    let config: serde_json::Value = serde_json::from_str(&content).ok()?;
    config.get("project_path")?.as_str().map(String::from)
}

/// 分析对话内容，返回记忆建议
#[command]
pub async fn analyze_memory_suggestions(
    messages: Vec<String>,
    project_path: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    use super::{MemorySuggester, ConversationContext};

    let context = ConversationContext {
        messages,
        project_context: project_path,
        language: None,
    };

    let suggester = MemorySuggester::new();
    let suggestions = suggester.detect_pattern(&context);

    // 转换为 JSON 格式
    let result: Vec<serde_json::Value> = suggestions
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "content": s.content,
                "category": match s.category {
                    super::MemoryCategory::Rule => "rule",
                    super::MemoryCategory::Preference => "preference",
                    super::MemoryCategory::Pattern => "pattern",
                    super::MemoryCategory::Context => "context",
                },
                "confidence": s.confidence,
                "reason": s.reason,
                "keywords": s.keywords,
                "suggested_at": s.suggested_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(result)
}
