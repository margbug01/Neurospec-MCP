//! 记忆管理器
//!
//! 提供统一的记忆管理接口，支持多种存储后端

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::storage::{MemoryStorage, SqliteStorage, FileStorage, MigrationManager};
use super::types::{MemoryEntry, MemoryCategory, MemoryListResult};

/// 存储后端类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageBackend {
    /// SQLite 存储（默认，推荐）
    Sqlite,
    /// 文件存储（兼容旧版）
    File,
}

/// 记忆管理器
pub struct MemoryManager {
    storage: Arc<dyn MemoryStorage>,
    #[allow(dead_code)] // 保留用于未来诊断/调试
    memory_dir: PathBuf,
    #[allow(dead_code)] // 保留用于未来诊断/调试
    project_path: String,
    backend: StorageBackend,
}

impl MemoryManager {
    /// 创建新的记忆管理器（默认使用 SQLite）
    pub fn new(project_path: &str) -> Result<Self> {
        Self::with_backend(project_path, StorageBackend::Sqlite)
    }

    /// 使用指定后端创建记忆管理器
    pub fn with_backend(project_path: &str, backend: StorageBackend) -> Result<Self> {
        let normalized_path = Self::normalize_project_path(project_path)?;
        let memory_dir = normalized_path.join(".neurospec-memory");

        fs::create_dir_all(&memory_dir)
            .map_err(|e| anyhow::anyhow!(
                "无法创建记忆目录: {}\n错误: {}",
                memory_dir.display(), e
            ))?;

        let project_path_str = normalized_path.to_string_lossy().to_string();

        // 检查是否需要迁移
        let migration_manager = MigrationManager::new(
            memory_dir.clone(),
            project_path_str.clone()
        );

        if migration_manager.needs_migration() && backend == StorageBackend::Sqlite {
            // 执行自动迁移
            let result = migration_manager.migrate()?;
            if !result.is_success() {
                // 迁移失败，回退到文件存储
                return Self::create_with_file_storage(memory_dir, project_path_str);
            }
        }

        // 创建存储后端
        let storage: Arc<dyn MemoryStorage> = match backend {
            StorageBackend::Sqlite => {
                Arc::new(SqliteStorage::new(&memory_dir, &project_path_str)?)
            }
            StorageBackend::File => {
                Arc::new(FileStorage::new(memory_dir.clone(), project_path_str.clone())?)
            }
        };

        Ok(Self {
            storage,
            memory_dir,
            project_path: project_path_str,
            backend,
        })
    }

    /// 使用文件存储创建（内部方法）
    fn create_with_file_storage(memory_dir: PathBuf, project_path: String) -> Result<Self> {
        let storage = Arc::new(FileStorage::new(memory_dir.clone(), project_path.clone())?);
        
        Ok(Self {
            storage,
            memory_dir,
            project_path,
            backend: StorageBackend::File,
        })
    }

    /// 获取当前存储后端类型
    pub fn backend(&self) -> StorageBackend {
        self.backend
    }

    /// 添加记忆条目
    pub fn add_memory(&self, content: &str, category: MemoryCategory) -> Result<String> {
        let entry = MemoryEntry::new(content.to_string(), category);
        self.storage.add(&entry)
    }

    /// 删除记忆条目
    pub fn delete_memory(&self, id: &str) -> Result<bool> {
        self.storage.delete(id)
    }

    /// 更新记忆条目
    pub fn update_memory(&self, id: &str, new_content: &str) -> Result<bool> {
        self.storage.update(id, new_content)
    }

    /// 分页获取记忆列表
    pub fn list_memories(
        &self,
        category: Option<MemoryCategory>,
        page: usize,
        page_size: usize,
    ) -> Result<MemoryListResult> {
        self.storage.list(category, page, page_size)
    }

    /// 根据ID获取单个记忆
    pub fn get_memory_by_id(&self, id: &str) -> Result<Option<MemoryEntry>> {
        self.storage.get_by_id(id)
    }

    /// 获取所有记忆
    pub fn get_all_memories(&self) -> Result<Vec<MemoryEntry>> {
        self.storage.get_all()
    }

    /// 获取指定分类的记忆
    pub fn get_memories_by_category(&self, category: MemoryCategory) -> Result<Vec<MemoryEntry>> {
        self.storage.get_by_category(category)
    }

    /// 记录记忆使用
    pub fn record_usage(&self, memory_id: &str) -> Result<()> {
        self.storage.record_usage(memory_id)
    }

