//! 记忆拦截器
//!
//! 在交互流程中自动召回和记录代码修改记忆

use crate::mcp::tools::memory::{ChangeTracker, CodeChangeMemory};
use crate::neurospec::services::embedding::{find_similar, is_embedding_available};

/// 记忆拦截器
/// 
/// 在用户交互时自动：
/// - 前置：召回相关的代码修改记忆
/// - 后置：记录新的代码修改（需要 AI 配合）
pub struct MemoryInterceptor {
    pub tracker: Option<ChangeTracker>,
    #[allow(dead_code)]
    project_path: Option<String>,
}

impl MemoryInterceptor {
    /// 创建新的拦截器
    pub fn new(project_path: Option<&str>) -> Self {
        let tracker = project_path.and_then(|p| ChangeTracker::new(p).ok());
        
        Self {
            tracker,
            project_path: project_path.map(|s| s.to_string()),
        }
    }

    /// 尝试从环境中自动检测项目路径
    pub fn auto_detect() -> Self {
        let project_path = Self::detect_git_root();
        Self::new(project_path.as_deref())
    }

    /// 检测 Git 根目录
    fn detect_git_root() -> Option<String> {
        let cwd = std::env::current_dir().ok()?;
        let mut current = cwd.as_path();
        
        loop {
            if current.join(".git").exists() {
                return Some(current.to_string_lossy().to_string());
            }
            current = current.parent()?;
        }
    }

    // ========================================================================
    // 前置处理：自动召回相关记忆
    // ========================================================================

    /// 根据用户消息召回相关的代码修改记忆
    /// 
    /// 返回格式化的记忆提示文本，可以附加到 AI 可见的上下文中
    pub fn recall_relevant_memories(&self, user_message: &str, limit: usize) -> Option<String> {
        let tracker = self.tracker.as_ref()?;
        
        // 从消息中提取可能的文件路径
        let file_paths = self.extract_file_paths(user_message);
        
        // 搜索相关记忆
        let memories = tracker.find_relevant_changes(&file_paths, user_message, limit).ok()?;
        
        if memories.is_empty() {
            return None;
        }

        // 格式化输出
        Some(self.format_memories_as_context(&memories))
    }

    /// 使用嵌入模型进行语义召回（异步版本）
    /// 
    /// 如果嵌入服务可用，则使用语义匹配；否则回退到关键词匹配
    pub async fn recall_with_embedding(&self, user_message: &str, limit: usize) -> Option<String> {
        let tracker = self.tracker.as_ref()?;
        
        // 获取所有记忆
        let all_memories = tracker.get_all_changes().ok()?;
        if all_memories.is_empty() {
            return None;
        }

        // 检查嵌入服务是否可用
        if is_embedding_available() {
            // 构建候选摘要列表
            let summaries: Vec<String> = all_memories.iter()
                .map(|m| format!("{} {}", m.summary, m.user_intent))
                .collect();
            
            // 使用嵌入进行语义匹配
            if let Some(similar) = find_similar(user_message, &summaries, limit).await {
                let matched_memories: Vec<&CodeChangeMemory> = similar.iter()
                    .filter(|(_, score)| *score > 0.5) // 相似度阈值
                    .map(|(idx, _)| &all_memories[*idx])
                    .collect();
                
                if !matched_memories.is_empty() {
                    let owned: Vec<CodeChangeMemory> = matched_memories.into_iter().cloned().collect();
                    return Some(self.format_memories_as_context(&owned));
                }
            }
        }
        
        // 回退到关键词匹配
        self.recall_relevant_memories(user_message, limit)
    }

    /// 从用户消息中提取可能的文件路径
    fn extract_file_paths(&self, message: &str) -> Vec<String> {
        let mut paths = Vec::new();
        
        // 简单的路径模式匹配
        for word in message.split_whitespace() {
            // 检查是否像文件路径
            if word.contains('/') || word.contains('\\') || word.contains('.') {
                // 检查常见代码文件扩展名
                let extensions = [".rs", ".ts", ".js", ".vue", ".py", ".go", ".java", ".tsx", ".jsx"];
                if extensions.iter().any(|ext| word.ends_with(ext)) {
                    paths.push(word.to_string());
                }
            }
        }
        
        paths
    }

    /// 将记忆格式化为上下文提示
    fn format_memories_as_context(&self, memories: &[CodeChangeMemory]) -> String {
        let mut output = String::new();
        
        output.push_str("\n\n---\n");
        output.push_str("## 📚 相关修改历史（自动召回）\n\n");
        output.push_str("以下是与当前任务相关的历史修改记录，供参考：\n\n");
        
        for (i, mem) in memories.iter().enumerate() {
            output.push_str(&format!("### {}. {}\n", i + 1, mem.summary));
            output.push_str(&format!("- **类型**: {}\n", mem.change_type));
            output.push_str(&format!("- **文件**: {}\n", mem.file_paths.join(", ")));
            output.push_str(&format!("- **意图**: {}\n", mem.user_intent));
            
            if let Some(ref diff) = mem.diff_snippet {
                if diff.len() < 500 {
                    output.push_str(&format!("```\n{}\n```\n", diff));
                }
            }
            output.push('\n');
        }
        
        output.push_str("---\n\n");
        output
    }

    // ========================================================================
    // 后置处理：检测并记录修改
    // ========================================================================

