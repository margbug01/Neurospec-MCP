//! SQLite 存储后端实现

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;

use super::traits::{MemoryStorage, MemoryUsageStat};
use crate::mcp::tools::memory::types::{
    MemoryEntry, MemoryCategory, MemoryListResult, MemoryMetadata,
    CodeChangeMemory, ChangeType,
};

const DB_FILENAME: &str = "memory.db";
const SCHEMA_VERSION: i32 = 3; // 升级到 v3 以支持向量存储

/// SQLite 存储实现
pub struct SqliteStorage {
    conn: Mutex<Connection>,
    project_path: String,
}

impl SqliteStorage {
    /// 创建新的 SQLite 存储
    pub fn new(memory_dir: &PathBuf, project_path: &str) -> Result<Self> {
        let db_path = memory_dir.join(DB_FILENAME);
        let conn = Connection::open(&db_path)?;
        
        let storage = Self {
            conn: Mutex::new(conn),
            project_path: project_path.to_string(),
        };
        
        storage.initialize_schema()?;
        Ok(storage)
    }

    /// 初始化数据库 schema
    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        // 创建 memories 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                category TEXT NOT NULL,
                project_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                is_deleted INTEGER DEFAULT 0
            )",
            [],
        )?;

        // 创建 memory_stats 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_stats (
                memory_id TEXT PRIMARY KEY,
                usage_count INTEGER DEFAULT 0,
                last_used_at INTEGER,
                contributed_count INTEGER DEFAULT 0,
                FOREIGN KEY (memory_id) REFERENCES memories(id)
            )",
            [],
        )?;

        // 创建 change_memories 表 (代码修改轨迹)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS change_memories (
                id TEXT PRIMARY KEY,
                change_type TEXT NOT NULL,
                file_paths TEXT NOT NULL,
                symbols TEXT NOT NULL,
                summary TEXT NOT NULL,
                diff_snippet TEXT,
                user_intent TEXT NOT NULL,
                keywords TEXT NOT NULL,
                project_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_recalled INTEGER,
                recall_count INTEGER DEFAULT 0,
                relevance_score REAL DEFAULT 1.0,
                is_deleted INTEGER DEFAULT 0,
                summary_embedding BLOB,
                embedding_model TEXT
            )",
            [],
        )?;

        // 创建 schema_version 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY
            )",
            [],
        )?;

        // 创建索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project_path)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(project_path, category)",
            [],
        )?;
        // 代码修改记忆索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_change_memories_project ON change_memories(project_path)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_change_memories_type ON change_memories(project_path, change_type)",
            [],
        )?;

        // 检查并更新 schema 版本
        let current_version: i32 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))
            .unwrap_or(0);

        if current_version < SCHEMA_VERSION {
            // 执行迁移
            Self::migrate_schema(&conn, current_version)?;
            conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (?1)", [SCHEMA_VERSION])?;
        }

        Ok(())
    }

    /// 执行 schema 迁移
    fn migrate_schema(conn: &Connection, from_version: i32) -> Result<()> {
        // v2 -> v3: 添加 embedding 字段
        if from_version < 3 {
            // 检查字段是否存在
            let has_embedding: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('change_memories') WHERE name='summary_embedding'",
                    [],
                    |row| row.get::<_, i32>(0),
                )
                .map(|c| c > 0)
                .unwrap_or(false);

            if !has_embedding {
                conn.execute("ALTER TABLE change_memories ADD COLUMN summary_embedding BLOB", [])?;
                conn.execute("ALTER TABLE change_memories ADD COLUMN embedding_model TEXT", [])?;
                log::info!("Migrated change_memories table to v3 (added embedding columns)");
            }
        }

        Ok(())
    }

    /// 将 MemoryCategory 转换为字符串
    fn category_to_str(category: &MemoryCategory) -> &'static str {
        match category {
            MemoryCategory::Rule => "rule",
            MemoryCategory::Preference => "preference",
            MemoryCategory::Pattern => "pattern",
            MemoryCategory::Context => "context",
        }
    }

    /// 从字符串解析 MemoryCategory
    fn str_to_category(s: &str) -> MemoryCategory {
        match s {
            "rule" => MemoryCategory::Rule,
            "preference" => MemoryCategory::Preference,
            "pattern" => MemoryCategory::Pattern,
            _ => MemoryCategory::Context,
        }
    }

    /// 从数据库行构建 MemoryEntry
    fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
        let id: String = row.get(0)?;
        let content: String = row.get(1)?;
        let category_str: String = row.get(2)?;
        let created_at_ts: i64 = row.get(3)?;
        let updated_at_ts: i64 = row.get(4)?;

        let created_at = DateTime::from_timestamp(created_at_ts, 0)
            .unwrap_or_else(Utc::now);
        let updated_at = DateTime::from_timestamp(updated_at_ts, 0)
            .unwrap_or_else(Utc::now);

        Ok(MemoryEntry {
            id,
            content,
            category: Self::str_to_category(&category_str),
            created_at,
            updated_at,
        })
    }
}


