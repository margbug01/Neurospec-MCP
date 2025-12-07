use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::Result;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use tantivy::schema::*;
use tantivy::{Document, Index, IndexWriter, Term};

use super::extractor;
use super::types::LocalEngineConfig;
use super::vector_store::{CodeVectorStore, CodeVectorEntry};

/// 文件元数据缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileMetadata {
    mtime: u64,
    size: u64,
}

/// 索引元数据
#[derive(Debug, Default, Serialize, Deserialize)]
struct IndexMetadata {
    /// 项目根路径 -> 文件路径 -> 元数据
    projects: HashMap<String, HashMap<String, FileMetadata>>,
}

/// Snippet 最大长度（字符）
const MAX_SNIPPET_LENGTH: usize = 500;

pub struct LocalIndexer {
    #[allow(dead_code)] // 保留用于未来查询优化
    index: Index,
    writer: IndexWriter,
    config: LocalEngineConfig,
    // Field handles
    field_path: Field,
    field_content: Field,
    field_symbols: Field,
    field_language: Field,
    field_snippet: Field,
}

impl LocalIndexer {
    pub fn new(config: &LocalEngineConfig) -> Result<Self> {
        // 1. Define Schema
        let mut schema_builder = Schema::builder();

        let field_path = schema_builder.add_text_field("path", TEXT | STORED);
        let field_content = schema_builder.add_text_field("content", TEXT);
        let field_symbols = schema_builder.add_text_field("symbols", TEXT | STORED);
        let field_language = schema_builder.add_text_field("language", STRING);
        let field_snippet = schema_builder.add_text_field("snippet", STORED);  // 预存 snippet

        let schema = schema_builder.build();

        // 2. Open or Create Index
        fs::create_dir_all(&config.index_path)?;
        let dir = tantivy::directory::MmapDirectory::open(&config.index_path)?;
        let index = Index::open_or_create(dir, schema)?;

        // 3. Create Writer (heap size 50MB)
        let writer = index.writer(50_000_000)?;

        Ok(Self {
            index,
            writer,
            config: config.clone(),
            field_path,
            field_content,
            field_symbols,
            field_language,
            field_snippet,
        })
    }

    /// 获取元数据文件路径
    fn metadata_path(&self) -> std::path::PathBuf {
        self.config.index_path.join("index_metadata.json")
    }

