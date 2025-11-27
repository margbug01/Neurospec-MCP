//! 嵌入向量缓存

use anyhow::Result;
use rusqlite::{Connection, params};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;

/// 嵌入向量缓存
/// 
/// 使用 SQLite 持久化缓存，避免重复 API 调用
pub struct EmbeddingCache {
    conn: Mutex<Connection>,
}

impl EmbeddingCache {
    /// 创建新的缓存
    pub fn new(cache_path: &PathBuf) -> Result<Self> {
        std::fs::create_dir_all(cache_path)?;
        
        let db_path = cache_path.join("embeddings.db");
        let conn = Connection::open(&db_path)?;
        
        // 初始化表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                text_hash TEXT PRIMARY KEY,
                vector BLOB NOT NULL,
                dimension INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        
        // 创建索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_embeddings_created ON embeddings(created_at)",
            [],
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 计算文本的 hash
    fn hash_text(text: &str) -> String {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// 获取缓存的嵌入向量
    pub fn get(&self, text: &str) -> Result<Option<Vec<f32>>> {
        let hash = Self::hash_text(text);
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let result: Option<(Vec<u8>, i64)> = conn.query_row(
            "SELECT vector, dimension FROM embeddings WHERE text_hash = ?1",
            params![hash],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).ok();

        if let Some((blob, dimension)) = result {
            let vector = Self::bytes_to_vector(&blob, dimension as usize);
            return Ok(Some(vector));
        }

        Ok(None)
    }

    /// 存入缓存
    pub fn set(&self, text: &str, vector: &[f32]) -> Result<()> {
        let hash = Self::hash_text(text);
        let blob = Self::vector_to_bytes(vector);
        let now = chrono::Utc::now().timestamp();
        
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        conn.execute(
            "INSERT OR REPLACE INTO embeddings (text_hash, vector, dimension, created_at) 
             VALUES (?1, ?2, ?3, ?4)",
            params![hash, blob, vector.len() as i64, now],
        )?;

        Ok(())
    }

    /// 清理过期缓存
    /// 
    /// 删除超过 `days` 天的缓存
    pub fn cleanup(&self, days: i64) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let cutoff = chrono::Utc::now().timestamp() - (days * 24 * 60 * 60);
        
        let deleted = conn.execute(
            "DELETE FROM embeddings WHERE created_at < ?1",
            params![cutoff],
        )?;

        Ok(deleted)
    }

    /// 获取缓存统计
    pub fn stats(&self) -> Result<CacheStats> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM embeddings",
            [],
            |row| row.get(0),
        )?;

        let size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(vector)), 0) FROM embeddings",
            [],
            |row| row.get(0),
        )?;

        Ok(CacheStats {
            entry_count: count as usize,
            total_bytes: size as usize,
        })
    }

    /// 将向量转换为字节
    fn vector_to_bytes(vector: &[f32]) -> Vec<u8> {
        vector.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect()
    }

    /// 将字节转换为向量
    fn bytes_to_vector(bytes: &[u8], dimension: usize) -> Vec<f32> {
        bytes.chunks_exact(4)
            .take(dimension)
            .map(|chunk| {
                let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                f32::from_le_bytes(arr)
            })
            .collect()
    }
}

/// 缓存统计
#[derive(Debug)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_bytes: usize,
}