impl MemoryStorage for SqliteStorage {
    fn add(&self, entry: &MemoryEntry) -> Result<String> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        conn.execute(
            "INSERT INTO memories (id, content, category, project_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.id,
                entry.content,
                Self::category_to_str(&entry.category),
                self.project_path,
                entry.created_at.timestamp(),
                entry.updated_at.timestamp(),
            ],
        )?;

        // 初始化使用统计
        conn.execute(
            "INSERT INTO memory_stats (memory_id, usage_count, contributed_count)
             VALUES (?1, 0, 0)",
            params![entry.id],
        )?;

        Ok(entry.id.clone())
    }

    fn delete(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        // 软删除
        let rows = conn.execute(
            "UPDATE memories SET is_deleted = 1, updated_at = ?1 
             WHERE id = ?2 AND project_path = ?3 AND is_deleted = 0",
            params![Utc::now().timestamp(), id, self.project_path],
        )?;

        Ok(rows > 0)
    }

    fn update(&self, id: &str, new_content: &str) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let rows = conn.execute(
            "UPDATE memories SET content = ?1, updated_at = ?2 
             WHERE id = ?3 AND project_path = ?4 AND is_deleted = 0",
            params![new_content, Utc::now().timestamp(), id, self.project_path],
        )?;

        Ok(rows > 0)
    }

    fn get_by_id(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, content, category, created_at, updated_at 
             FROM memories 
             WHERE id = ?1 AND project_path = ?2 AND is_deleted = 0"
        )?;

        let entry = stmt.query_row(params![id, self.project_path], Self::row_to_entry).ok();
        Ok(entry)
    }

    fn get_all(&self) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, content, category, created_at, updated_at 
             FROM memories 
             WHERE project_path = ?1 AND is_deleted = 0
             ORDER BY updated_at DESC"
        )?;

        let entries = stmt.query_map(params![self.project_path], Self::row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    fn get_by_category(&self, category: MemoryCategory) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, content, category, created_at, updated_at 
             FROM memories 
             WHERE project_path = ?1 AND category = ?2 AND is_deleted = 0
             ORDER BY updated_at DESC"
        )?;

        let entries = stmt.query_map(
            params![self.project_path, Self::category_to_str(&category)],
            Self::row_to_entry
        )?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    fn list(&self, category: Option<MemoryCategory>, page: usize, page_size: usize) -> Result<MemoryListResult> {
        let total = self.count(category)?;
        let total_pages = (total + page_size - 1) / page_size;
        let page = page.max(1);
        let offset = (page - 1) * page_size;

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let memories: Vec<MemoryEntry> = if let Some(cat) = category {
            let mut stmt = conn.prepare(
                "SELECT id, content, category, created_at, updated_at 
                 FROM memories 
                 WHERE project_path = ?1 AND category = ?2 AND is_deleted = 0
                 ORDER BY updated_at DESC
                 LIMIT ?3 OFFSET ?4"
            )?;
            let rows = stmt.query_map(
                params![self.project_path, Self::category_to_str(&cat), page_size as i64, offset as i64],
                Self::row_to_entry
            )?;
            rows.filter_map(|r| r.ok()).collect()
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, content, category, created_at, updated_at 
                 FROM memories 
                 WHERE project_path = ?1 AND is_deleted = 0
                 ORDER BY updated_at DESC
                 LIMIT ?2 OFFSET ?3"
            )?;
            let rows = stmt.query_map(
                params![self.project_path, page_size as i64, offset as i64],
                Self::row_to_entry
            )?;
            rows.filter_map(|r| r.ok()).collect()
        };

        Ok(MemoryListResult {
            memories,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    fn count(&self, category: Option<MemoryCategory>) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let count: i64 = if let Some(cat) = category {
            conn.query_row(
                "SELECT COUNT(*) FROM memories WHERE project_path = ?1 AND category = ?2 AND is_deleted = 0",
                params![self.project_path, Self::category_to_str(&cat)],
                |row| row.get(0)
            )?
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM memories WHERE project_path = ?1 AND is_deleted = 0",
                params![self.project_path],
                |row| row.get(0)
            )?
        };

        Ok(count as usize)
    }

    fn record_usage(&self, memory_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        conn.execute(
            "UPDATE memory_stats 
             SET usage_count = usage_count + 1, 
                 last_used_at = ?1,
                 contributed_count = contributed_count + 1
             WHERE memory_id = ?2",
            params![Utc::now().timestamp(), memory_id],
        )?;

        Ok(())
    }

    fn get_usage_stats(&self, memory_id: &str) -> Result<Option<MemoryUsageStat>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let stat = conn.query_row(
            "SELECT memory_id, usage_count, last_used_at, contributed_count 
             FROM memory_stats WHERE memory_id = ?1",
            params![memory_id],
            |row| {
                Ok(MemoryUsageStat {
                    memory_id: row.get(0)?,
                    usage_count: row.get(1)?,
                    last_used_at: row.get(2)?,
                    contributed_count: row.get(3)?,
                })
            }
        ).ok();

        Ok(stat)
    }

    fn get_metadata(&self) -> Result<MemoryMetadata> {
        let total = self.count(None)?;
        
        Ok(MemoryMetadata {
            project_path: self.project_path.clone(),
            last_organized: Utc::now(),
            total_entries: total,
            version: format!("sqlite-v{}", SCHEMA_VERSION),
        })
    }

    fn update_metadata(&self) -> Result<()> {
        // SQLite 存储不需要单独的元数据文件
        Ok(())
    }
}

