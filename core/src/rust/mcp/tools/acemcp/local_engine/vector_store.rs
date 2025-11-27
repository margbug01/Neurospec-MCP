//! 代码向量存储
//!
//! 存储代码文件的嵌入向量，用于语义搜索

use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;

/// 代码向量条目
#[derive(Debug, Clone)]
pub struct CodeVectorEntry {
    pub file_path: String,
    pub symbols: Vec<String>,
    pub summary: String,
    pub embedding: Vec<f32>,
    pub updated_at: i64,
}

/// 代码向量存储
pub struct CodeVectorStore {
    conn: Mutex<Connection>,
}

impl CodeVectorStore {
    /// 创建新的向量存储
    pub fn new(project_root: &PathBuf) -> Result<Self> {
        let store_dir = project_root.join(".neurospec");
        std::fs::create_dir_all(&store_dir)?;
        
        let db_path = store_dir.join("code_vectors.db");
        let conn = Connection::open(&db_path)?;
        
        Self::initialize_schema(&conn)?;
        
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 初始化数据库 schema
    fn initialize_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS code_vectors (
                file_path TEXT PRIMARY KEY,
                symbols TEXT NOT NULL,
                summary TEXT NOT NULL,
                embedding BLOB,
                dimension INTEGER DEFAULT 0,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_vectors_updated ON code_vectors(updated_at)",
            [],
        )?;
        
        Ok(())
    }

    /// 保存代码向量
    pub fn save(&self, entry: &CodeVectorEntry) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let symbols_json = serde_json::to_string(&entry.symbols)?;
        let embedding_blob = Self::vector_to_bytes(&entry.embedding);
        
        conn.execute(
            "INSERT OR REPLACE INTO code_vectors (file_path, symbols, summary, embedding, dimension, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.file_path,
                symbols_json,
                entry.summary,
                embedding_blob,
                entry.embedding.len() as i64,
                entry.updated_at
            ],
        )?;
        
        Ok(())
    }

    /// 批量保存
    pub fn save_batch(&self, entries: &[CodeVectorEntry]) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let mut count = 0;
        for entry in entries {
            let symbols_json = serde_json::to_string(&entry.symbols)?;
            let embedding_blob = Self::vector_to_bytes(&entry.embedding);
            
            conn.execute(
                "INSERT OR REPLACE INTO code_vectors (file_path, symbols, summary, embedding, dimension, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    entry.file_path,
                    symbols_json,
                    entry.summary,
                    embedding_blob,
                    entry.embedding.len() as i64,
                    entry.updated_at
                ],
            )?;
            count += 1;
        }
        
        Ok(count)
    }

    /// 获取代码向量
    pub fn get(&self, file_path: &str) -> Result<Option<CodeVectorEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let result = conn.query_row(
            "SELECT file_path, symbols, summary, embedding, dimension, updated_at FROM code_vectors WHERE file_path = ?1",
            params![file_path],
            |row| {
                let symbols_json: String = row.get(1)?;
                let blob: Vec<u8> = row.get(3)?;
                let dim: i64 = row.get(4)?;
                
                Ok((
                    row.get::<_, String>(0)?,
                    symbols_json,
                    row.get::<_, String>(2)?,
                    blob,
                    dim,
                    row.get::<_, i64>(5)?,
                ))
            },
        );

        match result {
            Ok((file_path, symbols_json, summary, blob, dim, updated_at)) => {
                let symbols: Vec<String> = serde_json::from_str(&symbols_json).unwrap_or_default();
                let embedding = Self::bytes_to_vector(&blob, dim as usize);
                
                Ok(Some(CodeVectorEntry {
                    file_path,
                    symbols,
                    summary,
                    embedding,
                    updated_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 获取所有有向量的条目
    pub fn get_all_with_vectors(&self) -> Result<Vec<CodeVectorEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT file_path, symbols, summary, embedding, dimension, updated_at 
             FROM code_vectors 
             WHERE embedding IS NOT NULL AND dimension > 0"
        )?;
        
        let rows = stmt.query_map([], |row| {
            let symbols_json: String = row.get(1)?;
            let blob: Vec<u8> = row.get(3)?;
            let dim: i64 = row.get(4)?;
            
            Ok((
                row.get::<_, String>(0)?,
                symbols_json,
                row.get::<_, String>(2)?,
                blob,
                dim,
                row.get::<_, i64>(5)?,
            ))
        })?;
        
        let mut entries = Vec::new();
        for row in rows {
            if let Ok((file_path, symbols_json, summary, blob, dim, updated_at)) = row {
                let symbols: Vec<String> = serde_json::from_str(&symbols_json).unwrap_or_default();
                let embedding = Self::bytes_to_vector(&blob, dim as usize);
                
                entries.push(CodeVectorEntry {
                    file_path,
                    symbols,
                    summary,
                    embedding,
                    updated_at,
                });
            }
        }
        
        Ok(entries)
    }

    /// 获取需要计算向量的文件
    pub fn get_files_without_vectors(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT file_path FROM code_vectors WHERE embedding IS NULL OR dimension = 0"
        )?;
        
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        
        let mut paths = Vec::new();
        for row in rows {
            if let Ok(path) = row {
                paths.push(path);
            }
        }
        
        Ok(paths)
    }

    /// 更新文件的向量
    pub fn update_embedding(&self, file_path: &str, embedding: &[f32]) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let blob = Self::vector_to_bytes(embedding);
        let now = chrono::Utc::now().timestamp();
        
        conn.execute(
            "UPDATE code_vectors SET embedding = ?1, dimension = ?2, updated_at = ?3 WHERE file_path = ?4",
            params![blob, embedding.len() as i64, now, file_path],
        )?;
        
        Ok(())
    }

    /// 删除文件的记录
    pub fn delete(&self, file_path: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        conn.execute("DELETE FROM code_vectors WHERE file_path = ?1", params![file_path])?;
        
        Ok(())
    }

    /// 清空所有记录
    pub fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        conn.execute("DELETE FROM code_vectors", [])?;
        
        Ok(())
    }

    /// 获取统计信息
    pub fn stats(&self) -> Result<VectorStoreStats> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM code_vectors", [], |row| row.get(0))?;
        let with_vectors: i64 = conn.query_row(
            "SELECT COUNT(*) FROM code_vectors WHERE embedding IS NOT NULL AND dimension > 0", 
            [], 
            |row| row.get(0)
        )?;
        
        Ok(VectorStoreStats {
            total_files: total as usize,
            files_with_vectors: with_vectors as usize,
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

/// 向量存储统计
#[derive(Debug)]
pub struct VectorStoreStats {
    pub total_files: usize,
    pub files_with_vectors: usize,
}
