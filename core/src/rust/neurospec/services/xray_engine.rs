use anyhow::Result;
use ignore::WalkBuilder;
use log::{debug, warn};
use rayon::prelude::*;
use std::fs;
use std::path::Path;

use crate::neurospec::models::{Symbol, SymbolKind, XRaySnapshot};
use crate::neurospec::services::analyzer;

/// 项目扫描（X-Ray）服务
///
/// MVP实现：递归遍历项目目录，生成按文件粒度的Symbol列表

/// Configuration for project scanning
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// Maximum number of files to scan
    pub max_files: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self { max_files: 10000 }
    }
}

/// 扫描项目目录，返回XRaySnapshot
pub fn scan_project<P: AsRef<Path>>(
    project_root: P,
    config: Option<ScanConfig>,
) -> Result<XRaySnapshot> {
    let root = project_root.as_ref();
    let root_path = root.canonicalize().map_err(|e| {
        anyhow::anyhow!("Failed to resolve project root '{}': {}", root.display(), e)
    })?;

    let config = config.unwrap_or_default();

    // 使用 ignore crate 来处理 .gitignore 等忽略规则
    let walker = WalkBuilder::new(&root_path)
        .hidden(false) // 不自动跳过隐藏文件（让.gitignore控制）
        .git_ignore(true) // 遵守.gitignore
        .git_global(true) // 遵守全局.gitignore
        .git_exclude(true) // 遵守.git/info/exclude
        .build();

    // Collect all file entries first
    let file_entries: Vec<_> = walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .take(config.max_files)
        .collect();

    debug!(
        "Collected {} files for scanning (limit: {})",
        file_entries.len(),
        config.max_files
    );

    if file_entries.is_empty() {
        warn!("No files found in project root: {}", root_path.display());
    }

    // Process files in parallel using rayon
    let symbols: Vec<Symbol> = file_entries
        .par_iter()
        .flat_map(|entry| {
            let path = entry.path();

            let rel_path = match path.strip_prefix(&root_path) {
                Ok(p) => p.to_string_lossy().replace("\\", "/"), // POSIX风格
                Err(_) => return Vec::new(),
            };

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let language = guess_language(path);

            // Try to use AST analyzer for supported languages
            let mut file_symbols = Vec::new();
            if let Some(ref lang) = language {
                if lang == "rust"
                    || lang == "typescript"
                    || lang == "javascript"
                    || lang == "python"
                {
                    // Read file content for AST analysis
                    match fs::read_to_string(path) {
                        Ok(content) => {
                            // Catch panics during AST analysis to prevent server crash
                            let result = std::panic::catch_unwind(|| {
                                analyzer::analyze_file_thread_local(
                                    Path::new(&rel_path),
                                    &content,
                                    lang,
                                )
                            });

                            match result {
                                Ok(symbols) => {
                                    file_symbols = symbols;
                                    debug!(
                                        "AST analysis found {} symbols in {}",
                                        file_symbols.len(),
                                        rel_path
                                    );
                                }
                                Err(_) => {
                                    warn!("AST analyzer panicked for file: {}", rel_path);
                                    // Fallback to file-level symbol will happen below
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read file {} for AST analysis: {}", rel_path, e);
                        }
                    }
                }
            }

            // If no symbols extracted from AST (unsupported language or failed), create file-level symbol
            if file_symbols.is_empty() {
                let symbol = Symbol {
                    kind: SymbolKind::File,
                    name,
                    path: rel_path,
                    language,
                    signature: None,
                    references: Vec::new(),
                };
                vec![symbol]
            } else {
                // Use AST-extracted symbols
                file_symbols
            }
        })
        .collect();

    debug!("Total symbols extracted: {}", symbols.len());

    // 计算置信度
    let total_files = file_entries.len();
    let symbol_files = symbols.iter().filter(|s| matches!(s.kind, SymbolKind::File)).count();
    let confidence = if total_files > 0 {
        (symbol_files as f32 / total_files as f32).min(1.0)
    } else {
        1.0
    };

    let mut warnings = Vec::new();
    if total_files >= config.max_files {
        warnings.push(format!("Scan truncated at {} files limit", config.max_files));
    }

    Ok(XRaySnapshot {
        project_root: root_path.to_string_lossy().to_string(),
        symbols,
        confidence,
        warnings,
        skipped_files: 0,
        failed_files: 0,
    })
}

/// Guess programming language from file path
fn guess_language(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext.to_lowercase().as_str() {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" => "python",
            "go" => "go",
            "c" | "h" => "c",
            "cpp" | "hpp" | "cc" => "cpp",
            "java" => "java",
            "md" => "markdown",
            "json" => "json",
            "toml" => "toml",
            "yaml" | "yml" => "yaml",
            "html" => "html",
            "css" => "css",
            "sh" => "shell",
            _ => "unknown",
        })
        .map(|s| s.to_string())
        .filter(|s| s != "unknown")
}


/// 使用统一缓存扫描项目（增量模式）
///
/// 相比 scan_project，此方法：
/// - 使用 UnifiedSymbolStore 作为数据源
/// - 支持增量更新（只扫描变化的文件）
/// - 首次调用后，后续调用速度大幅提升
pub fn scan_project_cached<P: AsRef<Path>>(
    project_root: P,
    store: &crate::mcp::tools::unified_store::UnifiedSymbolStore,
) -> Result<XRaySnapshot> {
    let root = project_root.as_ref();
    let root_path = root.canonicalize().map_err(|e| {
        anyhow::anyhow!("Failed to resolve project root '{}': {}", root.display(), e)
    })?;

    // 触发增量索引
    let stats = store.index_project(&root_path)?;
    debug!(
        "Incremental index: {} indexed, {} skipped",
        stats.indexed, stats.skipped
    );

    // 从缓存获取符号
    let unified_symbols = store.get_project_symbols(&root_path)?;

    // 转换为 X-Ray Symbol 格式
    let symbols: Vec<Symbol> = unified_symbols
        .into_iter()
        .map(|us| Symbol {
            kind: match us.kind {
                crate::mcp::tools::unified_store::store::SymbolKind::File => SymbolKind::File,
                crate::mcp::tools::unified_store::store::SymbolKind::Module => SymbolKind::Module,
                crate::mcp::tools::unified_store::store::SymbolKind::Class => SymbolKind::Class,
                crate::mcp::tools::unified_store::store::SymbolKind::Function => SymbolKind::Function,
                crate::mcp::tools::unified_store::store::SymbolKind::Variable => SymbolKind::Function, // fallback
            },
            name: us.name,
            path: us.path,
            language: us.language,
            signature: us.signature,
            references: us.references,
        })
        .collect();

    debug!("Loaded {} symbols from cache", symbols.len());

    // 计算置信度（基于增量索引统计）
    let total = stats.indexed + stats.skipped;
    let confidence = if total > 0 { 1.0 } else { 0.5 };

    Ok(XRaySnapshot {
        project_root: root_path.to_string_lossy().to_string(),
        symbols,
        confidence,
        warnings: Vec::new(),
        skipped_files: stats.skipped,
        failed_files: 0,
    })
}