    /// 智能召回：基于上下文返回相关记忆
    pub fn smart_recall(
        &self,
        context: Option<&str>,
        limit: usize,
        categories: Option<Vec<MemoryCategory>>,
    ) -> Result<Vec<super::retrieval::ScoredMemory>> {
        use super::retrieval::MemoryRanker;

        let all_memories = self.storage.get_all()?;
        if all_memories.is_empty() {
            return Ok(Vec::new());
        }

        // 按分类过滤
        let filtered_memories: Vec<MemoryEntry> = if let Some(cats) = categories {
            all_memories.into_iter()
                .filter(|m| cats.contains(&m.category))
                .collect()
        } else {
            all_memories
        };

        if filtered_memories.is_empty() {
            return Ok(Vec::new());
        }

        // 收集使用统计
        let usage_stats: Vec<(String, super::storage::MemoryUsageStat)> = filtered_memories.iter()
            .filter_map(|m| {
                self.storage.get_usage_stats(&m.id).ok().flatten()
                    .map(|stat| (m.id.clone(), stat))
            })
            .collect();

        // 构建排序器并排序
        let mut ranker = MemoryRanker::new();
        ranker.build_index(&filtered_memories);

        let query = context.unwrap_or("");
        let scored = ranker.rank(query, &filtered_memories, &usage_stats, limit);

        Ok(scored)
    }

    /// 获取项目信息供MCP调用方分析（智能版本）
    pub fn get_project_info_smart(&self, context: Option<&str>, limit: usize) -> Result<String> {
        let scored_memories = self.smart_recall(context, limit, None)?;
        
        if scored_memories.is_empty() {
            return Ok("📭 暂无项目记忆".to_string());
        }

        let mut output = String::new();
        output.push_str("📚 相关项目记忆:\n\n");

        // 去重：使用 HashSet 存储已见过的内容
        let mut seen = std::collections::HashSet::new();
        let mut index = 1;

        for sm in scored_memories.iter() {
            let content = sm.memory.content.trim();
            // 只显示第一次出现的内容
            if seen.insert(content.to_string()) {
                let category_icon = match sm.memory.category {
                    MemoryCategory::Rule => "🔵",
                    MemoryCategory::Preference => "🟢",
                    MemoryCategory::Pattern => "🟡",
                    MemoryCategory::Context => "⚪",
                };
                
                output.push_str(&format!(
                    "{}. {} {}\n",
                    index,
                    category_icon,
                    content
                ));
                index += 1;
            }
        }

        Ok(output)
    }

    /// 获取项目信息供MCP调用方分析
    pub fn get_project_info(&self) -> Result<String> {
        let all_memories = self.storage.get_all()?;
        if all_memories.is_empty() {
            return Ok("📭 暂无项目记忆".to_string());
        }

        let mut compressed_info = Vec::new();
        let categories = [
            (MemoryCategory::Rule, "规范"),
            (MemoryCategory::Preference, "偏好"),
            (MemoryCategory::Pattern, "模式"),
            (MemoryCategory::Context, "背景"),
        ];

        for (category, title) in categories.iter() {
            let memories = self.storage.get_by_category(*category)?;
            if !memories.is_empty() {
                // 去重：使用 HashSet 存储已见过的内容
                let mut seen = std::collections::HashSet::new();
                let items: Vec<String> = memories.iter()
                    .filter_map(|m| {
                        let content = m.content.trim();
                        if content.is_empty() {
                            None
                        } else {
                            let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
                            // 只保留第一次出现的内容
                            if seen.insert(normalized.clone()) {
                                Some(normalized)
                            } else {
                                None
                            }
                        }
                    })
                    .collect();
                
                if !items.is_empty() {
                    compressed_info.push(format!("**{}**: {}", title, items.join("; ")));
                }
            }
        }

        if compressed_info.is_empty() {
            Ok("📭 暂无有效项目记忆".to_string())
        } else {
            Ok(format!("📚 项目记忆总览: {}", compressed_info.join(" | ")))
        }
    }

    // ========== 路径处理方法 ==========

    fn normalize_project_path(project_path: &str) -> Result<PathBuf> {
        let normalized_path_str = crate::mcp::utils::decode_and_normalize_path(project_path)
            .map_err(|e| anyhow::anyhow!("路径格式错误: {}", e))?;

        let path = Path::new(&normalized_path_str);
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        let canonical_path = absolute_path.canonicalize()
            .unwrap_or_else(|_| Self::manual_canonicalize(&absolute_path).unwrap_or(absolute_path));

        if !canonical_path.exists() {
            return Err(anyhow::anyhow!(
                "项目路径不存在: {}", canonical_path.display()
            ));
        }

        if !canonical_path.is_dir() {
            return Err(anyhow::anyhow!(
                "项目路径不是目录: {}", canonical_path.display()
            ));
        }

        if let Some(git_root) = Self::find_git_root(&canonical_path) {
            Ok(git_root)
        } else {
            Err(anyhow::anyhow!(
                "项目路径不在 git 仓库中: {}", canonical_path.display()
            ))
        }
    }

    fn manual_canonicalize(path: &Path) -> Result<PathBuf> {
        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => { components.pop(); }
                _ => { components.push(component); }
            }
        }
        let mut result = PathBuf::new();
        for component in components {
            result.push(component);
        }
        Ok(result)
    }

    fn find_git_root(start_path: &Path) -> Option<PathBuf> {
        let mut current_path = start_path;
        loop {
            if current_path.join(".git").exists() {
                return Some(current_path.to_path_buf());
            }
            match current_path.parent() {
                Some(parent) => current_path = parent,
                None => break,
            }
        }
        None
    }
}
