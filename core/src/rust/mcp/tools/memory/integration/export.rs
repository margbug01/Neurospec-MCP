//! 记忆导入/导出
//!
//! 支持 JSON 和 Markdown 格式

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::mcp::tools::memory::types::{MemoryEntry, MemoryCategory};

/// 导出格式
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Markdown,
}

/// 导出数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub exported_at: String,
    pub project_path: String,
    pub memories: Vec<ExportedMemory>,
}

/// 导出的记忆条目
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedMemory {
    pub id: String,
    pub content: String,
    pub category: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<MemoryEntry> for ExportedMemory {
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

/// 记忆导出器
pub struct MemoryExporter;

impl MemoryExporter {
    /// 导出为 JSON
    pub fn export_json(memories: &[MemoryEntry], project_path: &str) -> Result<String> {
        let data = ExportData {
            version: "1.0".to_string(),
            exported_at: Utc::now().to_rfc3339(),
            project_path: project_path.to_string(),
            memories: memories.iter().cloned().map(Into::into).collect(),
        };

        Ok(serde_json::to_string_pretty(&data)?)
    }

    /// 导出为 Markdown
    pub fn export_markdown(memories: &[MemoryEntry], project_path: &str) -> Result<String> {
        let mut md = String::new();
        
        md.push_str(&format!("# 项目记忆导出\n\n"));
        md.push_str(&format!("- **项目路径**: {}\n", project_path));
        md.push_str(&format!("- **导出时间**: {}\n", Utc::now().format("%Y-%m-%d %H:%M:%S")));
        md.push_str(&format!("- **记忆总数**: {}\n\n", memories.len()));

        // 按分类分组
        let categories = [
            (MemoryCategory::Rule, "规则", "🔵"),
            (MemoryCategory::Preference, "偏好", "🟢"),
            (MemoryCategory::Pattern, "模式", "🟡"),
            (MemoryCategory::Context, "上下文", "⚪"),
        ];

        for (cat, name, icon) in &categories {
            let cat_memories: Vec<_> = memories.iter().filter(|m| m.category == *cat).collect();
            if !cat_memories.is_empty() {
                md.push_str(&format!("## {} {}\n\n", icon, name));
                for mem in cat_memories {
                    md.push_str(&format!("- {}\n", mem.content));
                }
                md.push_str("\n");
            }
        }

        Ok(md)
    }

    /// 从 JSON 导入
    pub fn import_json(json_str: &str) -> Result<Vec<MemoryEntry>> {
        let data: ExportData = serde_json::from_str(json_str)?;
        
        let memories = data.memories.into_iter().map(|em| {
            let category = match em.category.as_str() {
                "rule" => MemoryCategory::Rule,
                "preference" => MemoryCategory::Preference,
                "pattern" => MemoryCategory::Pattern,
                _ => MemoryCategory::Context,
            };

            let created_at = chrono::DateTime::parse_from_rfc3339(&em.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            
            let updated_at = chrono::DateTime::parse_from_rfc3339(&em.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            MemoryEntry {
                id: em.id,
                content: em.content,
                category,
                created_at,
                updated_at,
            }
        }).collect();

        Ok(memories)
    }

    /// 导出到文件
    pub fn export_to_file(
        memories: &[MemoryEntry],
        project_path: &str,
        file_path: &str,
        format: ExportFormat,
    ) -> Result<()> {
        let content = match format {
            ExportFormat::Json => Self::export_json(memories, project_path)?,
            ExportFormat::Markdown => Self::export_markdown(memories, project_path)?,
        };

        std::fs::write(file_path, content)?;
        Ok(())
    }

    /// 从文件导入
    pub fn import_from_file(file_path: &str) -> Result<Vec<MemoryEntry>> {
        let content = std::fs::read_to_string(file_path)?;
        Self::import_json(&content)
    }
}