// ============================================================================
// CodeChangeMemory 存储方法
// ============================================================================

impl SqliteStorage {
    /// 将 ChangeType 转换为字符串
    fn change_type_to_str(ct: &ChangeType) -> &'static str {
        match ct {
            ChangeType::BugFix => "bug-fix",
            ChangeType::Feature => "feature",
            ChangeType::Refactor => "refactor",
            ChangeType::Optimization => "optimization",
            ChangeType::Documentation => "documentation",
            ChangeType::Other => "other",
        }
    }

    /// 从字符串解析 ChangeType
    fn str_to_change_type(s: &str) -> ChangeType {
        match s {
            "bug-fix" => ChangeType::BugFix,
            "feature" => ChangeType::Feature,
            "refactor" => ChangeType::Refactor,
            "optimization" => ChangeType::Optimization,
            "documentation" => ChangeType::Documentation,
            _ => ChangeType::Other,
        }
    }

    /// 添加代码修改记忆
    pub fn add_change_memory(&self, memory: &CodeChangeMemory) -> Result<String> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        conn.execute(
            "INSERT INTO change_memories (
                id, change_type, file_paths, symbols, summary, diff_snippet,
                user_intent, keywords, project_path, created_at, recall_count, relevance_score
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                memory.id,
                Self::change_type_to_str(&memory.change_type),
                serde_json::to_string(&memory.file_paths).unwrap_or_default(),
                serde_json::to_string(&memory.symbols).unwrap_or_default(),
                memory.summary,
                memory.diff_snippet,
                memory.user_intent,
                serde_json::to_string(&memory.keywords).unwrap_or_default(),
                self.project_path,
                memory.created_at.timestamp(),
                memory.recall_count,
                memory.relevance_score,
            ],
        )?;

        Ok(memory.id.clone())
    }

    /// 获取所有代码修改记忆
    pub fn get_all_change_memories(&self) -> Result<Vec<CodeChangeMemory>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, change_type, file_paths, symbols, summary, diff_snippet,
                    user_intent, keywords, created_at, last_recalled, recall_count, relevance_score
             FROM change_memories 
             WHERE project_path = ?1 AND is_deleted = 0
             ORDER BY created_at DESC"
        )?;

        let memories = stmt.query_map(params![self.project_path], |row| {
            Ok(self.row_to_change_memory(row))
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(memories)
    }

    /// 根据关键词搜索代码修改记忆
    pub fn search_change_memories(&self, keywords: &[String], limit: usize) -> Result<Vec<CodeChangeMemory>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        // 构建 LIKE 查询条件
        let mut conditions = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(self.project_path.clone())];
        
        for (i, kw) in keywords.iter().enumerate() {
            conditions.push(format!(
                "(keywords LIKE ?{} OR summary LIKE ?{} OR user_intent LIKE ?{})",
                i * 3 + 2, i * 3 + 3, i * 3 + 4
            ));
            let pattern = format!("%{}%", kw);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("AND ({})", conditions.join(" OR "))
        };

        let query = format!(
            "SELECT id, change_type, file_paths, symbols, summary, diff_snippet,
                    user_intent, keywords, created_at, last_recalled, recall_count, relevance_score
             FROM change_memories 
             WHERE project_path = ?1 AND is_deleted = 0 {}
             ORDER BY relevance_score DESC, created_at DESC
             LIMIT {}",
            where_clause, limit
        );

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        
        let mut stmt = conn.prepare(&query)?;
        let memories = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(self.row_to_change_memory(row))
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(memories)
    }

    /// 根据文件路径搜索相关记忆
    pub fn search_by_file_path(&self, file_path: &str, limit: usize) -> Result<Vec<CodeChangeMemory>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let pattern = format!("%{}%", file_path);
        
        let mut stmt = conn.prepare(
            "SELECT id, change_type, file_paths, symbols, summary, diff_snippet,
                    user_intent, keywords, created_at, last_recalled, recall_count, relevance_score
             FROM change_memories 
             WHERE project_path = ?1 AND is_deleted = 0 AND file_paths LIKE ?2
             ORDER BY relevance_score DESC, created_at DESC
             LIMIT ?3"
        )?;

        let memories = stmt.query_map(params![self.project_path, pattern, limit as i64], |row| {
            Ok(self.row_to_change_memory(row))
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(memories)
    }

    /// 记录代码修改记忆被召回
    pub fn record_change_recall(&self, memory_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        conn.execute(
            "UPDATE change_memories 
             SET recall_count = recall_count + 1,
                 last_recalled = ?1,
                 relevance_score = MIN(relevance_score + 0.1, 1.0)
             WHERE id = ?2",
            params![Utc::now().timestamp(), memory_id],
        )?;

        Ok(())
    }

    /// 应用记忆衰减（批量更新）
    pub fn apply_memory_decay(&self, decay_rate: f32) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        // 计算衰减因子：每 30 天降低 decay_rate
        let affected = conn.execute(
            "UPDATE change_memories 
             SET relevance_score = MAX(relevance_score * (1.0 - ?1 * ((julianday('now') - julianday(datetime(created_at, 'unixepoch'))) / 30.0)), 0.0)
             WHERE project_path = ?2 AND is_deleted = 0",
            params![decay_rate as f64, self.project_path],
        )?;

        Ok(affected)
    }

    /// 清理低分记忆（软删除）
    pub fn cleanup_low_score_memories(&self, threshold: f32) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let affected = conn.execute(
            "UPDATE change_memories 
             SET is_deleted = 1 
             WHERE project_path = ?1 AND relevance_score < ?2 AND is_deleted = 0",
            params![self.project_path, threshold as f64],
        )?;

        Ok(affected)
    }

    /// 从数据库行构建 CodeChangeMemory
    fn row_to_change_memory(&self, row: &rusqlite::Row) -> CodeChangeMemory {
        let file_paths: Vec<String> = serde_json::from_str(row.get::<_, String>(2).unwrap_or_default().as_str())
            .unwrap_or_default();
        let symbols: Vec<String> = serde_json::from_str(row.get::<_, String>(3).unwrap_or_default().as_str())
            .unwrap_or_default();
        let keywords: Vec<String> = serde_json::from_str(row.get::<_, String>(7).unwrap_or_default().as_str())
            .unwrap_or_default();
        
        let created_at_ts: i64 = row.get(8).unwrap_or(0);
        let last_recalled_ts: Option<i64> = row.get(9).ok();

        CodeChangeMemory {
            id: row.get(0).unwrap_or_default(),
            change_type: Self::str_to_change_type(&row.get::<_, String>(1).unwrap_or_default()),
            file_paths,
            symbols,
            summary: row.get(4).unwrap_or_default(),
            diff_snippet: row.get(5).ok(),
            user_intent: row.get(6).unwrap_or_default(),
            keywords,
            created_at: DateTime::from_timestamp(created_at_ts, 0)
                .unwrap_or_else(|| Utc::now()),
            last_recalled: last_recalled_ts.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            recall_count: row.get(10).unwrap_or(0),
            relevance_score: row.get(11).unwrap_or(1.0),
        }
    }

    // ========================================================================
    // 向量存取方法
    // ========================================================================

    /// 保存记忆的向量
    pub fn save_embedding(&self, memory_id: &str, embedding: &[f32], model: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let blob = Self::vector_to_bytes(embedding);
        
        conn.execute(
            "UPDATE change_memories SET summary_embedding = ?1, embedding_model = ?2 WHERE id = ?3",
            params![blob, model, memory_id],
        )?;
        
        Ok(())
    }

    /// 获取记忆的向量
    pub fn get_embedding(&self, memory_id: &str) -> Result<Option<(Vec<f32>, String)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let result: Option<(Vec<u8>, String)> = conn.query_row(
            "SELECT summary_embedding, embedding_model FROM change_memories WHERE id = ?1 AND summary_embedding IS NOT NULL",
            params![memory_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).ok();
        
        if let Some((blob, model)) = result {
            let embedding = Self::bytes_to_vector(&blob);
            return Ok(Some((embedding, model)));
        }
        
        Ok(None)
    }

    /// 获取所有带向量的记忆 ID
    pub fn get_memories_with_embedding(&self) -> Result<Vec<(String, Vec<f32>)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, summary_embedding FROM change_memories 
             WHERE project_path = ?1 AND summary_embedding IS NOT NULL AND is_deleted = 0"
        )?;
        
        let rows = stmt.query_map(params![self.project_path], |row| {
            let id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((id, blob))
        })?;
        
        let mut results = Vec::new();
        for row in rows {
            if let Ok((id, blob)) = row {
                let embedding = Self::bytes_to_vector(&blob);
                results.push((id, embedding));
            }
        }
        
        Ok(results)
    }

    /// 获取没有向量的记忆
    pub fn get_memories_without_embedding(&self) -> Result<Vec<CodeChangeMemory>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, change_type, file_paths, symbols, summary, diff_snippet, user_intent, keywords,
                    created_at, last_recalled, recall_count, relevance_score
             FROM change_memories 
             WHERE project_path = ?1 AND summary_embedding IS NULL AND is_deleted = 0"
        )?;
        
        let rows = stmt.query_map(params![self.project_path], |row| {
            Ok(self.row_to_change_memory(row))
        })?;
        
        let mut results = Vec::new();
        for row in rows {
            if let Ok(memory) = row {
                results.push(memory);
            }
        }
        
        Ok(results)
    }

    /// 将向量转换为字节
    fn vector_to_bytes(vector: &[f32]) -> Vec<u8> {
        vector.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect()
    }

    /// 将字节转换为向量
    fn bytes_to_vector(bytes: &[u8]) -> Vec<f32> {
        bytes.chunks_exact(4)
            .map(|chunk| {
                let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                f32::from_le_bytes(arr)
            })
            .collect()
    }
}
