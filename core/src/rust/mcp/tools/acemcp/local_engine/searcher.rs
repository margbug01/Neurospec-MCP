use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use tantivy::collector::TopDocs;
use tantivy::query::{QueryParser, PhraseQuery};
use tantivy::schema::Field;
use tantivy::{Index, ReloadPolicy, Term};

use super::types::{LocalEngineConfig, SearchResult, SnippetContext, MatchInfo};
use super::vector_store::CodeVectorStore;
use crate::neurospec::services::embedding::{find_similar, is_embedding_available};

/// 增强的 Snippet 提取结果
struct EnhancedSnippet {
    code: String,
    line_number: usize,
    context: SnippetContext,
    matched_terms: Vec<String>,
}

pub struct LocalSearcher {
    index: Index,
    project_root: PathBuf,
    config: LocalEngineConfig,
}

impl LocalSearcher {
    pub fn new(config: LocalEngineConfig, project_root: PathBuf) -> Result<Self> {
        let index = Index::open_in_dir(&config.index_path)?;

        Ok(Self {
            index,
            project_root,
            config,
        })
    }

    /// 全文搜索
    pub fn search(&self, query_str: &str) -> Result<Vec<SearchResult>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        let searcher = reader.searcher();
        let schema = self.index.schema();

        let field_path = schema.get_field("path").context("Missing path field")?;
        let field_content = schema.get_field("content").context("Missing content field")?;
        let field_symbols = schema.get_field("symbols").context("Missing symbols field")?;
        let field_snippet = schema.get_field("snippet").ok();

        // 预处理查询：扩展常见术语
        let expanded_query = Self::expand_query(query_str);

        // 配置多字段查询解析器，优化权重策略：
        // - 符号名匹配最重要 (5.0)
        // - 路径包含关键词也重要 (2.0) - 如 auth/login.rs
        // - 内容兜底 (1.0)
        let mut query_parser = QueryParser::for_index(
            &self.index, 
            vec![field_symbols, field_path, field_content]
        );
        query_parser.set_field_boost(field_symbols, 5.0);
        query_parser.set_field_boost(field_path, 2.0);
        query_parser.set_field_boost(field_content, 1.0);

        let query = query_parser.parse_query(&expanded_query)?;

        // Execute Search
        let top_docs = searcher.search(&query, &TopDocs::with_limit(self.config.max_results))?;

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;

            let path_val = retrieved_doc
                .get_first(field_path)
                .and_then(|v| v.as_text())
                .unwrap_or("");

            // 优先使用预存 snippet，否则回退到读文件
            let (snippet, line) = if let Some(field) = field_snippet {
                if let Some(stored_snippet) = retrieved_doc.get_first(field).and_then(|v| v.as_text()) {
                    (stored_snippet.to_string(), 1)
                } else {
                    self.fallback_snippet(path_val, query_str)
                }
            } else {
                self.fallback_snippet(path_val, query_str)
            };

            // 提取增强上下文
            let full_path = self.project_root.join(path_val);
            let enhanced = if let Ok(content) = fs::read_to_string(&full_path) {
                self.extract_enhanced_snippet(&content, path_val, query_str, line)
            } else {
                EnhancedSnippet {
                    code: snippet.clone(),
                    line_number: line,
                    context: SnippetContext::default(),
                    matched_terms: vec![],
                }
            };