    /// 检测消息中是否包含代码修改报告
    /// 
    /// AI 可以在响应中包含特殊标记来报告修改：
    /// ```
    /// [CHANGE_REPORT]
    /// type: bug-fix
    /// files: src/auth/handler.rs, src/auth/token.rs
    /// symbols: handle_login, refresh_token
    /// summary: 修复了 token 刷新逻辑
    /// [/CHANGE_REPORT]
    /// ```
    pub fn detect_and_record_change(&self, ai_response: &str, user_intent: &str) -> Option<String> {
        let tracker = self.tracker.as_ref()?;
        
        // 解析 CHANGE_REPORT 标记
        let report = self.parse_change_report(ai_response)?;
        
        // 记录修改
        let id = tracker.record_change(
            report.change_type,
            report.files,
            report.symbols,
            report.summary,
            user_intent.to_string(),
        ).ok()?;
        
        Some(id)
    }

    /// 解析 AI 响应中的修改报告
    fn parse_change_report(&self, response: &str) -> Option<ChangeReport> {
        let start_tag = "[CHANGE_REPORT]";
        let end_tag = "[/CHANGE_REPORT]";
        
        let start = response.find(start_tag)? + start_tag.len();
        let end = response.find(end_tag)?;
        
        if start >= end {
            return None;
        }
        
        let content = &response[start..end];
        let mut report = ChangeReport::default();
        
        for line in content.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_lowercase();
                let value = value.trim();
                
                match key.as_str() {
                    "type" => {
                        report.change_type = match value {
                            "bug-fix" | "bugfix" => crate::mcp::tools::memory::ChangeType::BugFix,
                            "feature" => crate::mcp::tools::memory::ChangeType::Feature,
                            "refactor" => crate::mcp::tools::memory::ChangeType::Refactor,
                            "optimization" => crate::mcp::tools::memory::ChangeType::Optimization,
                            "documentation" | "doc" => crate::mcp::tools::memory::ChangeType::Documentation,
                            _ => crate::mcp::tools::memory::ChangeType::Other,
                        };
                    }
                    "files" => {
                        report.files = value.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "symbols" => {
                        report.symbols = value.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "summary" => {
                        report.summary = value.to_string();
                    }
                    _ => {}
                }
            }
        }
        
        // 验证必要字段
        if report.summary.is_empty() || report.files.is_empty() {
            return None;
        }
        
        Some(report)
    }

    // ========================================================================
    // 维护
    // ========================================================================

    /// 执行记忆维护（衰减 + 清理）
    pub fn maintenance(&self) -> Option<(usize, usize)> {
        self.tracker.as_ref()?.maintenance().ok()
    }
}

/// 修改报告结构
#[derive(Default)]
struct ChangeReport {
    change_type: crate::mcp::tools::memory::ChangeType,
    files: Vec<String>,
    symbols: Vec<String>,
    summary: String,
}

// ============================================================================
// 全局拦截器实例
// ============================================================================

use std::sync::OnceLock;
use std::sync::Mutex;

static GLOBAL_INTERCEPTOR: OnceLock<Mutex<MemoryInterceptor>> = OnceLock::new();

/// 获取或初始化全局拦截器
pub fn get_interceptor() -> &'static Mutex<MemoryInterceptor> {
    GLOBAL_INTERCEPTOR.get_or_init(|| {
        Mutex::new(MemoryInterceptor::auto_detect())
    })
}

/// 自动召回相关记忆（便捷函数，同步版本）
pub fn auto_recall(user_message: &str) -> Option<String> {
    let interceptor = get_interceptor().lock().ok()?;
    interceptor.recall_relevant_memories(user_message, 3)
}

/// 自动召回相关记忆（便捷函数，异步版本，使用嵌入模型）
pub async fn auto_recall_async(user_message: &str) -> Option<String> {
    // 检查嵌入服务是否可用
    if is_embedding_available() {
        // 先获取所有记忆（在锁内，立即释放）
        let (all_memories, fallback_result) = {
            let interceptor = get_interceptor().lock().ok()?;
            let tracker = interceptor.tracker.as_ref()?;
            let memories = tracker.get_all_changes().ok()?;
            let fallback = interceptor.recall_relevant_memories(user_message, 3);
            (memories, fallback)
        }; // 锁在这里释放
        
        if all_memories.is_empty() {
            return fallback_result;
        }
        
        // 异步调用嵌入服务（锁已释放）
        let summaries: Vec<String> = all_memories.iter()
            .map(|m| format!("{} {}", m.summary, m.user_intent))
            .collect();
        
        if let Some(similar) = find_similar(user_message, &summaries, 3).await {
            let matched: Vec<CodeChangeMemory> = similar.iter()
                .filter(|(_, score)| *score > 0.5)
                .map(|(idx, _)| all_memories[*idx].clone())
                .collect();
            
            if !matched.is_empty() {
                // 直接格式化，不需要锁
                return Some(format_memories_standalone(&matched));
            }
        }
        
        // 回退到关键词匹配结果
        fallback_result
    } else {
        let interceptor = get_interceptor().lock().ok()?;
        interceptor.recall_relevant_memories(user_message, 3)
    }
}

/// 独立的格式化函数（不需要锁）
fn format_memories_standalone(memories: &[CodeChangeMemory]) -> String {
    let mut output = String::new();
    
    output.push_str("\n\n---\n");
    output.push_str("## 📚 相关修改历史（语义匹配）\n\n");
    
    for (i, mem) in memories.iter().enumerate() {
        output.push_str(&format!("### {}. {}\n", i + 1, mem.summary));
        output.push_str(&format!("- **类型**: {}\n", mem.change_type));
        output.push_str(&format!("- **文件**: {}\n", mem.file_paths.join(", ")));
        output.push_str(&format!("- **意图**: {}\n", mem.user_intent));
        output.push('\n');
    }
    
    output.push_str("---\n\n");
    output
}

/// 自动记录修改（便捷函数）
pub fn auto_record(ai_response: &str, user_intent: &str) -> Option<String> {
    let interceptor = get_interceptor().lock().ok()?;
    interceptor.detect_and_record_change(ai_response, user_intent)
}
