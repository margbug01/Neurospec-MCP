use anyhow::Result;
use rmcp::model::*;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use super::types::{SearchRequest, SearchMode};
use super::local_engine::{LocalIndexer, LocalEngineConfig, RipgrepSearcher, CtagsIndexer};
use crate::log_important;
use crate::mcp::utils::errors::McpToolError;
use crate::mcp::tools::memory::{ChangeTracker, CodeChangeMemory};
use crate::mcp::tools::unified_store::{
    create_searcher_for_project, is_search_initialized,
    is_project_indexed, is_project_indexing, mark_indexing_started, mark_indexing_complete,
    get_index_state,
};

// ============================================================================
// Structure Mode: Project Insight 相关类型和辅助函数
// ============================================================================

/// 项目洞察结果
#[derive(Debug)]
struct ProjectInsight {
    /// 项目名称
    name: String,
    /// 项目类型 (e.g., "Rust Library", "TypeScript Web App")
    project_type: Option<String>,
    /// 语言分布
    lang_stats: Vec<(String, usize)>,
    /// 总文件数
    total_files: usize,
    /// 模块映射 (路径 -> 描述)
    module_map: Vec<ModuleEntry>,
    /// 依赖关系
    dependencies: Vec<DependencyEdge>,
    /// 核心符号/入口点
    key_symbols: Vec<KeySymbol>,
    /// 外部依赖
    external_deps: Vec<String>,
}

/// 模块条目
#[derive(Debug)]
struct ModuleEntry {
    path: String,
    depth: usize,
    is_dir: bool,
    symbol_count: usize,
    description: Option<String>,
}

/// 依赖边
#[derive(Debug)]
struct DependencyEdge {
    from: String,
    to: String,
    relation: String,
}

/// 核心符号
#[derive(Debug)]
struct KeySymbol {
    name: String,
    kind: String,
    location: String,
    signature: Option<String>,
}

/// Code search tool implementation (local Tantivy + Tree-sitter engine)
pub struct AcemcpTool;