            results.push(SearchResult {
                path: path_val.to_string(),
                score,
                snippet: enhanced.code,
                line_number: enhanced.line_number,
                context: Some(enhanced.context),
                match_info: Some(MatchInfo {
                    matched_terms: enhanced.matched_terms,
                    match_type: "content".to_string(),
                    match_quality: "partial".to_string(),
                }),
            });
        }

        Ok(results)
    }

    /// 使用嵌入模型进行语义增强的搜索（异步版本）
    /// 
    /// 如果嵌入服务可用，会对 TF-IDF 结果进行语义重排序
    /// 如果 TF-IDF 无结果，会尝试纯向量搜索
    pub async fn search_with_embedding(&self, query_str: &str) -> Result<Vec<SearchResult>> {
        // 先执行普通搜索
        let mut results = self.search(query_str)?;
        
        // 检查嵌入服务是否可用
        if !is_embedding_available() {
            return Ok(results);
        }
        
        // 如果 TF-IDF 无结果，尝试纯向量搜索
        if results.is_empty() {
            return self.search_by_vector(query_str).await;
        }
        
        // 构建候选文本列表（使用路径 + snippet 的组合）
        let candidates: Vec<String> = results.iter()
            .map(|r| format!("{} {}", r.path, r.snippet))
            .collect();
        
        // 使用嵌入进行语义匹配
        if let Some(similar) = find_similar(query_str, &candidates, results.len()).await {
            // 创建语义分数映射
            let semantic_scores: std::collections::HashMap<usize, f32> = similar.into_iter().collect();
            
            // 混合排序：TF-IDF (60%) + Embedding (40%)
            for (i, result) in results.iter_mut().enumerate() {
                let semantic_score = semantic_scores.get(&i).copied().unwrap_or(0.0);
                let combined = result.score * 0.6 + semantic_score * 10.0 * 0.4; // 归一化
                result.score = combined;
            }
            
            // 重新排序
            results.sort_by(|a, b| {
                b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        
        Ok(results)
    }

    /// 纯向量搜索（当 TF-IDF 无结果时使用）
    async fn search_by_vector(&self, query_str: &str) -> Result<Vec<SearchResult>> {
        // 尝试加载向量存储
        let vector_store = match CodeVectorStore::new(&self.project_root) {
            Ok(store) => store,
            Err(_) => return Ok(vec![]),
        };

        // 获取所有有向量的代码
        let entries = vector_store.get_all_with_vectors()?;
        if entries.is_empty() {
            return Ok(vec![]);
        }

        // 构建候选文本
        let candidates: Vec<String> = entries.iter()
            .map(|e| format!("{} {}", e.summary, e.symbols.join(" ")))
            .collect();

        // 使用嵌入计算相似度
        let similar = match find_similar(query_str, &candidates, self.config.max_results).await {
            Some(s) => s,
            None => return Ok(vec![]),
        };

        // 构建搜索结果
        let mut results = Vec::new();
        for (idx, score) in similar {
            if score < 0.3 {
                continue; // 过滤低相似度
            }

            let entry = &entries[idx];
            let full_path = self.project_root.join(&entry.file_path);
            
            // 读取文件生成 snippet
            let (snippet, line_number) = if let Ok(content) = fs::read_to_string(&full_path) {
                self.generate_snippet(&content, query_str)
            } else {
                ("(file not readable)".to_string(), 0)
            };

            results.push(SearchResult {
                path: entry.file_path.clone(),
                score: score * 10.0, // 归一化到类似 TF-IDF 的范围
                snippet,
                line_number,
                context: Some(SnippetContext::default()),
                match_info: Some(MatchInfo {
                    matched_terms: entry.symbols.clone(),
                    match_type: "semantic".to_string(),
                    match_quality: "vector".to_string(),
                }),
            });
        }

        Ok(results)
    }

    /// 回退方案：读取文件生成 snippet
    fn fallback_snippet(&self, path: &str, query: &str) -> (String, usize) {
        let full_path = self.project_root.join(path);
        match fs::read_to_string(&full_path) {
            Ok(content) => self.generate_snippet(&content, query),
            Err(_) => ("(file not readable)".to_string(), 0),
        }
    }

    /// 提取增强的 snippet 上下文
    fn extract_enhanced_snippet(
        &self, 
        content: &str, 
        path: &str, 
        query: &str, 
        match_line: usize
    ) -> EnhancedSnippet {
        let lines: Vec<&str> = content.lines().collect();
        let query_terms: Vec<String> = query
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        
        // 1. 生成基础 snippet
        let (code, line_num) = if match_line > 0 && match_line <= lines.len() {
            self.extract_snippet(&lines, match_line - 1)
        } else {
            self.generate_snippet(content, query)
        };
        
        // 2. 提取结构化上下文
        let context = self.extract_context(&lines, line_num.saturating_sub(1), path);
        
        // 3. 识别匹配的词项
        let matched_terms = self.find_matched_terms(&lines, line_num.saturating_sub(1), &query_terms);
        
        EnhancedSnippet {
            code,
            line_number: line_num,
            context,
            matched_terms,
        }
    }

    /// 提取代码上下文信息
    fn extract_context(&self, lines: &[&str], target_line: usize, path: &str) -> SnippetContext {
        let mut context = SnippetContext::default();
        
        // 设置模块信息 (从路径推断)
        context.module = Some(path.rsplit('/').skip(1).next().unwrap_or("").to_string());
        
        // 向上查找父级符号和可见性
        for i in (0..=target_line).rev() {
            let line = lines.get(i).unwrap_or(&"").trim();
            
            // 检测函数/方法定义
            if context.symbol_kind.is_none() {
                if line.contains("fn ") {
                    context.symbol_kind = Some("function".to_string());
                    context.signature = Some(Self::extract_signature(line));
                    if line.starts_with("pub ") {
                        context.visibility = Some("pub".to_string());
                    } else if line.starts_with("pub(crate)") {
                        context.visibility = Some("pub(crate)".to_string());
                    }
                } else if line.contains("async fn ") {
                    context.symbol_kind = Some("async function".to_string());
                    context.signature = Some(Self::extract_signature(line));
                }
            }
            
            // 检测 impl 块
            if context.parent_symbol.is_none() && line.starts_with("impl ") {
                context.parent_symbol = Some(Self::extract_impl_name(line));
            }
            
            // 检测文档注释
            if context.doc_comment.is_none() && line.starts_with("///") {
                context.doc_comment = Some(line.trim_start_matches("///").trim().to_string());
            }
            
            // 找到足够信息后停止
            if context.parent_symbol.is_some() && context.symbol_kind.is_some() {
                break;
            }
            
            // 最多向上查找 50 行
            if target_line - i > 50 {
                break;
            }
        }
        
        context
    }

    /// 提取函数签名
    fn extract_signature(line: &str) -> String {
        // 简化：取到第一个 { 或行尾
        if let Some(idx) = line.find('{') {
            line[..idx].trim().to_string()
        } else {
            line.trim().to_string()
        }
    }

    /// 提取 impl 名称
    fn extract_impl_name(line: &str) -> String {
        // "impl Foo" or "impl Trait for Foo"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            if parts.contains(&"for") {
                // impl Trait for Type
                parts.last().map(|s| s.trim_end_matches('{')).unwrap_or("").to_string()
            } else {
                // impl Type
                parts.get(1).map(|s| s.trim_end_matches('{')).unwrap_or("").to_string()
            }
        } else {
            line.to_string()
        }
    }

    /// 查找匹配的词项
    fn find_matched_terms(&self, lines: &[&str], target_line: usize, query_terms: &[String]) -> Vec<String> {
        let mut matched = Vec::new();
        
        // 检查目标行及上下文
        let start = target_line.saturating_sub(2);
        let end = (target_line + 3).min(lines.len());
        
        for i in start..end {
            if let Some(line) = lines.get(i) {
                let line_lower = line.to_lowercase();
                for term in query_terms {
                    if line_lower.contains(term) && !matched.contains(term) {
                        matched.push(term.clone());
                    }
                }
            }
        }
        
        matched
    }

    /// 符号搜索 - 精确匹配
    pub fn search_symbol(&self, symbol_name: &str) -> Result<Vec<SearchResult>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        let searcher = reader.searcher();
        let schema = self.index.schema();

        let field_path = schema.get_field("path").context("Missing path field")?;
        let field_symbols = schema.get_field("symbols").context("Missing symbols field")?;
        let field_snippet = schema.get_field("snippet").ok();

        // 使用 PhraseQuery 进行更精确的符号匹配
        let query = self.build_symbol_query(field_symbols, symbol_name);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(self.config.max_results))?;

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;

            let path_val = retrieved_doc
                .get_first(field_path)
                .and_then(|v| v.as_text())
                .unwrap_or("");

            // 符号搜索仍需读取文件来定位符号位置，但可优先使用预存 snippet 作为回退
            let (snippet, line) = {
                let full_path = self.project_root.join(path_val);
                match fs::read_to_string(&full_path) {
                    Ok(content) => self.find_symbol_definition(&content, symbol_name),
                    Err(_) => {
                        // 回退到预存 snippet
                        if let Some(field) = field_snippet {
                            if let Some(s) = retrieved_doc.get_first(field).and_then(|v| v.as_text()) {
                                (s.to_string(), 1)
                            } else {
                                ("(file not readable)".to_string(), 0)
                            }
                        } else {
                            ("(file not readable)".to_string(), 0)
                        }
                    }
                }
            };

            // 提取上下文信息 (符号搜索专用)
            let full_path = self.project_root.join(path_val);
            let context = if let Ok(content) = fs::read_to_string(&full_path) {
                let lines: Vec<&str> = content.lines().collect();
                Some(self.extract_context(&lines, line.saturating_sub(1), path_val))
            } else {
                None
            };

            results.push(SearchResult {
                path: path_val.to_string(),
                score,
                snippet,
                line_number: line,
                context,
                match_info: Some(MatchInfo {
                    matched_terms: vec![symbol_name.to_string()],
                    match_type: "symbol".to_string(),
                    match_quality: "exact".to_string(),
                }),
            });
        }

        Ok(results)
    }

    /// 构建符号查询
    fn build_symbol_query(&self, field: Field, symbol_name: &str) -> Box<dyn tantivy::query::Query> {
        // 将符号名转为小写进行匹配
        let terms: Vec<Term> = symbol_name
            .split_whitespace()
            .map(|word| Term::from_field_text(field, &word.to_lowercase()))
            .collect();

        if terms.len() == 1 {
            // 单词查询
            Box::new(tantivy::query::TermQuery::new(
                terms[0].clone(),
                tantivy::schema::IndexRecordOption::Basic,
            ))
        } else {
            // 多词短语查询
            Box::new(PhraseQuery::new(terms))
        }
    }

    /// 查找符号定义位置
    fn find_symbol_definition(&self, content: &str, symbol_name: &str) -> (String, usize) {
        let lines: Vec<&str> = content.lines().collect();
        let symbol_lower = symbol_name.to_lowercase();

        // 查找包含符号定义的行
        for (i, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();

            // 检查是否是定义行（包含 fn, class, struct, def 等关键字）
            let is_definition = (line_lower.contains("fn ")
                || line_lower.contains("class ")
                || line_lower.contains("struct ")
                || line_lower.contains("def ")
                || line_lower.contains("function ")
                || line_lower.contains("interface ")
                || line_lower.contains("trait "))
                && line_lower.contains(&symbol_lower);

            if is_definition {
                return self.extract_snippet(&lines, i);
            }
        }

        // 回退：查找任何包含符号的行
        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains(&symbol_lower) {
                return self.extract_snippet(&lines, i);
            }
        }

        // 默认返回文件开头
        let end = std::cmp::min(5, lines.len());
        (lines[0..end].join("\n"), 1)
    }

    /// 生成代码片段
    fn generate_snippet(&self, content: &str, query: &str) -> (String, usize) {
        let terms: Vec<&str> = query.split_whitespace().collect();
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let lower_line = line.to_lowercase();
            if terms.iter().any(|t| lower_line.contains(&t.to_lowercase())) {
                return self.extract_snippet(&lines, i);
            }
        }

        // 默认返回文件开头
        let end = std::cmp::min(5, lines.len());
        (lines[0..end].join("\n"), 1)
    }

    /// 提取带上下文的代码片段
    fn extract_snippet(&self, lines: &[&str], match_line: usize) -> (String, usize) {
        let start = match_line.saturating_sub(self.config.snippet_context);
        let end = std::cmp::min(match_line + self.config.snippet_context + 1, lines.len());

        let snippet_lines = &lines[start..end];
        let mut snippet = String::new();

        for (idx, l) in snippet_lines.iter().enumerate() {
            let current_line_num = start + idx + 1;
            let marker = if current_line_num == match_line + 1 {
                ">"
            } else {
                " "
            };
            snippet.push_str(&format!("{} {:4} | {}\n", marker, current_line_num, l));
        }

        (snippet, match_line + 1)
    }

    /// 扩展查询词项
    /// 
    /// 将常见中文术语映射到英文等价词，提升跨语言搜索能力
    fn expand_query(query: &str) -> String {
        let mut expanded = query.to_string();
        
        // 中英文术语映射
        let expansions = [
            // 认证相关
            ("登录", "login auth authenticate"),
            ("登陆", "login auth"),
            ("认证", "auth authenticate authentication"),
            ("授权", "authorize authorization"),
            ("权限", "permission role access"),
            ("密码", "password credential"),
            ("用户", "user account"),
            
            // 功能相关
            ("搜索", "search find query"),
            ("查询", "query search find"),
            ("配置", "config configuration settings"),
            ("设置", "settings config preferences"),
            ("保存", "save store persist"),
            ("删除", "delete remove"),
            ("更新", "update modify"),
            ("创建", "create new add"),
            ("获取", "get fetch retrieve"),
            
            // 架构相关
            ("服务", "service"),
            ("处理", "handler handle process"),
            ("请求", "request req"),
            ("响应", "response res"),
            ("错误", "error err"),
            ("日志", "log logger logging"),
            ("缓存", "cache"),
            ("数据库", "database db"),
        ];
        
        for (cn, en) in expansions.iter() {
            if query.contains(cn) {
                expanded.push(' ');
                expanded.push_str(en);
            }
        }
        
        expanded
    }
}
