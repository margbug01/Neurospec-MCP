//! 统一符号存储
//!
//! 提供符号缓存、增量更新、多消费者接口

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::UNIX_EPOCH;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 符号类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolKind {
    File,
    Module,
    Class,
    Function,
    Variable,
}

/// 统一符号结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSymbol {
    pub kind: SymbolKind,
    pub name: String,
    pub path: String,
    pub language: Option<String>,
    pub signature: Option<String>,
    pub references: Vec<String>,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
}

/// 文件缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileCacheEntry {
    mtime: u64,
    size: u64,
    symbols: Vec<UnifiedSymbol>,
}

/// 项目缓存
#[derive(Debug, Default, Serialize, Deserialize)]
struct ProjectCache {
    files: HashMap<String, FileCacheEntry>,
    last_full_scan: Option<u64>,
}

/// 统一符号存储
pub struct UnifiedSymbolStore {
    /// 项目根路径 -> 项目缓存
    projects: Arc<RwLock<HashMap<String, ProjectCache>>>,
    /// 缓存文件路径
    cache_path: PathBuf,
}


impl UnifiedSymbolStore {
    /// 创建新的统一存储
    pub fn new(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir)?;
        let cache_path = cache_dir.join("unified_symbols.json");
        
        let projects = if cache_path.exists() {
            let data = std::fs::read_to_string(&cache_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(Self {
            projects: Arc::new(RwLock::new(projects)),
            cache_path,
        })
    }

    /// 获取或创建项目缓存
    pub fn get_project_symbols(&self, project_root: &Path) -> Result<Vec<UnifiedSymbol>> {
        let root_key = project_root.to_string_lossy().to_string();
        let projects = self.projects.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        
        if let Some(cache) = projects.get(&root_key) {
            let symbols: Vec<UnifiedSymbol> = cache.files
                .values()
                .flat_map(|entry| entry.symbols.clone())
                .collect();
            return Ok(symbols);
        }
        
        Ok(Vec::new())
    }

    /// 检查文件是否需要重新索引
    fn should_reindex(&self, path: &Path, cached: Option<&FileCacheEntry>) -> Option<(u64, u64)> {
        let metadata = std::fs::metadata(path).ok()?;
        let mtime = metadata
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();
        let size = metadata.len();

        match cached {
            Some(c) if c.mtime == mtime && c.size == size => None,
            _ => Some((mtime, size)),
        }
    }

    /// 增量索引项目
    pub fn index_project(&self, project_root: &Path) -> Result<IndexStats> {
        let root_key = project_root.to_string_lossy().to_string();
        let mut stats = IndexStats::default();

        // 获取当前缓存
        let mut projects = self.projects.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        let cache = projects.entry(root_key.clone()).or_default();

        // 遍历文件
        for entry in walkdir::WalkDir::new(project_root)
            .into_iter()
            .filter_entry(|e| !is_ignored(e))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let rel_path = path
                .strip_prefix(project_root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            let cached = cache.files.get(&rel_path);
            
            if let Some((mtime, size)) = self.should_reindex(path, cached) {
                // 需要重新索引
                if let Ok(symbols) = extract_symbols_from_file(path) {
                    cache.files.insert(rel_path, FileCacheEntry {
                        mtime,
                        size,
                        symbols,
                    });
                    stats.indexed += 1;
                }
            } else {
                stats.skipped += 1;
            }
        }

        // 保存缓存
        drop(projects);
        self.save_cache()?;

        Ok(stats)
    }

    /// 使单个文件失效
    pub fn invalidate_file(&self, project_root: &Path, rel_path: &str) -> Result<()> {
        let root_key = project_root.to_string_lossy().to_string();
        let mut projects = self.projects.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        
        if let Some(cache) = projects.get_mut(&root_key) {
            cache.files.remove(rel_path);
        }
        
        Ok(())
    }

    /// 保存缓存到磁盘
    fn save_cache(&self) -> Result<()> {
        let projects = self.projects.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let data = serde_json::to_string_pretty(&*projects)?;
        std::fs::write(&self.cache_path, data)?;
        Ok(())
    }
}

/// 索引统计
#[derive(Debug, Default)]
pub struct IndexStats {
    pub indexed: usize,
    pub skipped: usize,
}

/// 从文件提取符号（使用 AST 分析）
fn extract_symbols_from_file(path: &Path) -> Result<Vec<UnifiedSymbol>> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let language = match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        _ => return Ok(vec![]),
    };

    let content = std::fs::read_to_string(path)?;
    let rel_path = path.to_string_lossy().replace('\\', "/");

    // 使用 AST 分析器提取符号
    let ast_symbols = std::panic::catch_unwind(|| {
        crate::neurospec::services::analyzer::analyze_file_thread_local(path, &content, language)
    });

    match ast_symbols {
        Ok(symbols) if !symbols.is_empty() => {
            // 转换为 UnifiedSymbol 格式
            Ok(symbols.into_iter().map(|s| UnifiedSymbol {
                kind: match s.kind {
                    crate::neurospec::models::SymbolKind::File => SymbolKind::File,
                    crate::neurospec::models::SymbolKind::Module => SymbolKind::Module,
                    crate::neurospec::models::SymbolKind::Class => SymbolKind::Class,
                    crate::neurospec::models::SymbolKind::Function => SymbolKind::Function,
                },
                name: s.name,
                path: s.path,
                language: s.language,
                signature: s.signature,
                references: s.references,
                start_line: None,
                end_line: None,
            }).collect())
        }
        _ => {
            // AST 分析失败或无符号，回退到文件级符号
            Ok(vec![UnifiedSymbol {
                kind: SymbolKind::File,
                name: path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                path: rel_path,
                language: Some(language.to_string()),
                signature: None,
                references: Vec::new(),
                start_line: Some(1),
                end_line: Some(content.lines().count() as u32),
            }])
        }
    }
}

/// 检查是否应该忽略
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
