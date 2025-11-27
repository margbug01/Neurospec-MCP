//! 数据迁移管理器
//!
//! 负责从旧版文件存储迁移到 SQLite 存储

use anyhow::Result;
use std::path::PathBuf;

use super::file::FileStorage;
use super::sqlite::SqliteStorage;
use super::traits::MemoryStorage;

/// 迁移管理器
pub struct MigrationManager {
    memory_dir: PathBuf,
    project_path: String,
}

impl MigrationManager {
    pub fn new(memory_dir: PathBuf, project_path: String) -> Self {
        Self { memory_dir, project_path }
    }

    /// 检查是否需要迁移
    pub fn needs_migration(&self) -> bool {
        let db_path = self.memory_dir.join("memory.db");
        let has_db = db_path.exists();
        
        // 如果已有数据库，检查是否有未迁移的文件数据
        if has_db {
            return false;
        }

        // 检查是否有旧版文件数据
        let file_storage = FileStorage::new(
            self.memory_dir.clone(),
            self.project_path.clone()
        ).ok();

        file_storage.map(|fs| fs.has_legacy_data()).unwrap_or(false)
    }

    /// 执行迁移
    pub fn migrate(&self) -> Result<MigrationResult> {
        let mut result = MigrationResult::default();

        // 创建文件存储读取旧数据
        let file_storage = FileStorage::new(
            self.memory_dir.clone(),
            self.project_path.clone()
        )?;

        // 获取所有旧记忆
        let old_memories = file_storage.get_all()?;
        result.total_found = old_memories.len();

        if old_memories.is_empty() {
            return Ok(result);
        }

        // 创建 SQLite 存储
        let sqlite_storage = SqliteStorage::new(&self.memory_dir, &self.project_path)?;

        // 迁移每条记忆
        for memory in old_memories {
            match sqlite_storage.add(&memory) {
                Ok(_) => result.migrated += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(format!("Failed to migrate '{}': {}", 
                        memory.content.chars().take(50).collect::<String>(), e));
                }
            }
        }

        // 备份旧文件（重命名为 .bak）
        self.backup_old_files()?;
        result.backup_created = true;

        Ok(result)
    }

    /// 备份旧文件
    fn backup_old_files(&self) -> Result<()> {
        let files = ["rules.md", "preferences.md", "patterns.md", "context.md"];
        
        for filename in files.iter() {
            let old_path = self.memory_dir.join(filename);
            if old_path.exists() {
                let backup_path = self.memory_dir.join(format!("{}.bak", filename));
                std::fs::rename(&old_path, &backup_path)?;
            }
        }

        Ok(())
    }

    /// 回滚迁移（恢复旧文件）
    pub fn rollback(&self) -> Result<()> {
        let files = ["rules.md", "preferences.md", "patterns.md", "context.md"];
        
        for filename in files.iter() {
            let backup_path = self.memory_dir.join(format!("{}.bak", filename));
            if backup_path.exists() {
                let original_path = self.memory_dir.join(filename);
                std::fs::rename(&backup_path, &original_path)?;
            }
        }

        // 删除数据库文件
        let db_path = self.memory_dir.join("memory.db");
        if db_path.exists() {
            std::fs::remove_file(&db_path)?;
        }

        Ok(())
    }
}

/// 迁移结果
#[derive(Debug, Default)]
pub struct MigrationResult {
    pub total_found: usize,
    pub migrated: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub backup_created: bool,
}

impl MigrationResult {
    pub fn is_success(&self) -> bool {
        self.failed == 0 && self.migrated == self.total_found
    }

    pub fn summary(&self) -> String {
        if self.total_found == 0 {
            return "No data to migrate".to_string();
        }

        let mut summary = format!(
            "Migration complete: {}/{} memories migrated",
            self.migrated, self.total_found
        );

        if self.failed > 0 {
            summary.push_str(&format!(", {} failed", self.failed));
        }

        if self.backup_created {
            summary.push_str(" (backup created)");
        }

        summary
    }
}
