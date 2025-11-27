use anyhow::Result;
use rmcp::model::*;
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::PathBuf;
use lazy_static::lazy_static;

use super::{MemoryCategory, MemoryManager, MemoryEntry, MemorySuggester, ConversationContext, MemoryListResult, ScoredMemory};
use crate::mcp::{
    utils::{
        errors::{invalid_params_error, memory_error, McpToolError},
        project_path_error, validate_project_path,
    },
    MemoryRequest, InteractRequest,
};
use crate::mcp::tools::interaction::InteractionTool;

// Simple LRU-like Path Cache (Global)
lazy_static! {
    static ref PATH_CACHE: Mutex<HashMap<String, PathBuf>> = Mutex::new(HashMap::new());
    static ref MEMORY_SUGGESTER: Mutex<MemorySuggester> = Mutex::new(MemorySuggester::new());
}

/// Global memory management tool
///
/// For storing and managing development rules, user preferences, and best practices
#[derive(Clone)]
pub struct MemoryTool;

impl MemoryTool {
    /// 自动推断项目路径
    /// 如果 project_path 为空，从当前工作目录向上查找 .git 目录
    fn resolve_project_path(project_path: &str) -> Result<String, McpToolError> {
        // 如果提供了路径，直接使用
        if !project_path.trim().is_empty() {
            return Ok(project_path.to_string());
        }

        // 自动推断：从当前工作目录查找 Git 根目录
        let cwd = std::env::current_dir()
            .map_err(|e| memory_error(format!("无法获取当前工作目录: {}", e)))?;

        let mut current = cwd.as_path();
        loop {
            if current.join(".git").exists() {
                return Ok(current.to_string_lossy().to_string());
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        Err(memory_error(
            "无法自动推断项目路径。请确保在 Git 仓库中运行，或手动指定 project_path 参数。"
        ))
    }

    pub async fn manage_memory(request: MemoryRequest) -> Result<CallToolResult, McpToolError> {
        // Security: Content Length Check
        if request.content.len() > 10000 {
            return Err(invalid_params_error(
                "Content exceeds maximum length of 10000 characters"
            ));
        }

        // 自动推断项目路径
        let project_path = Self::resolve_project_path(&request.project_path)?;

        // Performance: Path Cache Check
        let cached_path = {
            let cache = PATH_CACHE.lock().unwrap();
            cache.get(&project_path).cloned()
        };

        if cached_path.is_none() {
            // Cache miss: Validate path
            if let Err(e) = validate_project_path(&project_path) {
                return Err(project_path_error(format!(
                    "Path validation failed: {}\nResolved path: {}\nPlease check if the path format is correct.",
                    e,
                    project_path
                )));
            } else {
                let mut cache = PATH_CACHE.lock().unwrap();
                cache.insert(project_path.clone(), PathBuf::from(&project_path));
            }
        }

        let manager = MemoryManager::new(&project_path)
            .map_err(|e| memory_error(format!("Failed to create memory manager: {}", e)))?;

        let result = match request.action.as_str() {
            "remember" | "记忆" => {
                if request.content.trim().is_empty() {
                    return Err(invalid_params_error("Memory content is required"));
                }

                let category = match request.category.as_str() {
                    "rule" => MemoryCategory::Rule,
                    "preference" => MemoryCategory::Preference,
                    "pattern" => MemoryCategory::Pattern,
                    "context" => MemoryCategory::Context,
                    _ => MemoryCategory::Context,
                };

                let id = manager
                    .add_memory(&request.content, category)
                    .map_err(|e| memory_error(format!("Failed to add memory: {}", e)))?;

                format!(
                    "✅ Memory added successfully\nID: {}\nContent: {}\nCategory: {:?}",
                    id, request.content, category
                )
            }
            "recall" | "回忆" => {
                // 智能召回：如果提供了 context，使用智能检索
                if let Some(ref ctx) = request.context {
                    if !ctx.trim().is_empty() {
                        let limit = request.page_size.min(20).max(5);
                        let scored = manager
                            .smart_recall(Some(ctx), limit, None)
                            .map_err(|e| memory_error(format!("Smart recall failed: {}", e)))?;
                        
                        if scored.is_empty() {
                            "📭 未找到相关记忆".to_string()
                        } else {
                            Self::format_smart_recall_result(&scored)
                        }
                    } else {
                        manager
                            .get_project_info()
                            .map_err(|e| memory_error(format!("Failed to retrieve project info: {}", e)))?
                    }
                } else {
                    manager
                        .get_project_info()
                        .map_err(|e| memory_error(format!("Failed to retrieve project info: {}", e)))?
                }
            }
            
            "delete" | "删除" | "forget" | "忘记" => {
                let id = request.id.as_ref().ok_or_else(|| {
                    invalid_params_error("Memory ID is required for delete action")
                })?;

                let deleted = manager
                    .delete_memory(id)
                    .map_err(|e| memory_error(format!("Failed to delete memory: {}", e)))?;

                if deleted {
                    format!("✅ Memory deleted successfully\nID: {}", id)
                } else {
                    format!("⚠️ Memory not found\nID: {}", id)
                }
            }

            "update" | "更新" | "modify" | "修改" => {
                let id = request.id.as_ref().ok_or_else(|| {
                    invalid_params_error("Memory ID is required for update action")
                })?;

                if request.content.trim().is_empty() {
                    return Err(invalid_params_error("New content is required for update action"));
                }

                let updated = manager
                    .update_memory(id, &request.content)
                    .map_err(|e| memory_error(format!("Failed to update memory: {}", e)))?;

                if updated {
                    format!("✅ Memory updated successfully\nID: {}\nNew content: {}", id, request.content)
                } else {
                    format!("⚠️ Memory not found\nID: {}", id)
                }
            }

            "list" | "列表" => {
                let category = match request.category.as_str() {
                    "rule" => Some(MemoryCategory::Rule),
                    "preference" => Some(MemoryCategory::Preference),
                    "pattern" => Some(MemoryCategory::Pattern),
                    "context" => Some(MemoryCategory::Context),
                    "all" | "" => None,
                    _ => None,
                };

                let result = manager
                    .list_memories(category, request.page, request.page_size)
                    .map_err(|e| memory_error(format!("Failed to list memories: {}", e)))?;

                Self::format_list_result(&result)
            }

            "get" | "获取" => {
                let id = request.id.as_ref().ok_or_else(|| {
                    invalid_params_error("Memory ID is required for get action")
                })?;

                let memory = manager
                    .get_memory_by_id(id)
                    .map_err(|e| memory_error(format!("Failed to get memory: {}", e)))?;

                match memory {
                    Some(m) => format!(
                        "📝 Memory Details\nID: {}\nCategory: {:?}\nContent: {}\nCreated: {}\nUpdated: {}",
                        m.id, m.category, m.content, m.created_at, m.updated_at
                    ),
                    None => format!("⚠️ Memory not found\nID: {}", id),
                }
            }

            "export" | "导出" => {
                let memories = manager
                    .get_all_memories()
                    .map_err(|e| memory_error(format!("Failed to get memories: {}", e)))?;

                let format = match request.category.as_str() {
                    "markdown" | "md" => super::ExportFormat::Markdown,
                    _ => super::ExportFormat::Json,
                };

                let content = match format {
                    super::ExportFormat::Json => {
                        super::MemoryExporter::export_json(&memories, &request.project_path)
                            .map_err(|e| memory_error(format!("Export failed: {}", e)))?
                    }
                    super::ExportFormat::Markdown => {
                        super::MemoryExporter::export_markdown(&memories, &request.project_path)
                            .map_err(|e| memory_error(format!("Export failed: {}", e)))?
                    }
                };

                format!("📤 导出成功 ({} 条记忆)\n\n{}", memories.len(), content)
            }

            "import" | "导入" => {
                if request.content.trim().is_empty() {
                    return Err(invalid_params_error("Import content is required"));
                }

                let imported = super::MemoryExporter::import_json(&request.content)
                    .map_err(|e| memory_error(format!("Import failed: {}", e)))?;

                let mut success_count = 0;
                for mem in imported {
                    if manager.add_memory(&mem.content, mem.category).is_ok() {
                        success_count += 1;
                    }
                }

                format!("📥 导入成功: {} 条记忆", success_count)
            }

            "git_scan" | "扫描git" => {
                let git = super::GitIntegration::new(&request.project_path);
                let suggestions = git.extract_suggestions(50)
                    .map_err(|e| memory_error(format!("Git scan failed: {}", e)))?;

                if suggestions.is_empty() {
                    "📭 未从 Git 历史中发现可记忆的模式".to_string()
                } else {
                    let mut output = format!("🔍 从 Git 历史发现 {} 条建议:\n\n", suggestions.len());
                    for (i, s) in suggestions.iter().enumerate() {
                        output.push_str(&format!("{}. {} (置信度: {:.0}%)\n", i + 1, s.content, s.confidence * 100.0));
                    }
                    output
                }
            }

            "context" | "上下文" | "project_context" => {
                // 智能上下文注入：获取项目背景信息
                Self::get_project_context(&project_path, &manager)?
            }

            "analyze" | "分析" | "analyze_patterns" => {
                // 代码模式分析
                use super::ai_suggester::CodePatternAnalyzer;
                
                let analysis = CodePatternAnalyzer::analyze_project(&project_path)
                    .map_err(|e| memory_error(format!("代码分析失败: {}", e)))?;
                
                CodePatternAnalyzer::format_analysis(&analysis)
            }

            _ => {
                return Err(invalid_params_error(format!(
                    "Unknown action type: {}. Supported actions: 'remember', 'recall', 'delete', 'update', 'list', 'get', 'export', 'import', 'git_scan', 'context', 'analyze'",
                    request.action
                )));
            }
        };

        Ok(crate::mcp::create_success_result(vec![Content::text(
            result,
        )]))
    }

    // Legacy method name for backward compatibility
    pub async fn jiyi(request: MemoryRequest) -> Result<CallToolResult, McpToolError> {
        Self::manage_memory(request).await
    }

    /// 确认记忆管理重构计划
    /// 通过弹窗向用户确认执行方案
    pub async fn confirm_refactor_plan() -> Result<CallToolResult, McpToolError> {
        let message = r#"# 🔄 记忆管理模块重构计划确认

## 📋 NeuroSpec 协议执行方案

**目标**: 实施 Phase 0 - 智能记忆发现与建议系统

### Meta Information
- **nsp_version**: "1.0"
- **intent_summary**: "记忆管理模块重构 - Phase 0实施"
- **risk_level**: "MEDIUM"
- **open_questions**: ["用户确认方案", "技术实现细节"]

### 执行计划

#### Step 1: 创建 ai_suggester.rs 模块
- **action**: CREATE
- **instruction**: "实现 AI 记忆建议器核心功能，包括模式检测和建议生成"
- **涉及文件**: `src/rust/mcp/tools/memory/ai_suggester.rs`

#### Step 2: 扩展 MCP 接口
- **action**: MODIFY
- **path**: `src/rust/mcp/tools/memory/mcp.rs`
- **instruction**: "添加智能记忆建议相关接口，支持 AI 主动建议"

#### Step 3: 创建前端记忆建议弹窗
- **action**: CREATE
- **path**: `src/frontend/components/popup/MemorySuggestionModal.vue`
- **instruction**: "实现用户友好的记忆建议弹窗界面"

### 核心价值
- ✅ AI 主动建议添加记忆
- ✅ 智能记忆注入到对话
- ✅ 记忆使用情况可视化
- ✅ 零摩擦记忆添加（3秒完成）

**请确认是否授权执行此计划？**"#;

        let interact_request = InteractRequest {
            message: message.to_string(),
            predefined_options: vec![
                "✅ 确认执行 Phase 0 计划".to_string(),
                "❌ 暂停，需要更多说明".to_string(),
                "📝 修改计划细节".to_string(),
            ],
            is_markdown: true,
        };

        let response = InteractionTool::interact(interact_request)
            .await
            .map_err(|e| McpToolError::Generic(anyhow::anyhow!("{}", e)))?;

        Ok(response)
    }

    /// 获取智能记忆建议
    ///
    /// 分析对话内容，生成记忆建议
    pub async fn get_memory_suggestions(
        messages: Vec<String>,
        project_path: Option<String>,
    ) -> Result<CallToolResult, McpToolError> {
        // 创建对话上下文
        let context = ConversationContext {
            messages,
            project_context: project_path,
            language: None,
        };

        // 获取全局记忆建议器实例
        let suggester = MEMORY_SUGGESTER.lock().map_err(|e| {
            McpToolError::Generic(anyhow::anyhow!("Failed to acquire memory suggester lock: {}", e))
        })?;

        // 检测模式并生成建议
        let suggestions = suggester.detect_pattern(&context);

        if suggestions.is_empty() {
            return Ok(crate::mcp::create_success_result(vec![Content::text(
                "暂无记忆建议。系统正在学习您的对话模式...".to_string(),
            )]));
        }

        // 生成建议摘要
        let summary = suggester.generate_suggestion_summary(&suggestions);

        // 转换为JSON格式返回
        let suggestions_json = serde_json::to_string_pretty(&suggestions)
            .map_err(|e| McpToolError::Generic(anyhow::anyhow!("序列化建议失败: {}", e)))?;

        let response = format!(
            "# 🧠 AI 记忆建议\n\n{}\n\n## 详细信息\n\n```json\n{}\n```",
            summary, suggestions_json
        );

        Ok(crate::mcp::create_success_result(vec![Content::text(response)]))
    }

    /// 记录记忆使用
    pub async fn record_memory_usage(memory_id: String) -> Result<CallToolResult, McpToolError> {
        let mut suggester = MEMORY_SUGGESTER.lock().map_err(|e| {
            McpToolError::Generic(anyhow::anyhow!("Failed to acquire memory suggester lock: {}", e))
        })?;

        suggester.record_memory_usage(&memory_id);

        Ok(crate::mcp::create_success_result(vec![Content::text(
            format!("✅ 已记录记忆使用: {}", memory_id)
        )]))
    }

    /// 获取相关记忆
    pub async fn get_related_memories(
        query: String,
        existing_memories: Vec<MemoryEntry>,
    ) -> Result<CallToolResult, McpToolError> {
        let suggester = MEMORY_SUGGESTER.lock().map_err(|e| {
            McpToolError::Generic(anyhow::anyhow!("Failed to acquire memory suggester lock: {}", e))
        })?;

        let related = suggester.get_related_memories(&query, &existing_memories);

        if related.is_empty() {
            return Ok(crate::mcp::create_success_result(vec![Content::text(
                "未找到相关记忆".to_string()
            )]));
        }

        let response = format!(
            "找到 {} 条相关记忆:\n\n{}",
            related.len(),
            related.iter()
                .take(5)  // 只显示前5条
                .map(|(memory, score)| {
                    format!(
                        "- **{}** (相关度: {:.2})\n  {}",
                        match memory.category {
                            MemoryCategory::Rule => "规则",
                            MemoryCategory::Pattern => "模式",
                            MemoryCategory::Preference => "偏好",
                            MemoryCategory::Context => "上下文",
                        },
                        score,
                        memory.content
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        );

        Ok(crate::mcp::create_success_result(vec![Content::text(response)]))
    }

    /// 获取项目上下文信息
    /// 自动检测项目类型、依赖、并召回相关记忆
    fn get_project_context(project_path: &str, manager: &MemoryManager) -> Result<String, McpToolError> {
        use std::fs;
        use std::path::Path;

        let root = Path::new(project_path);
        let mut context = String::new();
        context.push_str("# 📋 项目上下文\n\n");

        // 1. 检测项目类型和依赖
        let mut project_type = "Unknown";
        let mut project_name = root.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let mut key_deps: Vec<String> = Vec::new();

        // Rust 项目
        let cargo_toml = root.join("Cargo.toml");
        if cargo_toml.exists() {
            project_type = "Rust";
            if let Ok(content) = fs::read_to_string(&cargo_toml) {
                // 简单解析 name
                for line in content.lines() {
                    if line.starts_with("name") {
                        if let Some(name) = line.split('=').nth(1) {
                            project_name = name.trim().trim_matches('"').to_string();
                        }
                    }
                }
                // 提取依赖
                let mut in_deps = false;
                for line in content.lines() {
                    if line.starts_with("[dependencies]") || line.starts_with("[dev-dependencies]") {
                        in_deps = true;
                        continue;
                    }
                    if line.starts_with('[') {
                        in_deps = false;
                    }
                    if in_deps && !line.trim().is_empty() {
                        if let Some(dep) = line.split('=').next() {
                            let dep = dep.trim();
                            if !dep.is_empty() && key_deps.len() < 10 {
                                key_deps.push(dep.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Node.js 项目
        let package_json = root.join("package.json");
        if package_json.exists() {
            if project_type == "Unknown" {
                project_type = "Node.js/TypeScript";
            }
            if let Ok(content) = fs::read_to_string(&package_json) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                        if project_name.is_empty() || project_name == root.file_name().unwrap_or_default().to_string_lossy() {
                            project_name = name.to_string();
                        }
                    }
                    // 提取依赖
                    if let Some(deps) = json.get("dependencies").and_then(|v| v.as_object()) {
                        for (k, _) in deps.iter().take(10) {
                            if !key_deps.contains(k) {
                                key_deps.push(k.clone());
                            }
                        }
                    }
                }
            }
        }

        // Python 项目
        let pyproject = root.join("pyproject.toml");
        let requirements = root.join("requirements.txt");
        if pyproject.exists() || requirements.exists() {
            if project_type == "Unknown" {
                project_type = "Python";
            }
        }

        context.push_str(&format!("## 项目信息\n"));
        context.push_str(&format!("- **名称**: {}\n", project_name));
        context.push_str(&format!("- **类型**: {}\n", project_type));
        context.push_str(&format!("- **路径**: {}\n", project_path));

        if !key_deps.is_empty() {
            context.push_str(&format!("- **主要依赖**: {}\n", key_deps.join(", ")));
        }

        // 2. 召回相关记忆
        context.push_str("\n## 项目记忆\n");
        let memories = manager.list_memories(None, 1, 10)
            .map_err(|e| memory_error(format!("Failed to list memories: {}", e)))?;

        if memories.memories.is_empty() {
            context.push_str("暂无项目记忆\n");
        } else {
            for mem in &memories.memories {
                let icon = match mem.category {
                    MemoryCategory::Rule => "🔵",
                    MemoryCategory::Preference => "🟢",
                    MemoryCategory::Pattern => "🟡",
                    MemoryCategory::Context => "⚪",
                };
                context.push_str(&format!("- {} {}\n", icon, mem.content));
            }
            if memories.total > 10 {
                context.push_str(&format!("\n_...还有 {} 条记忆_\n", memories.total - 10));
            }
        }

        Ok(context)
    }

    /// 格式化列表结果
    fn format_list_result(result: &MemoryListResult) -> String {
        if result.memories.is_empty() {
            return format!(
                "📭 No memories found\nPage: {}/{}\nTotal: {}",
                result.page, result.total_pages.max(1), result.total
            );
        }

        let mut output = format!(
            "📚 Memory List (Page {}/{})\nTotal: {} memories\n\n",
            result.page, result.total_pages, result.total
        );

        for (i, memory) in result.memories.iter().enumerate() {
            let category_icon = match memory.category {
                MemoryCategory::Rule => "🔵",
                MemoryCategory::Preference => "🟢",
                MemoryCategory::Pattern => "🟡",
                MemoryCategory::Context => "⚪",
            };
            
            output.push_str(&format!(
                "{}. {} [{}] {}\n   ID: {}\n\n",
                (result.page - 1) * result.page_size + i + 1,
                category_icon,
                format!("{:?}", memory.category),
                memory.content,
                memory.id
            ));
        }

        if result.page < result.total_pages {
            output.push_str(&format!(
                "---\n💡 Use page={} to see more",
                result.page + 1
            ));
        }

        output
    }

    /// 格式化智能召回结果
    fn format_smart_recall_result(scored: &[ScoredMemory]) -> String {
        let mut output = format!("📚 相关记忆 (共 {} 条):\n\n", scored.len());

        for (i, sm) in scored.iter().enumerate() {
            let category_icon = match sm.memory.category {
                MemoryCategory::Rule => "🔵",
                MemoryCategory::Preference => "🟢",
                MemoryCategory::Pattern => "🟡",
                MemoryCategory::Context => "⚪",
            };

            output.push_str(&format!(
                "{}. {} {} (相关度: {:.0}%)\n",
                i + 1,
                category_icon,
                sm.memory.content,
                sm.relevance_score * 100.0
            ));
        }

        output
    }
}