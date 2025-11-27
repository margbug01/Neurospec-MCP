//! 文件存储后端实现（兼容旧版）

use anyhow::Result;
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

use super::traits::{MemoryStorage, MemoryUsageStat};
use crate::mcp::tools::memory::types::{MemoryEntry, MemoryCategory, MemoryListResult, MemoryMetadata};

/// 文件存储实现（兼容旧版 .md 文件格式）
pub struct FileStorage {
    memory_dir: PathBuf,
    project_path: String,
}

impl FileStorage {
    pub fn new(memory_dir: PathBuf, project_path: String) -> Result<Self> {
        fs::create_dir_all(&memory_dir)?;
        
        let storage = Self { memory_dir, project_path };
        storage.initialize_structure()?;
        
        Ok(storage)
    }

    fn initialize_structure(&self) -> Result<()> {
        let categories = [
            (MemoryCategory::Rule, "rules.md"),
            (MemoryCategory::Preference, "preferences.md"),
            (MemoryCategory::Pattern, "patterns.md"),
            (MemoryCategory::Context, "context.md"),
        ];

        for (category, filename) in categories.iter() {
            let file_path = self.memory_dir.join(filename);
            if !file_path.exists() {
                let header = self.get_category_header(category);
                fs::write(&file_path, header)?;
            }
        }

        Ok(())
    }

    fn get_category_filename(category: &MemoryCategory) -> &'static str {
        match category {
            MemoryCategory::Rule => "rules.md",
            MemoryCategory::Preference => "preferences.md",
            MemoryCategory::Pattern => "patterns.md",
            MemoryCategory::Context => "context.md",
        }
    }

    fn get_category_header(&self, category: &MemoryCategory) -> String {
        let title = match category {
            MemoryCategory::Rule => "开发规范和规则",
            MemoryCategory::Preference => "用户偏好设置",
            MemoryCategory::Pattern => "常用模式和最佳实践",
            MemoryCategory::Context => "项目上下文信息",
        };
        format!("# {}\n\n", title)
    }

    fn parse_memory_file(&self, content: &str, category: MemoryCategory) -> Vec<MemoryEntry> {
        let mut memories = Vec::new();
        let mut line_index: i64 = 0;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("- ") && line.len() > 2 {
                let memory_content = line[2..].trim();
                if !memory_content.is_empty() {
                    let pseudo_timestamp = chrono::DateTime::from_timestamp(
                        1700000000 + line_index, 0
                    ).unwrap_or_else(Utc::now);
                    
                    let entry = MemoryEntry::from_content_with_timestamp(
                        memory_content.to_string(),
                        category,
                        pseudo_timestamp,
                    );
                    memories.push(entry);
                }
                line_index += 1;
            }
        }

        memories
    }

    fn rewrite_category_file(&self, category: MemoryCategory, memories: &[MemoryEntry]) -> Result<()> {
        let filename = Self::get_category_filename(&category);
        let file_path = self.memory_dir.join(filename);
        
        let mut content = self.get_category_header(&category);
        for memory in memories {
            content.push_str(&format!("- {}\n", memory.content));
        }

        fs::write(&file_path, content)?;
        Ok(())
    }

    /// 检查是否存在旧版文件数据
    pub fn has_legacy_data(&self) -> bool {
        let files = ["rules.md", "preferences.md", "patterns.md", "context.md"];
        files.iter().any(|f| {
            let path = self.memory_dir.join(f);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    content.lines().any(|l| l.trim().starts_with("- "))
                } else {
                    false
                }
            } else {
                false
            }
        })
    }
}


impl MemoryStorage for FileStorage {
    fn add(&self, entry: &MemoryEntry) -> Result<String> {
        let filename = Self::get_category_filename(&entry.category);
        let file_path = self.memory_dir.join(filename);
        
        let mut content = if file_path.exists() {
            fs::read_to_string(&file_path)?
        } else {
            self.get_category_header(&entry.category)
        };

        content.push_str(&format!("- {}\n", entry.content));
        fs::write(&file_path, content)?;

        Ok(entry.id.clone())
    }