    /// 加载索引元数据
    fn load_metadata(&self) -> IndexMetadata {
        let path = self.metadata_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(meta) = serde_json::from_str(&data) {
                    return meta;
                }
            }
        }
        IndexMetadata::default()
    }

    /// 保存索引元数据
    fn save_metadata(&self, metadata: &IndexMetadata) -> Result<()> {
        let path = self.metadata_path();
        let data = serde_json::to_string_pretty(metadata)?;
        fs::write(path, data)?;
        Ok(())
    }

    /// 检查文件是否需要重新索引
    fn should_reindex(
        &self,
        path: &Path,
        cached: Option<&FileMetadata>,
    ) -> Option<FileMetadata> {
        let metadata = fs::metadata(path).ok()?;
        let mtime = metadata
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();
        let size = metadata.len();

        let current = FileMetadata { mtime, size };

        match cached {
            Some(cached) if cached.mtime == mtime && cached.size == size => None,
            _ => Some(current),
        }
    }

    pub fn rebuild_index(&mut self, root: &Path) -> Result<usize> {
        self.writer.delete_all_documents()?;
        
        // 清除该项目的元数据缓存
        let mut metadata = self.load_metadata();
        let root_key = root.to_string_lossy().to_string();
        metadata.projects.remove(&root_key);
        self.save_metadata(&metadata)?;
        
        self.index_directory(root)
    }

    /// 增量索引目录
    pub fn index_directory(&mut self, root: &Path) -> Result<usize> {
        let root_key = root.to_string_lossy().to_string();
        
        crate::log_important!(info, "Starting index for: {}", root_key);
        crate::log_important!(info, "Index path: {:?}", self.config.index_path);
        
        let mut metadata = self.load_metadata();
        let project_cache = metadata.projects.entry(root_key.clone()).or_default();

        let mut indexed_count = 0;
        let mut skipped_count = 0;
        let mut current_files: HashMap<String, FileMetadata> = HashMap::new();
        let mut total_walked = 0;

        // 使用 ignore crate 遵守 .gitignore 规则
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();
        
        for entry in walker.filter_map(|e| e.ok()) {
            total_walked += 1;
            
            // ignore::DirEntry 的 file_type() 返回 Option<FileType>
            let is_file = entry.file_type()
                .map(|t| t.is_file())
                .unwrap_or(false);
            
            if !is_file {
                continue;
            }

            let path = entry.path();
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            // 检查是否需要重新索引
            let cached = project_cache.get(&rel_path);
            match self.should_reindex(path, cached) {
                Some(new_meta) => {
                    // 需要重新索引：先删除旧文档
                    let term = Term::from_field_text(self.field_path, &rel_path);
                    self.writer.delete_term(term);

                    // 索引新内容
                    if let Err(e) = self.index_file(path, root) {
                        crate::log_important!(error, "Failed to index file {:?}: {}", path, e);
                    } else {
                        indexed_count += 1;
                        current_files.insert(rel_path.clone(), new_meta);
                        
                        // 每 100 个文件输出一次进度
                        if indexed_count % 100 == 0 {
                            crate::log_important!(info, "Indexed {} files...", indexed_count);
                        }
                    }
                }
                None => {
                    // 文件未变化，跳过
                    skipped_count += 1;
                    if let Some(meta) = cached {
                        current_files.insert(rel_path, meta.clone());
                    }
                }
            }
        }

        // 更新元数据缓存
        let total_files = current_files.len();
        metadata.projects.insert(root_key, current_files);
        self.save_metadata(&metadata)?;

        self.commit()?;
        crate::log_important!(
            info,
            "Index complete: {} indexed, {} skipped (unchanged), {} total files, {} entries walked",
            indexed_count,
            skipped_count,
            total_files,
            total_walked
        );

        // 异步更新向量存储（仅在有 Tokio runtime 时执行）
        if indexed_count > 0 {
            let root_path = root.to_path_buf();
            // 使用 try_current() 检测是否在 Tokio runtime 上下文中
            // 避免在 std::thread::spawn 的后台线程中调用 tokio::spawn 导致 panic
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    if let Err(e) = Self::update_vector_store(&root_path).await {
                        crate::log_important!(warn, "Failed to update vector store: {}", e);
                    }
                });
            } else {
                crate::log_important!(info, "Skipping vector store update (no async runtime available)");
            }
        }

        // 返回总文件数（而非本次新索引数），用于正确显示索引状态
        Ok(total_files)
    }

    /// 异步更新向量存储
    async fn update_vector_store(root: &PathBuf) -> Result<()> {
        use crate::neurospec::services::embedding::{is_embedding_available, get_global_embedding_service};
        
        // 检查嵌入服务是否可用
        if !is_embedding_available() {
            crate::log_important!(info, "Embedding service not available, skipping vector store update");
            return Ok(());
        }

        // 创建向量存储
        let store = CodeVectorStore::new(root)?;
        
        // 遍历所有代码文件（遵守 .gitignore）
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();
        let mut entries_to_update = Vec::new();
        
        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }

            let path = entry.path();
            
            // 只处理代码文件
            if !is_code_file(path) {
                continue;
            }

            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            // 检查是否已有向量
            if let Ok(Some(_)) = store.get(&rel_path) {
                continue; // 已有向量，跳过
            }

            // 读取文件并提取符号
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(symbols) = super::extractor::extract_symbols(path, &content) {
                    let symbol_names: Vec<String> = symbols.iter().map(|s| s.name.clone()).collect();
                    let summary = generate_file_summary(path, &symbol_names);
                    
                    entries_to_update.push(CodeVectorEntry {
                        file_path: rel_path,
                        symbols: symbol_names,
                        summary,
                        embedding: vec![], // 稍后填充
                        updated_at: chrono::Utc::now().timestamp(),
                    });
                }
            }
        }

        if entries_to_update.is_empty() {
            return Ok(());
        }

        crate::log_important!(info, "Updating vector store: {} files to embed", entries_to_update.len());

        // 获取嵌入服务
        let lock = match get_global_embedding_service() {
            Some(l) => l,
            None => return Ok(()),
        };

        // 批量计算嵌入（每次最多 10 个）
        for chunk in entries_to_update.chunks(10) {
            let texts: Vec<String> = chunk.iter()
                .map(|e| format!("{} {}", e.summary, e.symbols.join(" ")))
                .collect();

            // 获取锁并计算嵌入
            let embeddings = {
                let guard = lock.read().await;
                if let Some(service) = guard.as_ref() {
                    service.embed_batch(&texts).await.ok()
                } else {
                    None
                }
            };

            if let Some(embeddings) = embeddings {
                for (entry, embedding) in chunk.iter().zip(embeddings.into_iter()) {
                    let mut updated_entry = entry.clone();
                    updated_entry.embedding = embedding;
                    let _ = store.save(&updated_entry);
                }
            }
        }

        let stats = store.stats()?;
        crate::log_important!(info, "Vector store updated: {}/{} files have embeddings", 
            stats.files_with_vectors, stats.total_files);

        Ok(())
    }

    pub fn index_file(&mut self, path: &Path, root: &Path) -> Result<()> {
        // Read content
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Ok(()), // Skip non-utf8 or unreadable files
        };

        // Extract symbols
        let symbols = extractor::extract_symbols(path, &content)?;
        let symbol_text = symbols
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Detect Language
        let lang_str = format!("{:?}", extractor::detect_language(path));

        // Generate preview snippet (first N characters with line numbers)
        let snippet = Self::generate_preview_snippet(&content);

        // Create Document
        let mut doc = Document::default();
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        doc.add_text(self.field_path, &rel_path);
        doc.add_text(self.field_content, &content);
        doc.add_text(self.field_symbols, &symbol_text);
        doc.add_text(self.field_language, &lang_str);
        doc.add_text(self.field_snippet, &snippet);

        self.writer.add_document(doc)?;
        Ok(())
    }

    /// 生成预览 snippet（跳过 imports，返回有意义的代码）
    fn generate_preview_snippet(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        
        // 查找有意义的起始位置（跳过 imports 和注释）
        let mut start_idx = 0;
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty() 
                && !trimmed.starts_with("use ")
                && !trimmed.starts_with("import ")
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with("*")
                && !trimmed.starts_with("#")
            {
                start_idx = i;
                break;
            }
        }
        
        let mut result = String::new();
        let mut char_count = 0;
        
        for (i, line) in lines.iter().enumerate().skip(start_idx) {
            if char_count >= MAX_SNIPPET_LENGTH {
                result.push_str(&format!("  ... (truncated)\n"));
                break;
            }
            
            let line_num = i + 1;
            let line_text = if line.chars().count() > 100 {
                let truncated: String = line.chars().take(100).collect();
                format!("{}...", truncated)
            } else {
                line.to_string()
            };
            
            result.push_str(&format!("  {:4} | {}\n", line_num, line_text));
            char_count += line.len();
        }
        
        result
    }

    pub fn commit(&mut self) -> Result<()> {
        self.writer.commit()?;
        Ok(())
    }

    /// 获取索引统计信息
    pub fn get_stats(&self, root: &Path) -> Result<IndexStats> {
        let metadata = self.load_metadata();
        let root_key = root.to_string_lossy().to_string();
        
        let project_files = metadata.projects.get(&root_key);
        let indexed_count = project_files.map(|m| m.len()).unwrap_or(0);
        
        Ok(IndexStats {
            indexed_files: indexed_count,
            index_path: self.config.index_path.clone(),
            last_updated: project_files
                .and_then(|m| m.values().map(|v| v.mtime).max()),
        })
    }
}

/// 索引统计信息
#[derive(Debug)]
pub struct IndexStats {
    pub indexed_files: usize,
    pub index_path: PathBuf,
    pub last_updated: Option<u64>,
}

#[allow(dead_code)]
fn is_ignored(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| {
            s.starts_with('.')
                || s == "target"
                || s == "node_modules"
                || s == "dist"
                || s == "vendor"
                || s == "build"
                || s == "__pycache__"
        })
        .unwrap_or(false)
}

/// 检查是否为代码文件
fn is_code_file(path: &Path) -> bool {
    let extensions = ["rs", "ts", "tsx", "js", "jsx", "vue", "py", "go", "java", "cpp", "c", "h", "hpp"];
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.contains(&ext))
        .unwrap_or(false)
}

/// 生成文件摘要
fn generate_file_summary(path: &Path, symbols: &[String]) -> String {
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    let parent = path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    let top_symbols = symbols.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    
    format!("{}/{} contains: {}", parent, file_name, top_symbols)
}