impl AcemcpTool {
    /// Execute codebase search using local engine
    pub async fn search_context(request: SearchRequest) -> Result<CallToolResult, McpToolError> {
        // 自动检测项目路径
        let project_root = match &request.project_root_path {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            _ => {
                // 自动检测：优先 Git 根目录，备选当前工作目录
                match detect_project_root() {
                    Some(path) => path,
                    None => {
                        return Ok(crate::mcp::create_error_result(
                            "无法自动检测项目路径。请提供 project_root_path 参数，或确保在 Git 仓库中运行。".to_string()
                        ));
                    }
                }
            }
        };

        let project_root_str = project_root.to_string_lossy().to_string();
        
        // 更新项目路径缓存（用于前端显示）
        crate::ui::agents_commands::update_project_path_cache(&project_root_str);
        
        log_important!(
            info,
            "Code search request: project_root_path={}, query={}, mode={:?}",
            project_root_str,
            request.query,
            request.mode
        );
        
        // Validate project path
        if !project_root.exists() {
            return Ok(crate::mcp::create_error_result(format!(
                "Project path does not exist: {}", project_root_str
            )));
        }

        let mode = request.mode.unwrap_or(SearchMode::Text);
        
        // Structure 模式：返回项目结构概览
        if matches!(mode, SearchMode::Structure) {
            return Self::get_project_structure(&project_root).await;
        }
        
        // 检查索引状态，决定使用 Tantivy 还是 ripgrep
        let use_tantivy = is_search_initialized() && is_project_indexed(&project_root);
        let is_indexing = is_project_indexing(&project_root);
        
        log_important!(
            info,
            "Search strategy: tantivy={}, indexing={}, mode={:?}",
            use_tantivy, is_indexing, mode
        );

        let search_result = if use_tantivy {
            // 索引就绪，使用 Tantivy 搜索
            let searcher = match create_searcher_for_project(&project_root) {
                Ok(s) => s,
                Err(e) => {
                    log_important!(warn, "Failed to create Tantivy searcher: {}, falling back to ripgrep", e);
                    return Self::search_with_ripgrep(&project_root, &request.query, mode).await;
                }
            };
            
            match mode {
                // 使用嵌入模型进行语义增强搜索（如果服务可用）
                SearchMode::Text => searcher.search_with_embedding(&request.query).await,
                SearchMode::Symbol => searcher.search_symbol(&request.query),
                SearchMode::Structure => unreachable!("Structure mode handled earlier"),
            }
        } else {
            // 索引未就绪，使用 ripgrep 回退
            // 🔧 修复: 无论 is_search_initialized 状态如何，都尝试触发后台索引
            if !is_indexing {
                // 先尝试确保搜索系统已初始化
                Self::ensure_search_initialized();
                // 然后触发后台索引
                if is_search_initialized() {
                    Self::trigger_background_indexing(&project_root);
                }
            }
            
            return Self::search_with_ripgrep(&project_root, &request.query, mode).await;
        };
            
        match search_result {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(crate::mcp::create_success_result(vec![Content::text(
                        "No relevant code context found."
                    )]));
                }
                
                let mut formatted = String::new();
                
                // 添加索引状态信息
                if let Some(state) = get_index_state(&project_root) {
                    let status = if state.indexing {
                        "⚡ Indexing"
                    } else if state.ready {
                        "✅ Ready"
                    } else {
                        "⏳ Pending"
                    };
                    formatted.push_str(&format!("[Index: {} | Files: {}]\n", status, state.file_count));
                }
                
                let mode_str = match mode { SearchMode::Text => "Text", SearchMode::Symbol => "Symbol", SearchMode::Structure => "Structure" };
                formatted.push_str(&format!("Found {} relevant snippets (Mode: {}):\n\n", results.len(), mode_str));
                
                // 批量查询所有相关文件的修改历史（性能优化）
                let all_paths: Vec<String> = results.iter().map(|r| r.path.clone()).collect();
                let changes_by_file = Self::get_changes_for_files(&project_root_str, &all_paths, &request.query);
                
                for res in &results {
                    // 增强格式：显示路径和分数
                    formatted.push_str(&format!("### 📄 `{}` (Score: {:.2})\n", res.path, res.score));
                    
                    // 显示该文件的最近修改历史
                    if let Some(changes) = changes_by_file.get(&res.path) {
                        for change in changes.iter().take(3) {
                            let ago = Self::format_time_ago(change.created_at);
                            formatted.push_str(&format!("  📝 {} ({})\n", change.summary, ago));
                        }
                    }
                    
                    // 显示结构化上下文（如果有）
                    if let Some(ref ctx) = res.context {
                        let mut context_parts = Vec::new();
                        
                        if let Some(ref parent) = ctx.parent_symbol {
                            context_parts.push(format!("**{}**", parent));
                        }
                        if let Some(ref kind) = ctx.symbol_kind {
                            if let Some(ref vis) = ctx.visibility {
                                context_parts.push(format!("{} {}", vis, kind));
                            } else {
                                context_parts.push(kind.clone());
                            }
                        }
                        
                        if !context_parts.is_empty() {
                            formatted.push_str(&format!("📍 {}\n", context_parts.join(" → ")));
                        }
                        
                        if let Some(ref sig) = ctx.signature {
                            formatted.push_str(&format!("📝 `{}`\n", sig));
                        }
                        
                        if let Some(ref doc) = ctx.doc_comment {
                            formatted.push_str(&format!("💡 {}\n", doc));
                        }
                    }
                    
                    // 显示匹配信息（如果有）
                    if let Some(ref info) = res.match_info {
                        if !info.matched_terms.is_empty() {
                            formatted.push_str(&format!("🔍 Matched: [{}] ({})\n", 
                                info.matched_terms.join(", "), 
                                info.match_type
                            ));
                        }
                    }
                    
                    // 代码片段
                    formatted.push_str("```\n");
                    formatted.push_str(&res.snippet);
                    formatted.push_str("```\n\n");
                }
                
                Ok(crate::mcp::create_success_result(vec![Content::text(formatted)]))
            }
            Err(e) => Ok(crate::mcp::create_error_result(format!("Search failed: {}", e)))
        }
    }

    /// 使用 ripgrep/ctags 进行搜索（回退方案）
    async fn search_with_ripgrep(
        project_root: &PathBuf,
        query: &str,
        mode: SearchMode,
    ) -> Result<CallToolResult, McpToolError> {
        // 符号搜索优先使用 ctags
        if matches!(mode, SearchMode::Symbol) && CtagsIndexer::is_available() {
            log_important!(info, "Using ctags for symbol search");
            return Self::search_with_ctags(project_root, query).await;
        }

        log_important!(info, "Using ripgrep fallback for search");
        
        // 检查 ripgrep 是否可用
        if !RipgrepSearcher::is_available() {
            return Ok(crate::mcp::create_error_result(
                "Search index not ready and ripgrep not available. Please install ripgrep (rg) or wait for indexing to complete.".to_string()
            ));
        }

        let rg_searcher = RipgrepSearcher::new(10, 3);
        
        match rg_searcher.search(project_root, query) {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(crate::mcp::create_success_result(vec![Content::text(
                        "No relevant code context found."
                    )]));
                }
                
                let mut formatted = String::new();
                let mode_str = match mode { SearchMode::Text => "Text", SearchMode::Symbol => "Symbol", SearchMode::Structure => "Structure" };
                formatted.push_str(&format!("Found {} snippets via ripgrep (Mode: {}):\n", results.len(), mode_str));
                formatted.push_str("💡 Note: Using ripgrep fallback. Index building in background for faster future searches.\n\n");
                
                for res in results {
                    formatted.push_str(&format!("--- {} ---\n", res.path));
                    formatted.push_str(&res.snippet);
                    formatted.push_str("\n\n");
                }
                
                Ok(crate::mcp::create_success_result(vec![Content::text(formatted)]))
            }
            Err(e) => Ok(crate::mcp::create_error_result(format!("Ripgrep search failed: {}", e)))
        }
    }

    /// 使用 ctags 进行符号搜索
    async fn search_with_ctags(
        project_root: &PathBuf,
        query: &str,
    ) -> Result<CallToolResult, McpToolError> {
        let mut indexer = CtagsIndexer::new(project_root);
        
        // 加载或生成 tags
        if let Err(e) = indexer.load_tags() {
            log_important!(warn, "Failed to load ctags: {}, falling back to ripgrep", e);
            // 回退到 ripgrep
            let rg_searcher = RipgrepSearcher::new(10, 3);
            return match rg_searcher.search(project_root, query) {
                Ok(results) => {
                    let mut formatted = format!("Found {} snippets via ripgrep (Symbol mode, ctags unavailable):\n\n", results.len());
                    for res in results {
                        formatted.push_str(&format!("--- {} ---\n{}\n\n", res.path, res.snippet));
                    }
                    Ok(crate::mcp::create_success_result(vec![Content::text(formatted)]))
                }
                Err(e) => Ok(crate::mcp::create_error_result(format!("Search failed: {}", e)))
            };
        }

        let symbols = indexer.search_symbol(query);
        
        if symbols.is_empty() {
            return Ok(crate::mcp::create_success_result(vec![Content::text(
                "No matching symbols found."
            )]));
        }

        let mut formatted = String::new();
        formatted.push_str(&format!("Found {} symbols via ctags:\n\n", symbols.len()));

        for symbol in symbols {
            formatted.push_str(&format!(
                "📍 **{}** ({}) in `{}`:{}\n",
                symbol.name,
                symbol.kind,
                symbol.file,
                symbol.line
            ));
            if let Some(sig) = &symbol.signature {
                formatted.push_str(&format!("   Signature: {}\n", sig));
            }
            formatted.push('\n');
        }

        Ok(crate::mcp::create_success_result(vec![Content::text(formatted)]))
    }

    /// 确保搜索系统已初始化
    /// 
    /// 在 MCP stdio 模式下，daemon 可能未启动，需要在此处初始化
    fn ensure_search_initialized() {
        use crate::mcp::tools::unified_store::{
            init_global_search_config, init_global_store, init_global_watcher,
        };
        
        if is_search_initialized() {
            return;
        }
        
        // 获取缓存目录
        let base_cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("neurospec");
        
        let store_cache_dir = base_cache_dir.join("unified_store");
        let index_cache_dir = base_cache_dir.join("search_index");
        
        // 初始化全局存储
        let _ = init_global_store(&store_cache_dir);
        
        // 初始化全局搜索配置
        if let Err(e) = init_global_search_config(&index_cache_dir) {
            log_important!(warn, "Failed to initialize search config in fallback: {}", e);
        } else {
            log_important!(info, "Search system initialized via fallback");
        }
        
        // 初始化文件监听器
        let _ = init_global_watcher();
    }

    /// 在后台触发索引
    fn trigger_background_indexing(project_root: &PathBuf) {
        let root = project_root.clone();
        
        std::thread::spawn(move || {
            log_important!(info, "Starting background indexing for: {}", root.display());
            mark_indexing_started(&root);
            
            let config = LocalEngineConfig::default();
            match LocalIndexer::new(&config) {
                Ok(mut indexer) => {
                    match indexer.index_directory(&root) {
                        Ok(count) => {
                            mark_indexing_complete(&root, count);
                            log_important!(info, "Background indexing complete: {} files indexed", count);
                            
                            // 启动文件变化监听循环
                            Self::start_file_change_loop(root, config);
                        }
                        Err(e) => {
                            log_important!(error, "Background indexing failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log_important!(error, "Failed to create indexer: {}", e);
                }
            }
        });
    }

    /// 启动文件变化监听循环
    /// 
    /// 使用自适应休眠策略：
    /// - 有文件变化时，快速响应（500ms）
    /// - 无文件变化时，逐渐延长间隔（最大 10s）
    fn start_file_change_loop(project_root: PathBuf, config: LocalEngineConfig) {
        use crate::mcp::tools::unified_store::process_file_changes;
        
        std::thread::spawn(move || {
            log_important!(info, "Starting file change loop for: {}", project_root.display());
            
            let mut idle_cycles = 0u32;
            const MIN_SLEEP_MS: u64 = 500;
            const MAX_SLEEP_MS: u64 = 10000;
            
            loop {
                // 自适应休眠：无变化时逐渐延长，有变化时重置
                let sleep_ms = MIN_SLEEP_MS.saturating_mul(1 + idle_cycles as u64).min(MAX_SLEEP_MS);
                std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
                
                // 处理文件变化
                match process_file_changes() {
                    Ok(count) if count > 0 => {
                        idle_cycles = 0; // 重置空闲计数
                        log_important!(info, "Detected {} file changes, updating index...", count);
                        
                        // 增量更新索引
                        if let Ok(mut indexer) = LocalIndexer::new(&config) {
                            if let Err(e) = indexer.index_directory(&project_root) {
                                log_important!(error, "Failed to update index: {}", e);
                            }
                        }
                    }
                    Ok(_) => {
                        // 无变化，增加空闲计数
                        idle_cycles = idle_cycles.saturating_add(1).min(20);
                    }
                    Err(e) => {
                        log_important!(error, "Error processing file changes: {}", e);
                    }
                }
            }
        });
    }

    /// Get project structure overview (structure mode)
    /// 
    /// 升级版：生成 Project Insight，包含：
    /// - 项目概览 (类型、语言分布)
    /// - 模块映射 (分层目录结构)
    /// - 依赖图谱 (模块间调用关系)
    /// - 核心符号 (公开 API/入口点)
    async fn get_project_structure(project_root: &PathBuf) -> Result<CallToolResult, McpToolError> {
        log_important!(info, "Generating Project Insight for: {}", project_root.display());
        
        // 🚀 优化：单次遍历收集基础信息和模块映射
        let (lang_stats, total_files, module_map) = Self::collect_project_data(project_root);
        
        // 生成依赖图谱 (使用 CodeGraph)
        let dependencies = Self::generate_dependency_graph(project_root);
        
        // 提取核心符号
        let key_symbols = Self::generate_key_symbols(project_root);
        
        // 解析外部依赖（用于类型检测）
        let external_deps = Self::parse_external_deps(project_root);
        
        // 检测项目类型
        let project_type = Self::detect_project_type(project_root, &lang_stats, &external_deps);
        
        // 7. 获取项目名称
        let project_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // 构建 ProjectInsight
        let insight = ProjectInsight {
            name: project_name,
            project_type,
            lang_stats,
            total_files,
            module_map,
            dependencies,
            key_symbols,
            external_deps,
        };
        
        // 格式化输出
        let output = Self::format_project_insight(&insight, project_root);
        
        Ok(crate::mcp::create_success_result(vec![Content::text(output)]))
    }

    /// 🚀 单次遍历收集项目数据
    /// 
    /// 合并了原 collect_basic_stats 和 generate_module_map 的逻辑，
    /// 一次遍历同时收集：语言统计、文件数、模块映射
    fn collect_project_data(project_root: &Path) -> (Vec<(String, usize)>, usize, Vec<ModuleEntry>) {
        use ignore::WalkBuilder;
        use std::collections::HashSet;
        
        let walker = WalkBuilder::new(project_root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();
        
        let mut lang_stats: HashMap<String, usize> = HashMap::new();
        let mut total_files = 0;
        let mut module_entries = Vec::new();
        let mut seen_dirs: HashSet<String> = HashSet::new();
        
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            let rel_path = match path.strip_prefix(project_root) {
                Ok(p) => p.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            
            if rel_path.is_empty() {
                continue;
            }
            
            let depth = rel_path.matches('/').count();
            
            if path.is_file() {
                total_files += 1;
                
                // 统计语言分布
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let lang = Self::ext_to_language(ext);
                    *lang_stats.entry(lang).or_insert(0) += 1;
                }
                
                // 收集关键入口文件（用于模块映射）
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if Self::is_key_file(name) && depth <= 4 {
                        module_entries.push(ModuleEntry {
                            path: rel_path,
                            depth,
                            is_dir: false,
                            symbol_count: 0,
                            description: None,
                        });
                    }
                }
                
                if total_files >= 5000 {
                    break;
                }
            } else if path.is_dir() && depth <= 4 {
                // 收集目录（用于模块映射）
                if Self::is_code_directory(&rel_path) && !seen_dirs.contains(&rel_path) {
                    let dir_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    let description = Self::infer_module_description(dir_name, &rel_path);
                    
                    seen_dirs.insert(rel_path.clone());
                    module_entries.push(ModuleEntry {
                        path: rel_path,
                        depth,
                        is_dir: true,
                        symbol_count: 0,
                        description,
                    });
                }
            }
        }
        
        // 排序语言统计
        let mut lang_list: Vec<_> = lang_stats.into_iter().collect();
        lang_list.sort_by(|a, b| b.1.cmp(&a.1));
        
        // 排序并限制模块映射
        module_entries.sort_by(|a, b| a.path.cmp(&b.path));
        module_entries.truncate(50);
        
        (lang_list, total_files, module_entries)
    }

    /// 扩展名转语言名
    fn ext_to_language(ext: &str) -> String {
        match ext.to_lowercase().as_str() {
            "rs" => "Rust",
            "ts" | "tsx" => "TypeScript",
            "js" | "jsx" => "JavaScript",
            "py" => "Python",
            "vue" => "Vue",
            "go" => "Go",
            "java" => "Java",
            "kt" => "Kotlin",
            "swift" => "Swift",
            "c" | "h" => "C",
            "cpp" | "hpp" | "cc" => "C++",
            "cs" => "C#",
            "rb" => "Ruby",
            "php" => "PHP",
            "md" => "Markdown",
            "json" => "JSON",
            "toml" => "TOML",
            "yaml" | "yml" => "YAML",
            "html" => "HTML",
            "css" | "scss" | "sass" | "less" => "CSS",
            "sql" => "SQL",
            "sh" | "bash" | "zsh" => "Shell",
            _ => "Other",
        }.to_string()
    }

    /// 判断是否为关键文件
    fn is_key_file(name: &str) -> bool {
        matches!(name,
            // Rust
            "main.rs" | "lib.rs" | "mod.rs" | "Cargo.toml" |
            // JavaScript/TypeScript
            "index.ts" | "index.js" | "main.ts" | "main.js" | "app.ts" | "app.js" |
            "package.json" | "tsconfig.json" |
            // Vue/React
            "App.vue" | "App.tsx" | "App.jsx" |
            // Python
            "main.py" | "__init__.py" | "app.py" | "pyproject.toml" | "setup.py" |
            // Go
            "main.go" | "go.mod" |
            // Config/Doc
            "README.md" | "AGENTS.md" | "Makefile" | "Dockerfile"
        )
    }

    // generate_module_map 已合并到 collect_project_data 中

    /// 判断是否为代码目录
    fn is_code_directory(path: &str) -> bool {
        // 排除非代码目录
        let exclude = ["node_modules", "target", "dist", "build", ".git", "__pycache__", "vendor"];
        !exclude.iter().any(|e| path.contains(e))
    }

    /// 推断模块描述
    fn infer_module_description(dir_name: &str, _path: &str) -> Option<String> {
        // 基于目录名推断功能
        let desc = match dir_name.to_lowercase().as_str() {
            "src" => "源代码",
            "lib" => "库代码",
            "bin" => "可执行入口",
            "mcp" => "MCP 协议实现",
            "tools" => "工具模块",
            "handlers" | "handler" => "请求处理器",
            "services" | "service" => "业务服务层",
            "models" | "model" => "数据模型",
            "types" => "类型定义",
            "utils" | "util" | "helpers" => "工具函数",
            "config" | "configs" => "配置管理",
            "api" => "API 接口",
            "routes" | "router" => "路由定义",
            "middleware" | "middlewares" => "中间件",
            "components" => "UI 组件",
            "pages" | "views" => "页面视图",
            "store" | "stores" => "状态管理",
            "hooks" => "React Hooks",
            "tests" | "test" | "__tests__" => "测试用例",
            "frontend" => "前端代码",
            "backend" => "后端代码",
            "core" => "核心模块",
            "common" | "shared" => "公共模块",
            "auth" | "authentication" => "认证模块",
            "database" | "db" => "数据库层",
            _ => return None,
        };
        Some(desc.to_string())
    }

    /// 生成依赖图谱 - 使用 CodeGraph 分析模块间调用关系
    fn generate_dependency_graph(project_root: &Path) -> Vec<DependencyEdge> {
        // 尝试使用现有的 CodeGraph 基础设施
        #[cfg(feature = "experimental-neurospec")]
        {
            use crate::neurospec::services::graph::builder::GraphBuilder;
            
            let graph = GraphBuilder::build_from_project(&project_root.to_string_lossy());
            
            let mut edges = Vec::new();
            
            // 遍历图中的边，提取模块级依赖
            for edge in graph.graph.edge_indices() {
                if let (Some(source), Some(target)) = (
                    graph.graph.edge_endpoints(edge).map(|(s, _)| s),
                    graph.graph.edge_endpoints(edge).map(|(_, t)| t),
                ) {
                    if let (Some(src_node), Some(tgt_node)) = (
                        graph.graph.node_weight(source),
                        graph.graph.node_weight(target),
                    ) {
                        // 只保留跨文件的调用
                        if src_node.file_path != tgt_node.file_path {
                            let relation = graph.graph.edge_weight(edge)
                                .map(|r| format!("{:?}", r))
                                .unwrap_or_else(|| "calls".to_string());
                            
                            edges.push(DependencyEdge {
                                from: format!("{}::{}", src_node.file_path, src_node.name),
                                to: format!("{}::{}", tgt_node.file_path, tgt_node.name),
                                relation,
                            });
                        }
                    }
                }
            }
            
            // 去重并限制数量
            edges.sort_by(|a, b| a.from.cmp(&b.from));
            edges.dedup_by(|a, b| a.from == b.from && a.to == b.to);
            edges.truncate(30);
            
            return edges;
        }
        
        #[cfg(not(feature = "experimental-neurospec"))]
        {
            // 无 neurospec feature 时返回空
            Vec::new()
        }
    }

    /// 提取核心符号/入口点
    fn generate_key_symbols(project_root: &Path) -> Vec<KeySymbol> {
        #[cfg(feature = "experimental-neurospec")]
        {
            use crate::neurospec::services::xray_engine::{scan_project, ScanConfig};
            
            let config = ScanConfig { max_files: 500 };
            
            match scan_project(project_root, Some(config)) {
                Ok(snapshot) => {
                    // 先过滤出函数和类
                    let filtered: Vec<_> = snapshot.symbols
                        .into_iter()
                        .filter(|s| {
                            matches!(s.kind, 
                                crate::neurospec::models::SymbolKind::Function |
                                crate::neurospec::models::SymbolKind::Class
                            )
                        })
                        .collect();
                    
                    // 优先获取公开 API
                    let public_symbols: Vec<KeySymbol> = filtered.iter()
                        .filter(|s| {
                            s.signature.as_ref().map(|sig| 
                                sig.contains("pub ") || sig.contains("export ")
                            ).unwrap_or(false)
                        })
                        .take(20)
                        .map(|s| KeySymbol {
                            name: s.name.clone(),
                            kind: format!("{:?}", s.kind),
                            location: s.path.clone(),
                            signature: s.signature.clone(),
                        })
                        .collect();
                    
                    // 如果公开 API 太少，补充其他符号
                    if public_symbols.len() >= 10 {
                        public_symbols
                    } else {
                        filtered.into_iter()
                            .take(15)
                            .map(|s| KeySymbol {
                                name: s.name,
                                kind: format!("{:?}", s.kind),
                                location: s.path,
                                signature: s.signature,
                            })
                            .collect()
                    }
                }
                Err(_) => Vec::new(),
            }
        }
        
        #[cfg(not(feature = "experimental-neurospec"))]
        {
            Vec::new()
        }
    }

    /// 解析外部依赖
    fn parse_external_deps(project_root: &Path) -> Vec<String> {
        let mut deps = Vec::new();
        
        // 尝试解析 Cargo.toml
        let cargo_path = project_root.join("Cargo.toml");
        if cargo_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_path) {
                // 解析多个依赖段：dependencies, dev-dependencies, build-dependencies
                let dep_sections = [
                    "[dependencies]",
                    "[dev-dependencies]", 
                    "[build-dependencies]",
                ];
                
                let mut in_deps = false;
                for line in content.lines() {
                    let trimmed = line.trim();
                    
                    // 检查是否进入依赖段
                    if dep_sections.iter().any(|s| trimmed.starts_with(s)) {
                        in_deps = true;
                        continue;
                    }
                    
                    // 遇到其他段落时退出
                    if trimmed.starts_with('[') {
                        in_deps = false;
                        continue;
                    }
                    
                    // 跳过注释和空行
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    
                    if in_deps {
                        // 提取依赖名：支持多种格式
                        // - name = "version"
                        // - name = { version = "1.0" }
                        // - name.workspace = true
                        if let Some(dep_name) = trimmed.split(['=', '.']).next() {
                            let name = dep_name.trim();
                            if !name.is_empty() && !deps.contains(&name.to_string()) {
                                deps.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // 尝试解析 package.json
        let pkg_path = project_root.join("package.json");
        if pkg_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(dependencies) = json.get("dependencies").and_then(|d| d.as_object()) {
                        for key in dependencies.keys() {
                            deps.push(key.clone());
                        }
                    }
                }
            }
        }
        
        // 限制数量
        deps.truncate(20);
        deps
    }

    /// 检测项目类型
    fn detect_project_type(
        project_root: &Path,
        lang_stats: &[(String, usize)],
        external_deps: &[String],
    ) -> Option<String> {
        let primary_lang = lang_stats.first().map(|(l, _)| l.as_str());
        
        // 基于文件和依赖推断项目类型
        let has_tauri = project_root.join("tauri.conf.json").exists() 
            || external_deps.iter().any(|d| d == "tauri");
        let has_mcp = external_deps.iter().any(|d| d.contains("mcp") || d.contains("rmcp"));
        let has_web = project_root.join("index.html").exists() 
            || external_deps.iter().any(|d| d == "react" || d == "vue" || d == "vite");
        let has_api = external_deps.iter().any(|d| 
            d == "axum" || d == "actix-web" || d == "express" || d == "fastapi"
        );
        
        match primary_lang {
            Some("Rust") => {
                if has_tauri && has_mcp {
                    Some("Tauri + MCP Server".to_string())
                } else if has_tauri {
                    Some("Tauri Desktop App".to_string())
                } else if has_mcp {
                    Some("MCP Server".to_string())
                } else if has_api {
                    Some("Rust Web API".to_string())
                } else if project_root.join("Cargo.toml").exists() {
                    // 检查是 lib 还是 bin
                    let cargo = std::fs::read_to_string(project_root.join("Cargo.toml")).unwrap_or_default();
                    if cargo.contains("[lib]") && !cargo.contains("[[bin]]") {
                        Some("Rust Library".to_string())
                    } else {
                        Some("Rust Application".to_string())
                    }
                } else {
                    Some("Rust Project".to_string())
                }
            }
            Some("TypeScript") | Some("JavaScript") => {
                if has_web {
                    Some("Web Application".to_string())
                } else if has_api {
                    Some("Node.js API".to_string())
                } else {
                    Some("TypeScript/JavaScript Project".to_string())
                }
            }
            Some("Python") => {
                if has_api {
                    Some("Python Web API".to_string())
                } else {
                    Some("Python Project".to_string())
                }
            }
            Some("Vue") => Some("Vue.js Application".to_string()),
            Some("Go") => Some("Go Application".to_string()),
            _ => None,
        }
    }

    /// 格式化 Project Insight 输出
    fn format_project_insight(insight: &ProjectInsight, project_root: &Path) -> String {
        let mut output = String::new();
        
        // Header
        output.push_str(&format!("# 🔍 Project Insight: {}\n\n", insight.name));
        
        // Overview
        output.push_str("## Overview\n");
        if let Some(ref ptype) = insight.project_type {
            output.push_str(&format!("- **Type:** {}\n", ptype));
        }
        let stack: Vec<_> = insight.lang_stats.iter()
            .take(3)
            .map(|(l, _)| l.as_str())
            .collect();
        output.push_str(&format!("- **Stack:** {}\n", stack.join(", ")));
        output.push_str(&format!("- **Size:** {} files\n\n", insight.total_files));
        
        // Module Map
        if !insight.module_map.is_empty() {
            output.push_str("## 🏗️ Module Map\n");
            output.push_str("```\n");
            for entry in &insight.module_map {
                let indent = "  ".repeat(entry.depth);
                let icon = if entry.is_dir { "📁" } else { "📄" };
                let desc = entry.description.as_ref()
                    .map(|d| format!("  # {}", d))
                    .unwrap_or_default();
                output.push_str(&format!("{}{} {}{}\n", indent, icon, entry.path.split('/').last().unwrap_or(&entry.path), desc));
            }
            output.push_str("```\n\n");
        }
        
        // Dependency Graph
        if !insight.dependencies.is_empty() {
            output.push_str("## 🔗 Dependency Graph\n");
            output.push_str("```\n");
            for edge in &insight.dependencies {
                // 简化路径显示
                let from_short = edge.from.split("::").last().unwrap_or(&edge.from);
                let to_short = edge.to.split("::").last().unwrap_or(&edge.to);
                output.push_str(&format!("{} → {} ({})\n", from_short, to_short, edge.relation));
            }
            output.push_str("```\n\n");
        }
        
        // Key Symbols
        if !insight.key_symbols.is_empty() {
            output.push_str("## 🔑 Key Symbols\n");
            output.push_str("| Symbol | Kind | Location |\n");
            output.push_str("|--------|------|----------|\n");
            for sym in &insight.key_symbols {
                output.push_str(&format!("| `{}` | {} | {} |\n", 
                    sym.name, 
                    sym.kind,
                    sym.location.split('/').last().unwrap_or(&sym.location)
                ));
            }
            output.push('\n');
        }
        
        // Index Status
        if let Some(state) = get_index_state(project_root) {
            output.push_str("## 📈 Index Status\n");
            let status = if state.indexing { 
                "⚡ Building" 
            } else if state.ready { 
                "✅ Ready" 
            } else { 
                "⏳ Pending" 
            };
            output.push_str(&format!("- **Status:** {}\n", status));
            output.push_str(&format!("- **Indexed Files:** {}\n", state.file_count));
        }
        
        output
    }

    /// Get tool definition for MCP
    pub fn get_tool_definition() -> Tool {
        use schemars::schema_for;

        let schema = schema_for!(SearchRequest);
        let schema_json = serde_json::to_value(&schema.schema).expect("Failed to serialize schema");

        if let serde_json::Value::Object(schema_map) = schema_json {
            crate::mcp::create_tool(
                "search",
                "🔍 PRIORITY TOOL: Always use this FIRST before reading files! Search for relevant code context in a project. Supports text search (natural language), symbol search (function/class names), and structure mode (project overview). Uses local Tantivy index with Tree-sitter for symbol extraction.",
                schema_map,
            )
        } else {
            panic!("Schema creation failed");
        }
    }

    // ========================================================================
    // 修改历史辅助函数
    // ========================================================================

    /// 批量获取文件的修改历史
    /// 
    /// 性能优化：一次查询获取所有相关文件的修改记录，按文件分组返回
    fn get_changes_for_files(
        project_root: &str,
        file_paths: &[String],
        query: &str,
    ) -> HashMap<String, Vec<CodeChangeMemory>> {
        let mut result: HashMap<String, Vec<CodeChangeMemory>> = HashMap::new();
        
        // 尝试创建 ChangeTracker
        let tracker = match ChangeTracker::new(project_root) {
            Ok(t) => t,
            Err(e) => {
                log_important!(warn, "Failed to create ChangeTracker: {}", e);
                return result;
            }
        };
        
        // 批量查询所有相关修改
        match tracker.find_relevant_changes(file_paths, query, 20) {
            Ok(changes) => {
                // 按文件路径分组
                for change in changes {
                    for file_path in &change.file_paths {
                        // 尝试匹配搜索结果中的路径
                        for search_path in file_paths {
                            if search_path.contains(file_path) || file_path.contains(search_path) {
                                result.entry(search_path.clone())
                                    .or_default()
                                    .push(change.clone());
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log_important!(warn, "Failed to query change history: {}", e);
            }
        }
        
        result
    }

    /// 格式化时间为相对时间（如 "3天前"、"1周前"）
    fn format_time_ago(time: DateTime<Utc>) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(time);
        
        let days = duration.num_days();
        let hours = duration.num_hours();
        let minutes = duration.num_minutes();
        
        if days > 30 {
            format!("{}个月前", days / 30)
        } else if days > 7 {
            format!("{}周前", days / 7)
        } else if days > 0 {
            format!("{}天前", days)
        } else if hours > 0 {
            format!("{}小时前", hours)
        } else if minutes > 0 {
            format!("{}分钟前", minutes)
        } else {
            "刚刚".to_string()
        }
    }
}

/// 自动检测项目根目录
/// 
/// 检测策略：
/// 1. 从当前工作目录向上查找 .git 目录
/// 2. 如果找不到 .git，返回当前工作目录
fn detect_project_root() -> Option<PathBuf> {
    // 获取当前工作目录
    let cwd = std::env::current_dir().ok()?;
    
    // 向上查找 .git 目录
    let mut current = cwd.as_path();
    loop {
        let git_dir = current.join(".git");
        if git_dir.exists() {
            log_important!(info, "Auto-detected project root (Git): {}", current.display());
            return Some(current.to_path_buf());
        }
        
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    
    // 没找到 .git，返回当前工作目录
    log_important!(info, "Auto-detected project root (CWD): {}", cwd.display());
    Some(cwd)
}