    fn delete(&self, id: &str) -> Result<bool> {
        let categories = [
            MemoryCategory::Rule,
            MemoryCategory::Preference,
            MemoryCategory::Pattern,
            MemoryCategory::Context,
        ];

        for category in categories.iter() {
            let filename = Self::get_category_filename(category);
            let file_path = self.memory_dir.join(filename);
            
            if !file_path.exists() {
                continue;
            }

            let content = fs::read_to_string(&file_path)?;
            let memories = self.parse_memory_file(&content, *category);
            let original_count = memories.len();
            
            let filtered: Vec<_> = memories.into_iter()
                .filter(|m| m.id != id)
                .collect();

            if filtered.len() < original_count {
                self.rewrite_category_file(*category, &filtered)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn update(&self, id: &str, new_content: &str) -> Result<bool> {
        let categories = [
            MemoryCategory::Rule,
            MemoryCategory::Preference,
            MemoryCategory::Pattern,
            MemoryCategory::Context,
        ];

        for category in categories.iter() {
            let filename = Self::get_category_filename(category);
            let file_path = self.memory_dir.join(filename);
            
            if !file_path.exists() {
                continue;
            }

            let content = fs::read_to_string(&file_path)?;
            let mut memories = self.parse_memory_file(&content, *category);
            
            let mut found = false;
            for memory in memories.iter_mut() {
                if memory.id == id {
                    memory.content = new_content.to_string();
                    memory.updated_at = Utc::now();
                    found = true;
                    break;
                }
            }

            if found {
                self.rewrite_category_file(*category, &memories)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn get_by_id(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let all = self.get_all()?;
        Ok(all.into_iter().find(|m| m.id == id))
    }

    fn get_all(&self) -> Result<Vec<MemoryEntry>> {
        let mut memories = Vec::new();

        let categories = [
            (MemoryCategory::Rule, "rules.md"),
            (MemoryCategory::Preference, "preferences.md"),
            (MemoryCategory::Pattern, "patterns.md"),
            (MemoryCategory::Context, "context.md"),
        ];

        for (category, filename) in categories.iter() {
            let file_path = self.memory_dir.join(filename);
            if file_path.exists() {
                let content = fs::read_to_string(&file_path)?;
                let entries = self.parse_memory_file(&content, *category);
                memories.extend(entries);
            }
        }

        memories.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(memories)
    }

    fn get_by_category(&self, category: MemoryCategory) -> Result<Vec<MemoryEntry>> {
        let filename = Self::get_category_filename(&category);
        let file_path = self.memory_dir.join(filename);
        
        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&file_path)?;
        Ok(self.parse_memory_file(&content, category))
    }

    fn list(&self, category: Option<MemoryCategory>, page: usize, page_size: usize) -> Result<MemoryListResult> {
        let memories = if let Some(cat) = category {
            self.get_by_category(cat)?
        } else {
            self.get_all()?
        };

        let total = memories.len();
        let total_pages = (total + page_size - 1) / page_size;
        let page = page.max(1);
        
        let start = (page - 1) * page_size;
        let end = (start + page_size).min(total);
        
        let page_memories = if start < total {
            memories[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok(MemoryListResult {
            memories: page_memories,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    fn count(&self, category: Option<MemoryCategory>) -> Result<usize> {
        let memories = if let Some(cat) = category {
            self.get_by_category(cat)?
        } else {
            self.get_all()?
        };
        Ok(memories.len())
    }

    fn record_usage(&self, _memory_id: &str) -> Result<()> {
        // 文件存储不支持使用统计
        Ok(())
    }

    fn get_usage_stats(&self, _memory_id: &str) -> Result<Option<MemoryUsageStat>> {
        // 文件存储不支持使用统计
        Ok(None)
    }

    fn get_metadata(&self) -> Result<MemoryMetadata> {
        let total = self.count(None)?;
        
        Ok(MemoryMetadata {
            project_path: self.project_path.clone(),
            last_organized: Utc::now(),
            total_entries: total,
            version: "file-v1".to_string(),
        })
    }

    fn update_metadata(&self) -> Result<()> {
        let metadata = self.get_metadata()?;
        let metadata_path = self.memory_dir.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(metadata_path, metadata_json)?;
        Ok(())
    }
}
