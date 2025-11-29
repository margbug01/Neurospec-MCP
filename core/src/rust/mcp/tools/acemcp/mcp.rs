use anyhow::Result;
use rmcp::model::*;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use super::types::{SearchRequest, SearchMode, SearchProfile, SearchScope, SearchScopeKind, SearchError};
use super::local_engine::{LocalIndexer, LocalEngineConfig, RipgrepSearcher, CtagsIndexer};
use crate::log_important;
use crate::mcp::utils::errors::McpToolError;
use crate::mcp::tools::memory::{ChangeTracker, CodeChangeMemory};
use crate::mcp::tools::unified_store::{
    create_searcher_for_project, is_search_initialized, get_global_search_config,
    is_project_indexed, is_project_indexing, mark_indexing_started, mark_indexing_complete,
    get_index_state, assess_index_health, IndexHealth,
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
    // ========================================================================
    // Step 2: Profile-aware search context (重构后的主入口)
    // ========================================================================

    /// Execute codebase search using local engine
    /// 
    /// 优先级规则：
    /// 1. profile 一旦存在 → 优先生效
    /// 2. mode 只作为底层搜索引擎的 hint（Text / Symbol）
    /// 3. StructureOnly 走纯结构路径，不再看 mode
    /// 4. mode = Structure 仅在 profile.is_none() 时兼容旧行为
    pub async fn search_context(request: SearchRequest) -> Result<CallToolResult, McpToolError> {
        // ====== 阶段 1: 请求预处理 ======
        let project_root = match &request.project_root_path {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            _ => {
                match detect_project_root() {
                    Some(path) => path,
                    None => {
                        let err = SearchError::invalid_project_path("<auto-detect failed>");
                        return Ok(crate::mcp::create_error_result(err.to_json()));
                    }
                }
            }
        };

        let project_root_str = project_root.to_string_lossy().to_string();
        let profile = request.profile.clone();
        
        crate::ui::agents_commands::update_project_path_cache(&project_root_str);
        
        log_important!(
            info,
            "Code search request: project_root_path={}, query={}, mode={:?}, profile={:?}",
            project_root_str,
            request.query,
            request.mode,
            profile
        );
        
        if !project_root.exists() {
            let err = SearchError::invalid_project_path(&project_root_str);
            return Ok(crate::mcp::create_error_result(err.to_json()));
        }

        // ====== 阶段 2: Profile 决策层（profile 优先生效）======
        
        // 2.1 StructureOnly：直接返回结构概览，不看 mode
        if let Some(SearchProfile::StructureOnly { max_depth, max_nodes }) = &profile {
            return Self::get_project_structure(&project_root, *max_depth, *max_nodes).await;
        }

        // 2.2 SmartStructure：走独立的 orchestrator 路径
        let mode = request.mode.clone().unwrap_or(SearchMode::Text);
        if let Some(ref smart_profile) = profile {
            if matches!(smart_profile, SearchProfile::SmartStructure { .. }) {
                return Self::smart_structure_search(
                    &project_root,
                    &project_root_str,
                    &request,
                    mode,
                    smart_profile,
                ).await;
            }
        }

        // 2.3 兼容旧调用：仅当 profile 为空时才使用 mode=Structure
        if profile.is_none() && matches!(mode, SearchMode::Structure) {
            return Self::get_project_structure(&project_root, None, None).await;
        }
        
        // ====== 阶段 3: 旧模式（profile = None）的简单搜索 ======
        Self::legacy_search(&project_root, &project_root_str, &request, mode).await
    }

    // ========================================================================
    // Step 4: SmartStructure Orchestrator
    // ========================================================================

    /// SmartStructure 专用 orchestrator
    /// 
    /// 职责：
    /// - 调用引擎（tantivy / ripgrep）得到原始结果
    /// - 应用 scope / max_results 过滤
    /// - 处理 0 结果 → StructureOnly fallback
    /// - 生成「匹配分布 + 关键符号」汇总
    async fn smart_structure_search(
        project_root: &PathBuf,
        project_root_str: &str,
        request: &SearchRequest,
        mode: SearchMode,
        profile: &SearchProfile,
    ) -> Result<CallToolResult, McpToolError> {
        use crate::mcp::tools::acemcp::types::SearchTrace;
        use std::time::Instant;
        
        let start = Instant::now();
        let mut trace = SearchTrace::new(
            request.query.clone(),
            format!("{:?}", mode),
        );
        trace.profile = Some("SmartStructure".to_string());
        trace.index_health = format!("{:?}", assess_index_health(project_root));
        
        log_important!(info, "SmartStructure orchestrator: mode={:?}", mode);

        // 1. 调用统一引擎获取原始结果
        let raw_results = Self::run_search_engine(project_root, &request.query, mode.clone()).await;

        match raw_results {
            Ok(results) => {
                trace.result_count = results.len();
                trace.engine_used = if is_search_initialized() && is_project_indexed(project_root) {
                    "tantivy".to_string()
                } else {
                    "ripgrep".to_string()
                };
                
                // 2. 应用 SmartStructure 的 scope / max_results 过滤
                let filtered = Self::apply_smart_profile_filters(results, project_root, &Some(profile.clone()));

                // 3. 处理 0 结果 - 分级降级策略
                if filtered.is_empty() {
                    trace.fallback_chain.push("empty_results_fallback".to_string());
                    log_important!(info, "SmartStructure search returned no results, trying fallback strategies");
                    trace.duration_ms = start.elapsed().as_millis() as u64;
                    trace.log();
                    return Self::handle_empty_results(project_root, &request.query, mode).await;
                }

                trace.result_count = filtered.len();
                trace.duration_ms = start.elapsed().as_millis() as u64;
                trace.log();
                
                // 4. 格式化结果 + SmartStructure 汇总
                let formatted = Self::format_smart_structure_results(
                    &filtered,
                    project_root,
                    project_root_str,
                    &request.query,
                    mode,
                );

                Ok(crate::mcp::create_success_result(vec![Content::text(formatted)]))
            }
            Err(e) => {
                trace.engine_used = "failed".to_string();
                trace.duration_ms = start.elapsed().as_millis() as u64;
                trace.log();
                let err = SearchError::search_engine_error(&e);
                Ok(crate::mcp::create_error_result(err.to_json()))
            }
        }
    }
    
    /// 处理空结果 - 分级降级策略
    /// 
    /// 降级链：模糊匹配 → 文件名搜索 → 项目结构 + 建议
    async fn handle_empty_results(
        project_root: &PathBuf,
        query: &str,
        mode: SearchMode,
    ) -> Result<CallToolResult, McpToolError> {
        let mut suggestions = Vec::new();
        
        // Step 1: 尝试模糊匹配（简单拼写纠错）
        if let Some(fuzzy_query) = Self::generate_fuzzy_query(query) {
            log_important!(info, "Trying fuzzy match: '{}' -> '{}'", query, fuzzy_query);
            
            let fuzzy_results = Self::run_search_engine(project_root, &fuzzy_query, mode.clone()).await;
            if let Ok(results) = fuzzy_results {
                if !results.is_empty() {
                    suggestions.push(format!("未找到 `{}`，您是否要搜索 `{}`？", query, fuzzy_query));
                    
                    let formatted = format!(
                        "⚠️ **未找到精确匹配，以下是相似结果**\n\n\
                         💡 原查询：`{}`\n\
                         🔍 建议查询：`{}`\n\n\
                         ---\n\n{}",
                        query,
                        fuzzy_query,
                        Self::format_simple_results(&results, project_root, 5)
                    );
                    return Ok(crate::mcp::create_success_result(vec![Content::text(formatted)]));
                }
            }
        }
        
        // Step 2: 检测路径模式，尝试文件名搜索
        if Self::looks_like_path(query) {
            log_important!(info, "Query looks like a path, searching filenames");
            
            if let Ok(file_results) = Self::search_by_filename(project_root, query).await {
                if !file_results.is_empty() {
                    let formatted = format!(
                        "⚠️ **未找到内容匹配，但找到了相似文件名**\n\n\
                         💡 查询：`{}`\n\n\
                         ---\n\n{}",
                        query,
                        file_results.join("\n")
                    );
                    return Ok(crate::mcp::create_success_result(vec![Content::text(formatted)]));
                }
            }
        }
        
        // Step 3: 最后回退到项目结构 + 搜索建议
        log_important!(info, "All fallback strategies failed, showing project structure");
        
        let fallback_result = Self::get_project_structure(project_root, Some(3), Some(50)).await?;
        
        let structure_text = fallback_result.content.iter()
            .filter_map(|c| {
                if let Ok(val) = serde_json::to_value(c) {
                    val.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        // 生成搜索建议
        let query_suggestions = Self::generate_search_suggestions(query, &mode);
        let suggestions_text = if query_suggestions.is_empty() {
            String::new()
        } else {
            format!("\n💡 **搜索建议**：\n{}\n", query_suggestions.iter()
                .map(|s| format!("   - {}", s))
                .collect::<Vec<_>>()
                .join("\n"))
        };
        
        let wrapped = format!(
            "⚠️ **搜索无结果**\n\n\
             查询：`{}`\n\
             模式：{:?}\n{}\
             \n---\n\n\
             📁 **项目结构概览**（供参考）：\n\n{}",
            query,
            mode,
            suggestions_text,
            structure_text
        );
        
        Ok(crate::mcp::create_success_result(vec![Content::text(wrapped)]))
    }
    
    /// 生成模糊查询（简单拼写纠错）
    fn generate_fuzzy_query(query: &str) -> Option<String> {
        // 常见拼写错误纠正词典
        let corrections = [
            ("strucutre", "structure"),
            ("fucntion", "function"),
            ("calss", "class"),
            ("inteface", "interface"),
            ("pubic", "public"),
            ("privte", "private"),
            ("consturctor", "constructor"),
            ("retrun", "return"),
            ("imoprt", "import"),
            ("exprot", "export"),
        ];
        
        let lower = query.to_lowercase();
        for (typo, correct) in &corrections {
            if lower.contains(typo) {
                return Some(lower.replace(typo, correct));
            }
        }
        
        None
    }
    
    /// 检查查询是否像路径
    fn looks_like_path(query: &str) -> bool {
        query.contains('/') || query.contains('\\') || query.contains(".rs") 
            || query.contains(".ts") || query.contains(".js") || query.contains(".py")
    }
    
    /// 按文件名搜索
    async fn search_by_filename(project_root: &PathBuf, pattern: &str) -> Result<Vec<String>, String> {
        use ignore::WalkBuilder;
        
        let walker = WalkBuilder::new(project_root)
            .hidden(false)
            .git_ignore(true)
            .max_depth(Some(10))
            .build();
        
        let pattern_lower = pattern.to_lowercase();
        let mut matches = Vec::new();
        
        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.to_lowercase().contains(&pattern_lower) {
                        if let Ok(rel_path) = entry.path().strip_prefix(project_root) {
                            matches.push(format!("📄 `{}`", rel_path.display()));
                        }
                    }
                }
            }
            
            if matches.len() >= 10 {
                break;
            }
        }
        
        Ok(matches)
    }
    
    /// 生成搜索建议
    fn generate_search_suggestions(query: &str, mode: &SearchMode) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // 基于查询长度的建议
        if query.len() < 3 {
            suggestions.push("查询词过短，建议使用至少 3 个字符".to_string());
        }
        
        // 基于模式的建议
        match mode {
            SearchMode::Symbol if !query.chars().next().map(|c| c.is_alphanumeric()).unwrap_or(false) => {
                suggestions.push("符号搜索建议使用字母或数字开头".to_string());
            }
            SearchMode::Text if query.split_whitespace().count() == 1 => {
                suggestions.push("尝试使用多个关键词或相关术语".to_string());
            }
            _ => {}
        }
        
        // 通用建议
        if query.chars().all(|c| c.is_lowercase()) {
            suggestions.push("尝试使用驼峰命名或首字母大写".to_string());
        }
        
        suggestions
    }
    
    /// 简化结果格式（用于 fallback 展示）
    fn format_simple_results(
        results: &[crate::mcp::tools::acemcp::local_engine::types::SearchResult],
        _project_root: &PathBuf,
        limit: usize,
    ) -> String {
        let mut formatted = String::new();
        
        for (i, res) in results.iter().take(limit).enumerate() {
            formatted.push_str(&format!("{}. **{}** (行 {})\n", i + 1, res.path, res.line_number));
            formatted.push_str("```\n");
            formatted.push_str(&res.snippet.lines().take(5).collect::<Vec<_>>().join("\n"));
            formatted.push_str("\n```\n\n");
        }
        
        formatted
    }

    /// 格式化 SmartStructure 结果（含匹配分布 + 关键符号汇总）
    fn format_smart_structure_results(
        results: &[crate::mcp::tools::acemcp::local_engine::types::SearchResult],
        project_root: &PathBuf,
        project_root_str: &str,
        query: &str,
        mode: SearchMode,
    ) -> String {
        let mut formatted = String::new();

        // 索引状态
        if let Some(state) = get_index_state(project_root) {
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
        formatted.push_str(&format!("Found {} relevant snippets (Mode: {} | Profile: SmartStructure):\n\n", results.len(), mode_str));

        // 批量查询修改历史
        let all_paths: Vec<String> = results.iter().map(|r| r.path.clone()).collect();
        let changes_by_file = Self::get_changes_for_files(project_root_str, &all_paths, query);

        for res in results {
            formatted.push_str(&format!("### 📄 `{}` (Score: {:.2})\n", res.path, res.score));
            
            if let Some(changes) = changes_by_file.get(&res.path) {
                for change in changes.iter().take(3) {
                    let ago = Self::format_time_ago(change.created_at);
                    formatted.push_str(&format!("  📝 {} ({})\n", change.summary, ago));
                }
            }
            
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
            
            if let Some(ref info) = res.match_info {
                if !info.matched_terms.is_empty() {
                    formatted.push_str(&format!("🔍 Matched: [{}] ({})\n", 
                        info.matched_terms.join(", "), 
                        info.match_type
                    ));
                }
            }
            
            formatted.push_str("```\n");
            formatted.push_str(&res.snippet);
            formatted.push_str("```\n\n");
        }

        // SmartStructure 汇总
        formatted.push_str("\n---\n\n");
        
        // 匹配分布
        let mut dir_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for res in results {
            let dir = std::path::Path::new(&res.path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());
            *dir_counts.entry(dir).or_insert(0) += 1;
        }
        
        let mut dir_list: Vec<_> = dir_counts.into_iter().collect();
        dir_list.sort_by(|a, b| b.1.cmp(&a.1));
        
        formatted.push_str("## 📁 匹配分布\n\n");
        formatted.push_str("| 目录 | 匹配数 |\n");
        formatted.push_str("|------|--------|\n");
        for (dir, count) in dir_list.iter().take(5) {
            formatted.push_str(&format!("| `{}` | {} |\n", dir, count));
        }
        formatted.push_str("\n");
        
        // 关键符号
        let mut symbols: Vec<(String, String, usize)> = Vec::new();
        for res in results {
            if let Some(ref ctx) = res.context {
                if let Some(ref parent) = ctx.parent_symbol {
                    symbols.push((parent.clone(), res.path.clone(), res.line_number));
                }
            }
        }
        symbols.sort_by(|a, b| a.0.cmp(&b.0));
        symbols.dedup_by(|a, b| a.0 == b.0);
        
        if !symbols.is_empty() {
            formatted.push_str("## 🔗 关键符号\n\n");
            for (name, path, line) in symbols.iter().take(10) {
                formatted.push_str(&format!("- `{}` (`{}`:{})\n", name, path, line));
            }
            formatted.push_str("\n");
        }

        formatted
    }

    // ========================================================================
    // Step 2 & 3: 统一搜索引擎入口
    // ========================================================================

    /// 统一搜索引擎入口（tantivy 或 ripgrep）
    /// 
    /// 只负责：
    /// - 决定使用哪个引擎
    /// - 返回原始 Vec<SearchResult>
    /// - 错误统一为 String
    /// 
    /// 不负责：profile 过滤、格式化、fallback
    async fn run_search_engine(
        project_root: &PathBuf,
        query: &str,
        mode: SearchMode,
    ) -> Result<Vec<crate::mcp::tools::acemcp::local_engine::types::SearchResult>, String> {
        let is_indexing = is_project_indexing(project_root);
        
        // 使用智能健康检查替代硬编码阈值
        let health = assess_index_health(project_root);
        let use_tantivy = is_search_initialized() && matches!(health, IndexHealth::Healthy | IndexHealth::Degraded { .. });

        log_important!(
            info,
            "run_search_engine: tantivy={}, health={:?}, indexing={}, mode={:?}",
            use_tantivy, health, is_indexing, mode
        );

        if use_tantivy {
            // Tantivy 路径
            let searcher = match create_searcher_for_project(project_root) {
                Ok(s) => s,
                Err(e) => {
                    log_important!(warn, "Failed to create Tantivy searcher: {}, falling back to ripgrep", e);
                    return Self::search_with_ripgrep_raw_async(project_root, query, mode).await;
                }
            };

            let result = match mode {
                SearchMode::Text => searcher.search_with_embedding(query).await.map_err(|e| e.to_string()),
                SearchMode::Symbol => searcher.search_symbol(query).map_err(|e| e.to_string()),
                SearchMode::Structure => unreachable!("Structure mode handled earlier"),
            };
            
            // 如果 Tantivy 返回空结果且索引状态为 Degraded，尝试 ripgrep 补充
            match &result {
                Ok(results) if results.is_empty() && matches!(health, IndexHealth::Degraded { .. }) => {
                    log_important!(info, "Tantivy returned empty, trying ripgrep supplement due to degraded index");
                    Self::search_with_ripgrep_raw_async(project_root, query, mode).await
                }
                _ => result,
            }
        } else {
            // Ripgrep 回退路径
            if !is_indexing {
                Self::ensure_search_initialized();
                // 触发后台索引（带锁保护）
                Self::trigger_background_indexing_safe(project_root);
            }
            Self::search_with_ripgrep_raw_async(project_root, query, mode).await
        }
    }

    /// 异步包装的 ripgrep 搜索（避免阻塞 async runtime）
    async fn search_with_ripgrep_raw_async(
        project_root: &PathBuf,
        query: &str,
        mode: SearchMode,
    ) -> Result<Vec<crate::mcp::tools::acemcp::local_engine::types::SearchResult>, String> {
        let project_root = project_root.clone();
        let query = query.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::search_with_ripgrep_raw(&project_root, &query, mode)
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    /// Step 3: Ripgrep 原始结果接口（返回 Vec<SearchResult>，不做格式化）
    /// 
    /// 用于 SmartStructure 等需要后续 profile 过滤的场景
    fn search_with_ripgrep_raw(
        project_root: &PathBuf,
        query: &str,
        mode: SearchMode,
    ) -> Result<Vec<crate::mcp::tools::acemcp::local_engine::types::SearchResult>, String> {
        // 符号搜索优先使用 ctags
        if matches!(mode, SearchMode::Symbol) && CtagsIndexer::is_available() {
            log_important!(info, "Using ctags for symbol search (raw)");
            return Self::search_with_ctags_raw(project_root, query);
        }
        
        // 符号模式下，无 ctags 时使用正则符号搜索
        if matches!(mode, SearchMode::Symbol) {
            log_important!(info, "Using regex-based symbol search (ctags not available)");
            return Self::search_symbols_with_regex(project_root, query);
        }

        log_important!(info, "Using ripgrep fallback (raw)");

        if !RipgrepSearcher::is_available() {
            return Err("Ripgrep not available and index not ready".to_string());
        }

        let rg_searcher = RipgrepSearcher::new(10, 3);
        rg_searcher.search(project_root, query).map_err(|e| e.to_string())
    }
    
    /// 使用正则表达式搜索符号定义
    /// 
    /// 当 ctags 不可用时的回退方案，使用 ripgrep + 正则匹配符号定义行
    fn search_symbols_with_regex(
        project_root: &PathBuf,
        symbol_name: &str,
    ) -> Result<Vec<crate::mcp::tools::acemcp::local_engine::types::SearchResult>, String> {
        use std::process::{Command, Stdio};
        use std::io::{BufRead, BufReader};
        
        let rg_cmd = if cfg!(windows) { "rg.exe" } else { "rg" };
        
        // 构建符号定义正则表达式
        // 匹配常见符号定义：fn, struct, class, def, func, interface, trait, enum, type
        let patterns = vec![
            format!(r"fn\s+{}\s*[(<]", symbol_name),          // Rust function
            format!(r"struct\s+{}\s*[{{<]", symbol_name),      // Rust struct
            format!(r"enum\s+{}\s*[{{<]", symbol_name),        // Rust enum
            format!(r"trait\s+{}\s*[{{<:]", symbol_name),      // Rust trait
            format!(r"type\s+{}\s*=", symbol_name),            // Rust type alias
            format!(r"class\s+{}\s*[{{(<:]", symbol_name),     // Class (TS/JS/Python/Java)
            format!(r"interface\s+{}\s*[{{<]", symbol_name),   // TypeScript interface
            format!(r"def\s+{}\s*\(", symbol_name),            // Python function
            format!(r"func\s+{}\s*\(", symbol_name),           // Go function
            format!(r"function\s+{}\s*\(", symbol_name),       // JavaScript function
            format!(r"export\s+(const|let|var)\s+{}\s*=", symbol_name), // JS/TS export
        ];
        
        let combined_pattern = patterns.join("|");
        
        let mut child = Command::new(rg_cmd)
            .current_dir(project_root)
            .args([
                "--json",
                "-e", &combined_pattern,
                "--type-add", "code:*.{rs,ts,tsx,js,jsx,py,go,java,c,cpp,h,hpp,vue,svelte}",
                "--type", "code",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn ripgrep: {}", e))?;
        
        let stdout = child.stdout.take()
            .ok_or_else(|| "Failed to capture stdout".to_string())?;
        
        let reader = BufReader::new(stdout);
        let mut results: Vec<crate::mcp::tools::acemcp::local_engine::types::SearchResult> = Vec::new();
        let mut current_file: Option<String> = None;
        let mut current_line: Option<(usize, String)> = None;
        
        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => continue,
            };
            
            if line.is_empty() {
                continue;
            }
            
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                match json.get("type").and_then(|t| t.as_str()) {
                    Some("begin") => {
                        // 保存上一个匹配
                        if let (Some(file), Some((line_num, text))) = (current_file.take(), current_line.take()) {
                            results.push(crate::mcp::tools::acemcp::local_engine::types::SearchResult {
                                path: file,
                                score: 1.0,
                                snippet: text,
                                line_number: line_num,
                                context: None,
                                match_info: Some(crate::mcp::tools::acemcp::local_engine::types::MatchInfo {
                                    matched_terms: vec![symbol_name.to_string()],
                                    match_type: "symbol".to_string(),
                                    match_quality: "regex_symbol".to_string(),
                                }),
                            });
                        }
                        
                        if let Some(path) = json.get("data")
                            .and_then(|d| d.get("path"))
                            .and_then(|p| p.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            current_file = Some(path.to_string());
                        }
                    }
                    Some("match") => {
                        if let Some(data) = json.get("data") {
                            let line_num = data.get("line_number")
                                .and_then(|n| n.as_u64())
                                .unwrap_or(0) as usize;
                            
                            if let Some(text) = data.get("lines")
                                .and_then(|l| l.get("text"))
                                .and_then(|t| t.as_str())
                            {
                                current_line = Some((line_num, format!("{:4} | {}", line_num, text.trim())));
                            }
                        }
                    }
                    Some("end") => {
                        if let (Some(file), Some((line_num, text))) = (current_file.take(), current_line.take()) {
                            results.push(crate::mcp::tools::acemcp::local_engine::types::SearchResult {
                                path: file,
                                score: 1.0,
                                snippet: text,
                                line_number: line_num,
                                context: None,
                                match_info: Some(crate::mcp::tools::acemcp::local_engine::types::MatchInfo {
                                    matched_terms: vec![symbol_name.to_string()],
                                    match_type: "symbol".to_string(),
                                    match_quality: "regex_symbol".to_string(),
                                }),
                            });
                        }
                    }
                    _ => {}
                }
            }
            
            if results.len() >= 10 {
                break;
            }
        }
        
        // 处理最后一个
        if let (Some(file), Some((line_num, text))) = (current_file, current_line) {
            results.push(crate::mcp::tools::acemcp::local_engine::types::SearchResult {
                path: file,
                score: 1.0,
                snippet: text,
                line_number: line_num,
                context: None,
                match_info: Some(crate::mcp::tools::acemcp::local_engine::types::MatchInfo {
                    matched_terms: vec![symbol_name.to_string()],
                    match_type: "symbol".to_string(),
                    match_quality: "regex_symbol".to_string(),
                }),
            });
        }
        
        let _ = child.wait();
        Ok(results)
    }

    /// Ctags 原始结果接口
    fn search_with_ctags_raw(
        project_root: &PathBuf,
        query: &str,
    ) -> Result<Vec<crate::mcp::tools::acemcp::local_engine::types::SearchResult>, String> {
        let mut indexer = CtagsIndexer::new(project_root);
        
        if let Err(e) = indexer.load_tags() {
            log_important!(warn, "Failed to load ctags: {}, falling back to ripgrep", e);
            let rg_searcher = RipgrepSearcher::new(10, 3);
            return rg_searcher.search(project_root, query).map_err(|e| e.to_string());
        }

        let symbols = indexer.search_symbol(query);
        
        // 将 ctags 结果转换为 SearchResult 格式
        let results: Vec<crate::mcp::tools::acemcp::local_engine::types::SearchResult> = symbols
            .into_iter()
            .map(|sym| {
                let sig_clone = sym.signature.clone();
                crate::mcp::tools::acemcp::local_engine::types::SearchResult {
                    path: sym.file.clone(),
                    score: 1.0,
                    snippet: sig_clone.clone().unwrap_or_else(|| format!("{} ({})", sym.name, sym.kind)),
                    line_number: sym.line,
                    context: Some(crate::mcp::tools::acemcp::local_engine::types::SnippetContext {
                        module: None,
                        parent_symbol: None,
                        symbol_kind: Some(sym.kind.clone()),
                        visibility: None,
                        doc_comment: None,
                        signature: sig_clone,
                    }),
                    match_info: Some(crate::mcp::tools::acemcp::local_engine::types::MatchInfo {
                        matched_terms: vec![query.to_string()],
                        match_type: "symbol".to_string(),
                        match_quality: "exact".to_string(),
                    }),
                }
            })
            .collect();

        Ok(results)
    }

    /// 旧模式搜索（profile = None 时的兼容路径）
    async fn legacy_search(
        project_root: &PathBuf,
        project_root_str: &str,
        request: &SearchRequest,
        mode: SearchMode,
    ) -> Result<CallToolResult, McpToolError> {
        let use_tantivy = is_search_initialized() && is_project_indexed(project_root);
        let is_indexing = is_project_indexing(project_root);

        log_important!(
            info,
            "Legacy search: tantivy={}, indexing={}, mode={:?}",
            use_tantivy, is_indexing, mode
        );

        if use_tantivy {
            let searcher = match create_searcher_for_project(project_root) {
                Ok(s) => s,
                Err(e) => {
                    log_important!(warn, "Failed to create Tantivy searcher: {}, falling back to ripgrep", e);
                    return Self::search_with_ripgrep(project_root, &request.query, mode).await;
                }
            };

            let search_result = match mode {
                SearchMode::Text => searcher.search_with_embedding(&request.query).await,
                SearchMode::Symbol => searcher.search_symbol(&request.query),
                SearchMode::Structure => unreachable!("Structure mode handled earlier"),
            };

            match search_result {
                Ok(results) => {
                    if results.is_empty() {
                        return Ok(crate::mcp::create_success_result(vec![Content::text(
                            "No relevant code context found."
                        )]));
                    }
                    let formatted = Self::format_legacy_results(&results, project_root, project_root_str, &request.query, mode);
                    Ok(crate::mcp::create_success_result(vec![Content::text(formatted)]))
                }
                Err(e) => {
                    let err = SearchError::search_engine_error(&e.to_string());
                    Ok(crate::mcp::create_error_result(err.to_json()))
                }
            }
        } else {
            if !is_indexing {
                Self::ensure_search_initialized();
                if is_search_initialized() {
                    Self::trigger_background_indexing(project_root);
                }
            }
            Self::search_with_ripgrep(project_root, &request.query, mode).await
        }
    }

    /// 格式化旧模式结果（不含 SmartStructure 汇总）
    fn format_legacy_results(
        results: &[crate::mcp::tools::acemcp::local_engine::types::SearchResult],
        project_root: &PathBuf,
        project_root_str: &str,
        query: &str,
        mode: SearchMode,
    ) -> String {
        let mut formatted = String::new();

        if let Some(state) = get_index_state(project_root) {
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

        let all_paths: Vec<String> = results.iter().map(|r| r.path.clone()).collect();
        let changes_by_file = Self::get_changes_for_files(project_root_str, &all_paths, query);

        for res in results {
            formatted.push_str(&format!("### 📄 `{}` (Score: {:.2})\n", res.path, res.score));
            
            if let Some(changes) = changes_by_file.get(&res.path) {
                for change in changes.iter().take(3) {
                    let ago = Self::format_time_ago(change.created_at);
                    formatted.push_str(&format!("  📝 {} ({})\n", change.summary, ago));
                }
            }
            
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
            
            if let Some(ref info) = res.match_info {
                if !info.matched_terms.is_empty() {
                    formatted.push_str(&format!("🔍 Matched: [{}] ({})\n", 
                        info.matched_terms.join(", "), 
                        info.match_type
                    ));
                }
            }
            
            formatted.push_str("```\n");
            formatted.push_str(&res.snippet);
            formatted.push_str("```\n\n");
        }

        formatted
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
            let err = SearchError::index_not_ready();
            return Ok(crate::mcp::create_error_result(err.to_json()));
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
            Err(e) => {
                let err = SearchError::io_error(&e.to_string());
                Ok(crate::mcp::create_error_result(err.to_json()))
            }
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
                Err(e) => {
                    let err = SearchError::io_error(&e.to_string());
                    Ok(crate::mcp::create_error_result(err.to_json()))
                }
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

    /// 安全触发后台索引（带文件锁保护）
    /// 
    /// 使用简单的文件锁机制防止并发触发多个索引任务
    fn trigger_background_indexing_safe(project_root: &PathBuf) {
        use std::fs::{File, OpenOptions};
        use std::io::{Read, Write};
        
        // 获取锁文件路径
        let lock_path = match get_global_search_config() {
            Ok(config) => config.index_path.join(".indexing.lock"),
            Err(_) => {
                log_important!(warn, "Cannot get config for lock file, falling back to unsafe indexing");
                Self::trigger_background_indexing(project_root);
                return;
            }
        };
        
        // 确保锁文件目录存在
        if let Some(parent) = lock_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        // 检查锁文件是否存在且有效（包含正在运行的 PID）
        if lock_path.exists() {
            if let Ok(mut file) = File::open(&lock_path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    if let Ok(pid) = content.trim().parse::<u32>() {
                        // 检查进程是否还在运行
                        if Self::is_process_running(pid) {
                            log_important!(info, "Index lock held by PID {}, skipping duplicate indexing", pid);
                            return;
                        }
                    }
                }
            }
        }
        
        // 写入当前进程 PID 到锁文件
        let current_pid = std::process::id();
        match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                if write!(file, "{}", current_pid).is_err() {
                    log_important!(warn, "Failed to write lock file");
                }
            }
            Err(e) => {
                log_important!(warn, "Cannot create lock file: {}", e);
            }
        }
        
        log_important!(info, "Acquired index lock (PID: {}), triggering background indexing", current_pid);
        
        let root = project_root.clone();
        let lock_path_clone = lock_path.clone();
        std::thread::spawn(move || {
            Self::do_background_indexing(&root);
            // 索引完成后删除锁文件
            let _ = std::fs::remove_file(&lock_path_clone);
        });
    }
    
    /// 检查进程是否正在运行
    #[cfg(windows)]
    fn is_process_running(pid: u32) -> bool {
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains(&pid.to_string())
            })
            .unwrap_or(false)
    }
    
    #[cfg(not(windows))]
    fn is_process_running(pid: u32) -> bool {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }

    /// 在后台触发索引
    /// 
    /// 如果索引文件数 < 10，则执行重建索引；否则执行增量索引
    fn trigger_background_indexing(project_root: &PathBuf) {
        let root = project_root.clone();
        std::thread::spawn(move || {
            Self::do_background_indexing(&root);
        });
    }
    
    /// 执行后台索引的实际逻辑
    fn do_background_indexing(project_root: &PathBuf) {
        use crate::mcp::tools::unified_store::get_indexed_file_count;
        
        // 检查是否正在索引
        if is_project_indexing(project_root) {
            log_important!(info, "Project is already being indexed, skipping");
            return;
        }
        
        // 检查索引文件数，如果 < 10 则重建
        let should_rebuild = match get_indexed_file_count(project_root) {
            Some(count) if count < 10 => {
                log_important!(info, "Index has only {} files, will rebuild", count);
                true
            }
            None => {
                log_important!(info, "No existing index, will build from scratch");
                true
            }
            Some(count) => {
                log_important!(info, "Index has {} files, will do incremental update", count);
                false
            }
        };
        
        // 获取全局配置
        let config = match get_global_search_config() {
            Ok(c) => c,
            Err(_) => LocalEngineConfig::default(),
        };
        
        mark_indexing_started(project_root);
        
        log_important!(info, "Starting background indexing for: {} (index_path: {:?})", project_root.display(), config.index_path);
        
        match LocalIndexer::new(&config) {
            Ok(mut indexer) => {
                let result = if should_rebuild {
                    log_important!(info, "Executing full index rebuild...");
                    indexer.rebuild_index(project_root)
                } else {
                    log_important!(info, "Executing incremental indexing...");
                    indexer.index_directory(project_root)
                };
                
                match result {
                    Ok(count) => {
                        mark_indexing_complete(project_root, count);
                        log_important!(info, "Background indexing complete: {} files indexed", count);
                        
                        // 启动文件变化监听循环
                        Self::start_file_change_loop(project_root.clone(), config);
                    }
                    Err(e) => {
                        use crate::mcp::tools::unified_store::mark_index_corrupted;
                        mark_index_corrupted(project_root, &format!("Indexing failed: {}", e));
                        log_important!(error, "Background indexing failed: {}", e);
                    }
                }
            }
            Err(e) => {
                use crate::mcp::tools::unified_store::mark_index_corrupted;
                mark_index_corrupted(project_root, &format!("Failed to create indexer: {}", e));
                log_important!(error, "Failed to create indexer: {}", e);
            }
        }
    }

    /// 根据 SmartStructure profile 对搜索结果进行 scope / max_results 过滤
    fn apply_smart_profile_filters(
        mut results: Vec<crate::mcp::tools::acemcp::local_engine::types::SearchResult>,
        project_root: &PathBuf,
        profile: &Option<SearchProfile>,
    ) -> Vec<crate::mcp::tools::acemcp::local_engine::types::SearchResult> {
        let Some(SearchProfile::SmartStructure { scope, max_results }) = profile.as_ref() else {
            return results;
        };

        // 作用域过滤（目前只对 Folder/File 生效，Project/Symbol 不做额外限制）
        if let Some(scope) = scope.as_ref() {
            let root_str = project_root.to_string_lossy().to_string();

            results.retain(|res| Self::matches_scope(&root_str, &res.path, scope));
        }

        // 结果数量裁剪
        if let Some(max) = *max_results {
            let max = max as usize;
            if results.len() > max {
                results.truncate(max);
            }
        }

        results
    }

    /// 判断搜索结果是否命中指定 scope
    fn matches_scope(project_root: &str, result_path: &str, scope: &SearchScope) -> bool {
        use std::path::Path;

        match scope.kind {
            SearchScopeKind::Project => true,
            SearchScopeKind::Folder => {
                if let Some(ref folder) = scope.path {
                    let base = if Path::new(folder).is_absolute() {
                        folder.clone()
                    } else {
                        format!("{}/{}", project_root, folder)
                    };
                    result_path.starts_with(&base)
                } else {
                    true
                }
            }
            SearchScopeKind::File => {
                if let Some(ref file) = scope.path {
                    if Path::new(file).is_absolute() {
                        result_path == *file
                    } else {
                        let full = format!("{}/{}", project_root, file);
                        result_path == full
                    }
                } else {
                    true
                }
            }
            // 暂不根据符号名做进一步过滤，后续可以结合 SnippetContext/MatchInfo 增强
            SearchScopeKind::Symbol => true,
        }
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
    /// 并根据可选的 max_depth / max_nodes 进行简单裁剪。
    async fn get_project_structure(
        project_root: &PathBuf,
        max_depth: Option<u8>,
        max_nodes: Option<u32>,
    ) -> Result<CallToolResult, McpToolError> {
        log_important!(info, "Generating Project Insight for: {}", project_root.display());
        
        // 🚀 优化：单次遍历收集基础信息和模块映射
        let (lang_stats, total_files, mut module_map) = Self::collect_project_data(project_root);

        // 按深度和节点数量进行裁剪（如果配置了）
        if let Some(limit_depth) = max_depth {
            let limit = limit_depth as usize;
            module_map.retain(|m| m.depth <= limit);
        }

        if let Some(max_nodes) = max_nodes {
            let limit = max_nodes as usize;
            if module_map.len() > limit {
                module_map.truncate(limit);
            }
        }
        
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
fn detect_project_root() -> Option<PathBuf> {
    // 1. 优先使用缓存的项目路径
    if let Some(cached_path) = crate::ui::agents_commands::get_cached_project_path() {
        let path = PathBuf::from(&cached_path);
        if path.exists() && path.join(".git").exists() {
            log_important!(info, "Using cached project root: {}", path.display());
            return Some(path);
        }
    }
    
    // 2. 从当前工作目录检测（回退方案）
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


