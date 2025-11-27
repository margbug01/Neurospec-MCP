use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use rmcp::{ErrorData as McpError, model::*};

use crate::mcp::{InteractRequest, PopupRequest};
use crate::mcp::handlers::{create_tauri_popup, parse_mcp_response};
use crate::mcp::utils::popup_error;

use super::history::save_interact_record;
use super::interceptor::auto_recall_async;

/// 标记是否已经提示过创建 AGENTS.md（避免重复提示）
static AGENTS_PROMPT_SHOWN: AtomicBool = AtomicBool::new(false);

/// Interactive dialogue tool
///
/// 智能交互入口，支持弹窗交互（确认/选择/输入）
#[derive(Clone)]
pub struct InteractionTool;

impl InteractionTool {
    /// 智能交互入口
    pub async fn interact(
        request: InteractRequest,
    ) -> Result<CallToolResult, McpError> {
        // 首次调用时检测 AGENTS.md
        Self::check_agents_md_on_first_call().await;
        
        Self::handle_normal_interaction(&request).await
    }

    /// 处理普通交互流程
    async fn handle_normal_interaction(
        request: &InteractRequest,
    ) -> Result<CallToolResult, McpError> {
        let request_id = uuid::Uuid::new_v4().to_string();
        
        // 🔮 前置拦截：自动召回相关的代码修改记忆（使用嵌入模型语义匹配）
        let enhanced_message = if let Some(memory_context) = auto_recall_async(&request.message).await {
            // 将历史修改记忆附加到消息末尾
            format!("{}{}", request.message, memory_context)
        } else {
            request.message.clone()
        };
        
        let popup_request = PopupRequest {
            id: request_id.clone(),
            message: enhanced_message,
            predefined_options: if request.predefined_options.is_empty() {
                None
            } else {
                Some(request.predefined_options.clone())
            },
            is_markdown: request.is_markdown,
        };

        match create_tauri_popup(&popup_request).await {
            Ok(response) => {
                // 保存历史记录
                let project_path = Self::detect_project_root()
                    .map(|p| p.to_string_lossy().to_string());
                
                // 尝试解析 JSON 格式的响应（兼容两种格式）
                let (user_input, selected) = if let Ok(resp_json) = serde_json::from_str::<serde_json::Value>(&response) {
                    // JSON 格式：提取 user_input 和 selected_options
                    let input = resp_json.get("user_input")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let opts = resp_json.get("selected_options")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Vec<_>>())
                        .unwrap_or_default();
                    (input, opts)
                } else {
                    // 纯文本格式：直接使用响应作为 user_input
                    (Some(response.clone()), vec![])
                };
                
                // 保存历史记录（无论哪种格式都会执行）
                if let Err(e) = save_interact_record(
                    &request_id,
                    &request.message,
                    &request.predefined_options,
                    user_input.as_deref(),
                    &selected,
                    project_path.as_deref(),
                ) {
                    log::warn!("Failed to save interact record: {}", e);
                }
                
                let mut content = parse_mcp_response(&response)?;
                
                // 🔔 在返回内容末尾添加 CHANGE_REPORT 提醒
                content = Self::append_change_report_reminder(content);
                
                Ok(crate::mcp::create_success_result(content))
            }
            Err(e) => {
                Err(popup_error(e.to_string()).into())
            }
        }
    }
    
    // Legacy method name for backward compatibility
    pub async fn zhi(request: InteractRequest) -> Result<CallToolResult, McpError> {
        Self::interact(request).await
    }

    /// 首次调用时检测 AGENTS.md
    async fn check_agents_md_on_first_call() {
        // 如果已经提示过，跳过
        if AGENTS_PROMPT_SHOWN.load(Ordering::Relaxed) {
            return;
        }

        // 标记已提示（即使检测失败也不再提示）
        AGENTS_PROMPT_SHOWN.store(true, Ordering::Relaxed);

        // 检测项目根目录
        let project_root = match Self::detect_project_root() {
            Some(root) => root,
            None => return,
        };

        // 检查是否存在 AGENTS.md
        let agents_path = project_root.join("AGENTS.md");
        if agents_path.exists() {
            log::info!("AGENTS.md detected at: {}", agents_path.display());
            return;
        }

        // 不存在 AGENTS.md，发送提示
        log::info!("No AGENTS.md found in project: {}", project_root.display());
        
        // 创建提示弹窗
        let popup_request = PopupRequest {
            id: uuid::Uuid::new_v4().to_string(),
            message: format!(
                "## 📋 未检测到 AGENTS.md\n\n\
                在项目 `{}` 中未找到 AGENTS.md 配置文件。\n\n\
                AGENTS.md 可以定义 AI 的行为规范，包括：\n\
                - 🎭 角色定义\n\
                - 🔧 可用工具\n\
                - ⚠️ 最高原则\n\
                - 📝 自定义规则\n\n\
                建议在应用的 **AGENTS** 标签页中创建配置。",
                project_root.display()
            ),
            predefined_options: Some(vec![
                "✅ 知道了".to_string(),
                "🚀 稍后创建".to_string(),
            ]),
            is_markdown: true,
        };

        // 发送提示（异步，不阻塞主流程）
        if let Err(e) = create_tauri_popup(&popup_request).await {
            log::warn!("Failed to show AGENTS.md prompt: {}", e);
        }
    }

    /// 检测项目根目录
    fn detect_project_root() -> Option<PathBuf> {
        let cwd = std::env::current_dir().ok()?;
        
        // 向上查找 .git 目录
        let mut current = cwd.as_path();
        loop {
            if current.join(".git").exists() {
                return Some(current.to_path_buf());
            }
            current = current.parent()?;
        }
    }

    /// 在返回内容末尾添加 CHANGE_REPORT 提醒（精简版）
    fn append_change_report_reminder(content: Vec<rmcp::model::Content>) -> Vec<rmcp::model::Content> {
        use rmcp::model::Content;
        
        // 精简版提醒，减少 token 消耗
        let reminder = "\n\n---\n💡 代码修改后请添加 `[CHANGE_REPORT]` 标记";

        let mut result = content;
        result.push(Content::text(reminder));
        result
    }
}
