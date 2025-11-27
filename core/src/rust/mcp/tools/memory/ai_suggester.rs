//! AI 记忆建议器

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::types::{MemoryCategory, MemoryEntry};

#[derive(Debug, Clone)]
pub struct MemorySuggester {
    memory_stats: HashMap<String, MemoryUsageStats>,
    #[allow(dead_code)] // Phase 2: 用于模式检测优化
    detected_patterns: HashMap<String, u32>,
    recent_conversations: Vec<String>,
    feedback_history: HashMap<String, bool>,
    ignored_suggestions: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsageStats {
    pub memory_id: String,
    pub usage_count: u32,
    pub last_used_at: chrono::DateTime<Utc>,
    pub contributed_to_answers: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySuggestion {
    pub id: String,
    pub content: String,
    pub category: MemoryCategory,
    pub confidence: f32,
    pub reason: String,
    pub keywords: Vec<String>,
    pub suggested_at: chrono::DateTime<Utc>,
    #[serde(default)]
    pub source: SuggestionSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SuggestionSource {
    #[default]
    KeywordMatch,
    ExplicitRequest,
    RepeatedContent,
    UserCorrection,
}

#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub messages: Vec<String>,
    pub project_context: Option<String>,
    pub language: Option<String>,
}


impl MemorySuggester {
    pub fn new() -> Self {
        Self {
            memory_stats: HashMap::new(),
            detected_patterns: HashMap::new(),
            recent_conversations: Vec::new(),
            feedback_history: HashMap::new(),
            ignored_suggestions: HashSet::new(),
        }
    }

    pub fn detect_pattern(&self, context: &ConversationContext) -> Vec<MemorySuggestion> {
        let mut suggestions = Vec::new();
        if let Some(s) = self.detect_explicit_remember(&context.messages) { suggestions.push(s); }
        if let Some(s) = self.detect_user_correction(&context.messages) { suggestions.push(s); }
        if let Some(s) = self.detect_preference(&context.messages) { suggestions.push(s); }
        if let Some(s) = self.detect_coding_standards(&context.messages) { suggestions.push(s); }
        if let Some(s) = self.detect_best_practices(&context.messages) { suggestions.push(s); }
        suggestions.retain(|s| !self.ignored_suggestions.contains(&s.id));
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        suggestions
    }

    fn detect_explicit_remember(&self, messages: &[String]) -> Option<MemorySuggestion> {
        // 扩展触发词列表
        let triggers = [
            // 明确记忆请求
            "请记住", "记住这个", "remember", "记住",
            // 规则/约定表达
            "以后都要", "每次都", "总是", "一定要", "必须",
            "下次", "规定", "约定", "统一使用",
            // 禁止/避免表达
            "不要", "禁止", "避免", "不允许", "不能",
        ];
        let text = messages.join(" ");
        for trigger in &triggers {
            if let Some(pos) = text.to_lowercase().find(&trigger.to_lowercase()) {
                let after_trigger = &text[pos..];
                // 使用句子边界检测，提取到句号/换行/问号/感叹号为止
                let content = Self::extract_sentence(after_trigger);
                if content.len() > trigger.len() + 3 {
                    return Some(MemorySuggestion {
                        id: format!("explicit_{:08x}", Self::hash_str(&content)),
                        content,
                        category: MemoryCategory::Rule,
                        confidence: 0.95,
                        reason: "用户明确要求记住".to_string(),
                        keywords: vec![trigger.to_string()],
                        suggested_at: Utc::now(),
                        source: SuggestionSource::ExplicitRequest,
                    });
                }
            }
        }
        None
    }
    
    /// 提取完整句子（到句子结束符为止）
    fn extract_sentence(text: &str) -> String {
        // 句子结束符
        let end_markers = ['。', '.', '！', '!', '？', '?', '\n'];
        
        // 找到第一个结束符的位置
        let end_pos = text.char_indices()
            .find(|(_, c)| end_markers.contains(c))
            .map(|(i, _)| i)
            .unwrap_or(text.len().min(200)); // 最大 200 字符
        
        text[..end_pos].trim().to_string()
    }

    fn detect_user_correction(&self, messages: &[String]) -> Option<MemorySuggestion> {
        let patterns = [("不对", "纠正"), ("错了", "纠正"), ("应该是", "正确做法")];
        let text = messages.join(" ");
        for (trigger, reason) in &patterns {
            if text.to_lowercase().contains(&trigger.to_lowercase()) {
                return Some(MemorySuggestion {
                    id: format!("correction_{:08x}", Self::hash_str(&text)),
                    content: format!("用户纠正: {}", &text[..text.len().min(100)]),
                    category: MemoryCategory::Rule,
                    confidence: 0.85,
                    reason: format!("检测到用户{}行为", reason),
                    keywords: vec![trigger.to_string()],
                    suggested_at: Utc::now(),
                    source: SuggestionSource::UserCorrection,
                });
            }
        }
        None
    }

    fn detect_coding_standards(&self, messages: &[String]) -> Option<MemorySuggestion> {
        let patterns = [("缩进", &["空格", "缩进", "indent"][..]), ("命名", &["camelCase", "snake_case"][..])];
        let text = messages.join(" ").to_lowercase();
        for (name, keywords) in &patterns {
            if keywords.iter().any(|k| text.contains(&k.to_lowercase())) {
                return Some(MemorySuggestion {
                    id: format!("std_{}", name),
                    content: format!("项目{}规范", name),
                    category: MemoryCategory::Rule,
                    confidence: 0.75,
                    reason: format!("检测到{}相关讨论", name),
                    keywords: keywords.iter().map(|s| s.to_string()).collect(),
                    suggested_at: Utc::now(),
                    source: SuggestionSource::KeywordMatch,
                });
            }
        }
        None
    }

    fn detect_best_practices(&self, messages: &[String]) -> Option<MemorySuggestion> {
        let keywords = ["最佳实践", "best practice", "建议", "应该", "避免"];
        let text = messages.join(" ").to_lowercase();
        if keywords.iter().any(|k| text.contains(&k.to_lowercase())) {
            return Some(MemorySuggestion {
                id: "best_practices".to_string(),
                content: "项目最佳实践".to_string(),
                category: MemoryCategory::Pattern,
                confidence: 0.6,
                reason: "检测到最佳实践相关讨论".to_string(),
                keywords: keywords.iter().map(|s| s.to_string()).collect(),
                suggested_at: Utc::now(),
                source: SuggestionSource::KeywordMatch,
            });
        }
        None
    }

    /// 检测用户偏好表达
    fn detect_preference(&self, messages: &[String]) -> Option<MemorySuggestion> {
        let triggers = [
            ("我喜欢", "用户偏好"),
            ("我偏好", "用户偏好"),
            ("我习惯", "用户习惯"),
            ("我更倾向", "用户倾向"),
            ("我通常", "用户习惯"),
            ("我一般", "用户习惯"),
        ];
        let text = messages.join(" ");
        for (trigger, reason) in &triggers {
            if let Some(pos) = text.to_lowercase().find(&trigger.to_lowercase()) {
                let content = text[pos..].trim();
                // 提取到句号或换行为止
                let end_pos = content.find(|c| c == '。' || c == '\n' || c == '.').unwrap_or(content.len().min(150));
                let extracted = &content[..end_pos];
                if extracted.len() > trigger.len() + 3 {
                    return Some(MemorySuggestion {
                        id: format!("pref_{:08x}", Self::hash_str(extracted)),
                        content: extracted.to_string(),
                        category: MemoryCategory::Preference,
                        confidence: 0.85,
                        reason: format!("检测到{}", reason),
                        keywords: vec![trigger.to_string()],
                        suggested_at: Utc::now(),
                        source: SuggestionSource::KeywordMatch,
                    });
                }
            }
        }
        None
    }

    fn hash_str(s: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        h.finish()
    }

    pub fn record_feedback(&mut self, id: &str, accepted: bool) {
        self.feedback_history.insert(id.to_string(), accepted);
        if !accepted { self.ignored_suggestions.insert(id.to_string()); }
    }

    pub fn record_memory_usage(&mut self, memory_id: &str) {
        let stats = self.memory_stats.entry(memory_id.to_string()).or_insert_with(|| MemoryUsageStats {
            memory_id: memory_id.to_string(), usage_count: 0, last_used_at: Utc::now(), contributed_to_answers: 0,
        });
        stats.usage_count += 1;
        stats.last_used_at = Utc::now();
    }

    pub fn get_memory_stats(&self, memory_id: &str) -> Option<&MemoryUsageStats> { self.memory_stats.get(memory_id) }

    pub fn get_frequently_used_memories(&self, limit: usize) -> Vec<&MemoryUsageStats> {
        let mut stats: Vec<_> = self.memory_stats.values().collect();
        stats.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        stats.into_iter().take(limit).collect()
    }

    pub fn add_conversation(&mut self, message: String) {
        self.recent_conversations.push(message);
        if self.recent_conversations.len() > 20 { self.recent_conversations.remove(0); }
    }

    pub fn get_related_memories(&self, query: &str, existing: &[MemoryEntry]) -> Vec<(MemoryEntry, f32)> {
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut result = Vec::new();
        for mem in existing {
            let mem_lower = mem.content.to_lowercase();
            let mut score = words.iter().filter(|w| mem_lower.contains(*w)).count() as f32;
            score += match mem.category { MemoryCategory::Rule => 0.5, MemoryCategory::Pattern => 0.3, MemoryCategory::Preference => 0.2, MemoryCategory::Context => 0.1 };
            if score > 0.0 { result.push((mem.clone(), score)); }
        }
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    pub fn generate_suggestion_summary(&self, suggestions: &[MemorySuggestion]) -> String {
        if suggestions.is_empty() { return "暂无记忆建议".to_string(); }
        let mut s = format!("检测到 {} 条潜在记忆:\n", suggestions.len());
        for (i, sg) in suggestions.iter().enumerate() {
            s.push_str(&format!("{}. {} ({:.0}%)\n", i + 1, sg.content, sg.confidence * 100.0));
        }
        s
    }
}

impl Default for MemorySuggester { fn default() -> Self { Self::new() } }

/// 代码模式分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePatternAnalysis {
    pub naming_convention: Option<NamingConvention>,
    pub error_handling: Option<ErrorHandlingPattern>,
    pub logging_style: Option<String>,
    pub doc_comment_ratio: f32,
    pub suggestions: Vec<MemorySuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NamingConvention {
    SnakeCase,
    CamelCase,
    PascalCase,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandlingPattern {
    ResultBased,      // Rust Result<T, E>
    TryCatch,         // try-catch
    ExceptionBased,   // Python exceptions
    Mixed,
}

/// 代码模式分析器
pub struct CodePatternAnalyzer;

impl CodePatternAnalyzer {
    /// 分析项目代码模式
    pub fn analyze_project(project_path: &str) -> anyhow::Result<CodePatternAnalysis> {
        use std::fs;
        use std::path::Path;
        use walkdir::WalkDir;

        let root = Path::new(project_path);
        let mut snake_count = 0;
        let mut camel_count = 0;
        let mut result_count = 0;
        let mut try_catch_count = 0;
        let mut unwrap_count = 0;
        let mut _expect_count = 0;
        let mut log_macro_count = 0;
        let mut println_count = 0;
        let mut doc_comment_lines = 0;
        let mut total_lines = 0;
        let mut files_analyzed = 0;

        let walker = WalkDir::new(root).into_iter();
        for entry in walker.filter_entry(|e| !is_ignored_dir(e)) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            
            // 只分析代码文件
            if !matches!(ext, "rs" | "ts" | "js" | "py" | "tsx" | "jsx") {
                continue;
            }

            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            files_analyzed += 1;
            total_lines += content.lines().count();

            // 分析命名规范
            for line in content.lines() {
                // 检测 snake_case (fn xxx_xxx, let xxx_xxx)
                if line.contains("fn ") || line.contains("let ") || line.contains("def ") {
                    if line.contains('_') && !line.contains("__") {
                        snake_count += 1;
                    }
                }
                // 检测 camelCase
                if line.contains("function ") || line.contains("const ") || line.contains("let ") {
                    // 简单检测：小写开头后跟大写
                    let words: Vec<&str> = line.split_whitespace().collect();
                    for word in words {
                        if word.chars().next().map(|c| c.is_lowercase()).unwrap_or(false)
                            && word.chars().any(|c| c.is_uppercase())
                        {
                            camel_count += 1;
                        }
                    }
                }

                // 分析错误处理
                if line.contains("Result<") || line.contains("-> Result") {
                    result_count += 1;
                }
                if line.contains("try {") || line.contains("try:") || line.contains(".catch(") {
                    try_catch_count += 1;
                }
                if line.contains(".unwrap()") {
                    unwrap_count += 1;
                }
                if line.contains(".expect(") {
                    _expect_count += 1;
                }

                // 分析日志风格
                if line.contains("log::") || line.contains("tracing::") || line.contains("log!") {
                    log_macro_count += 1;
                }
                if line.contains("println!") || line.contains("console.log") || line.contains("print(") {
                    println_count += 1;
                }

                // 分析文档注释
                if line.trim().starts_with("///") || line.trim().starts_with("/**") || line.trim().starts_with("\"\"\"") {
                    doc_comment_lines += 1;
                }
            }
        }

        // 生成分析结果
        let naming_convention = if snake_count > camel_count * 2 {
            Some(NamingConvention::SnakeCase)
        } else if camel_count > snake_count * 2 {
            Some(NamingConvention::CamelCase)
        } else if snake_count > 0 || camel_count > 0 {
            Some(NamingConvention::Mixed)
        } else {
            None
        };

        let error_handling = if result_count > try_catch_count * 2 {
            Some(ErrorHandlingPattern::ResultBased)
        } else if try_catch_count > result_count * 2 {
            Some(ErrorHandlingPattern::TryCatch)
        } else if result_count > 0 || try_catch_count > 0 {
            Some(ErrorHandlingPattern::Mixed)
        } else {
            None
        };

        let logging_style = if log_macro_count > println_count * 2 {
            Some("结构化日志 (log/tracing)".to_string())
        } else if println_count > log_macro_count * 2 {
            Some("println/console.log".to_string())
        } else if log_macro_count > 0 || println_count > 0 {
            Some("混合".to_string())
        } else {
            None
        };

        let doc_comment_ratio = if total_lines > 0 {
            doc_comment_lines as f32 / total_lines as f32
        } else {
            0.0
        };

        // 生成建议
        let mut suggestions = Vec::new();

        if let Some(ref naming) = naming_convention {
            let content = match naming {
                NamingConvention::SnakeCase => "项目使用 snake_case 命名规范",
                NamingConvention::CamelCase => "项目使用 camelCase 命名规范",
                NamingConvention::PascalCase => "项目使用 PascalCase 命名规范",
                NamingConvention::Mixed => "项目命名规范混合使用",
            };
            suggestions.push(MemorySuggestion {
                id: "pattern_naming".to_string(),
                content: content.to_string(),
                category: MemoryCategory::Pattern,
                confidence: 0.8,
                reason: format!("分析 {} 个文件得出", files_analyzed),
                keywords: vec!["命名".to_string(), "规范".to_string()],
                suggested_at: Utc::now(),
                source: SuggestionSource::KeywordMatch,
            });
        }

        if let Some(ref err) = error_handling {
            let content = match err {
                ErrorHandlingPattern::ResultBased => "项目使用 Result 类型进行错误处理",
                ErrorHandlingPattern::TryCatch => "项目使用 try-catch 进行错误处理",
                ErrorHandlingPattern::ExceptionBased => "项目使用异常进行错误处理",
                ErrorHandlingPattern::Mixed => "项目错误处理方式混合",
            };
            suggestions.push(MemorySuggestion {
                id: "pattern_error".to_string(),
                content: content.to_string(),
                category: MemoryCategory::Pattern,
                confidence: 0.75,
                reason: format!("Result: {}, try-catch: {}", result_count, try_catch_count),
                keywords: vec!["错误处理".to_string()],
                suggested_at: Utc::now(),
                source: SuggestionSource::KeywordMatch,
            });
        }

        // unwrap 使用警告
        if unwrap_count > 10 {
            suggestions.push(MemorySuggestion {
                id: "pattern_unwrap_warning".to_string(),
                content: format!("项目中有 {} 处 .unwrap() 调用，建议使用 ? 或 .expect() 替代", unwrap_count),
                category: MemoryCategory::Rule,
                confidence: 0.7,
                reason: "代码质量建议".to_string(),
                keywords: vec!["unwrap".to_string(), "错误处理".to_string()],
                suggested_at: Utc::now(),
                source: SuggestionSource::KeywordMatch,
            });
        }

        Ok(CodePatternAnalysis {
            naming_convention,
            error_handling,
            logging_style,
            doc_comment_ratio,
            suggestions,
        })
    }

    /// 格式化分析结果
    pub fn format_analysis(analysis: &CodePatternAnalysis) -> String {
        let mut output = String::new();
        output.push_str("# 🔍 代码模式分析\n\n");

        output.push_str("## 检测结果\n\n");

        if let Some(ref naming) = analysis.naming_convention {
            output.push_str(&format!("- **命名规范**: {:?}\n", naming));
        }

        if let Some(ref err) = analysis.error_handling {
            output.push_str(&format!("- **错误处理**: {:?}\n", err));
        }

        if let Some(ref log) = analysis.logging_style {
            output.push_str(&format!("- **日志风格**: {}\n", log));
        }

        output.push_str(&format!("- **文档注释比例**: {:.1}%\n", analysis.doc_comment_ratio * 100.0));

        if !analysis.suggestions.is_empty() {
            output.push_str("\n## 建议记忆\n\n");
            for (i, s) in analysis.suggestions.iter().enumerate() {
                let icon = match s.category {
                    MemoryCategory::Rule => "🔵",
                    MemoryCategory::Pattern => "🟡",
                    MemoryCategory::Preference => "🟢",
                    MemoryCategory::Context => "⚪",
                };
                output.push_str(&format!("{}. {} {} (置信度: {:.0}%)\n", i + 1, icon, s.content, s.confidence * 100.0));
            }
        }

        output
    }
}

fn is_ignored_dir(entry: &walkdir::DirEntry) -> bool {
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
