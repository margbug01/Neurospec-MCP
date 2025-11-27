//! 上下文编排器
//! 
//! 在消息发送给 AI 之前，自动注入相关上下文：
//! - 项目信息
//! - 相关记忆
//! - 相关代码片段

use std::path::PathBuf;

use crate::mcp::tools::memory::{MemoryManager, MemoryCategory};
use crate::log_important;

/// 上下文编排配置
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// 是否启用自动上下文注入
    pub enabled: bool,
    /// 最大记忆数量
    pub max_memories: usize,
    /// 最大代码片段数量
    pub max_code_snippets: usize,
    /// 是否显示上下文来源
    pub show_source: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_memories: 5,
            max_code_snippets: 3,
            show_source: false,
        }
    }
}

/// 增强后的上下文
#[derive(Debug, Clone)]
pub struct EnhancedContext {
    /// 项目信息
    pub project_info: Option<ProjectInfo>,
    /// 相关记忆
    pub memories: Vec<RelevantMemory>,
    /// 相关代码
    pub code_snippets: Vec<CodeSnippet>,
}

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub name: String,
    pub project_type: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct RelevantMemory {
    pub content: String,
    pub category: String,
    pub relevance: f32,
}

#[derive(Debug, Clone)]
pub struct CodeSnippet {
    pub path: String,
    pub snippet: String,
    pub relevance: f32,
}

/// 上下文编排器
pub struct ContextOrchestrator {
    config: OrchestratorConfig,
}

impl ContextOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(OrchestratorConfig::default())
    }

    /// 检测项目路径
    /// 
    /// 检测策略（优先级从高到低）：
    /// 1. 从配置文件加载已保存的项目路径
    /// 2. 从当前工作目录向上查找 .git 目录
    fn detect_project_path() -> Option<String> {
        // 1. 尝试从配置文件加载
        if let Some(saved_path) = Self::load_saved_project_path() {
            let root = std::path::PathBuf::from(&saved_path);
            if root.exists() {
                return Some(saved_path);
            }
        }
        
        // 2. 从当前工作目录查找
        let cwd = std::env::current_dir().ok()?;
        let mut current = cwd.as_path();
        
        loop {
            if current.join(".git").exists() {
                return Some(current.to_string_lossy().to_string());
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }
        None
    }
    
    /// 从配置文件加载已保存的项目路径
    fn load_saved_project_path() -> Option<String> {
        let config_path = dirs::data_dir()?.join("neurospec").join("project_config.json");
        
        if !config_path.exists() {
            return None;
        }
        
        let content = std::fs::read_to_string(&config_path).ok()?;
        let config: serde_json::Value = serde_json::from_str(&content).ok()?;
        config.get("project_path")?.as_str().map(String::from)
    }

    /// 提取消息中的关键词
    fn extract_keywords(message: &str) -> Vec<String> {
        // 简单的关键词提取：分词 + 过滤停用词
        let stop_words = [
            "的", "是", "在", "有", "和", "了", "我", "你", "他", "她", "它",
            "这", "那", "什么", "怎么", "如何", "为什么", "请", "帮", "能",
            "the", "a", "an", "is", "are", "was", "were", "be", "been",
            "have", "has", "had", "do", "does", "did", "will", "would",
            "can", "could", "should", "may", "might", "must", "to", "of",
            "in", "on", "at", "for", "with", "by", "from", "as", "this",
            "that", "it", "i", "you", "he", "she", "we", "they", "my",
            "your", "his", "her", "its", "our", "their", "what", "how",
            "why", "when", "where", "which", "who", "please", "help", "me",
        ];

        message
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() > 2)
            .filter(|w| !stop_words.contains(&w.to_lowercase().as_str()))
            .map(|s| s.to_string())
            .take(10)
            .collect()
    }

    /// 获取增强上下文
    pub fn get_enhanced_context(&self, message: &str) -> EnhancedContext {
        if !self.config.enabled {
            return EnhancedContext {
                project_info: None,
                memories: vec![],
                code_snippets: vec![],
            };
        }

        let project_path = Self::detect_project_path();
        let keywords = Self::extract_keywords(message);

        log_important!(info, "Context orchestrator: keywords={:?}", keywords);

        // 获取项目信息
        let project_info = project_path.as_ref().map(|path| {
            let p = PathBuf::from(path);
            ProjectInfo {
                name: p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
                project_type: Self::detect_project_type(path),
                path: path.clone(),
            }
        });

        // 获取相关记忆
        let memories = if let Some(ref path) = project_path {
            self.get_relevant_memories(path, &keywords)
        } else {
            vec![]
        };

        EnhancedContext {
            project_info,
            memories,
            code_snippets: vec![], // 代码搜索可选，避免延迟
        }
    }

    /// 检测项目类型
    fn detect_project_type(path: &str) -> String {
        let root = PathBuf::from(path);
        
        if root.join("Cargo.toml").exists() {
            "Rust".to_string()
        } else if root.join("package.json").exists() {
            "Node.js/TypeScript".to_string()
        } else if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
            "Python".to_string()
        } else if root.join("go.mod").exists() {
            "Go".to_string()
        } else if root.join("pom.xml").exists() || root.join("build.gradle").exists() {
            "Java".to_string()
        } else {
            "Unknown".to_string()
        }
    }

    /// 获取相关记忆
    fn get_relevant_memories(&self, project_path: &str, keywords: &[String]) -> Vec<RelevantMemory> {
        let manager = match MemoryManager::new(project_path) {
            Ok(m) => m,
            Err(_) => return vec![],
        };

        // 获取所有记忆
        let all_memories = match manager.list_memories(None, 1, 50) {
            Ok(result) => result.memories,
            Err(_) => return vec![],
        };

        // 计算相关性并排序
        let mut scored: Vec<_> = all_memories
            .into_iter()
            .map(|mem| {
                let content_lower = mem.content.to_lowercase();
                let keyword_matches = keywords
                    .iter()
                    .filter(|k| content_lower.contains(&k.to_lowercase()))
                    .count();
                
                let category_boost = match mem.category {
                    MemoryCategory::Rule => 1.3,
                    MemoryCategory::Pattern => 1.2,
                    MemoryCategory::Preference => 1.1,
                    MemoryCategory::Context => 1.0,
                };

                let relevance = (keyword_matches as f32 * 0.3 + 0.2) * category_boost;

                RelevantMemory {
                    content: mem.content,
                    category: format!("{:?}", mem.category),
                    relevance,
                }
            })
            .collect();

        // 按相关性排序
        scored.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));

        // 返回前 N 条
        scored.into_iter().take(self.config.max_memories).collect()
    }

    /// 格式化上下文为文本
    pub fn format_context(&self, ctx: &EnhancedContext) -> Option<String> {
        if ctx.project_info.is_none() && ctx.memories.is_empty() && ctx.code_snippets.is_empty() {
            return None;
        }

        let mut output = String::new();
        output.push_str("\n\n---\n📋 **系统上下文** (自动注入)\n\n");

        // 项目信息
        if let Some(ref info) = ctx.project_info {
            output.push_str(&format!("**项目**: {} ({})\n", info.name, info.project_type));
        }

        // 相关记忆
        if !ctx.memories.is_empty() {
            output.push_str("\n**相关记忆**:\n");
            for mem in &ctx.memories {
                let icon = match mem.category.as_str() {
                    "Rule" => "🔵",
                    "Pattern" => "🟡",
                    "Preference" => "🟢",
                    _ => "⚪",
                };
                output.push_str(&format!("- {} {}\n", icon, mem.content));
            }
        }

        // 代码片段
        if !ctx.code_snippets.is_empty() {
            output.push_str("\n**相关代码**:\n");
            for snippet in &ctx.code_snippets {
                output.push_str(&format!("```\n// {}\n{}\n```\n", snippet.path, snippet.snippet));
            }
        }

        output.push_str("---\n");

        Some(output)
    }

    /// 增强消息
    pub fn enhance_message(&self, message: &str) -> String {
        let ctx = self.get_enhanced_context(message);
        
        if let Some(context_text) = self.format_context(&ctx) {
            format!("{}{}", message, context_text)
        } else {
            message.to_string()
        }
    }
}

// 全局编排器实例
lazy_static::lazy_static! {
    static ref GLOBAL_ORCHESTRATOR: std::sync::Mutex<ContextOrchestrator> = 
        std::sync::Mutex::new(ContextOrchestrator::with_defaults());
}

/// 增强消息（全局函数）
pub fn enhance_message_with_context(message: &str) -> String {
    match GLOBAL_ORCHESTRATOR.lock() {
        Ok(orchestrator) => orchestrator.enhance_message(message),
        Err(_) => message.to_string(),
    }
}

/// 设置编排器配置
pub fn set_orchestrator_config(config: OrchestratorConfig) {
    if let Ok(mut orchestrator) = GLOBAL_ORCHESTRATOR.lock() {
        *orchestrator = ContextOrchestrator::new(config);
    }
}
