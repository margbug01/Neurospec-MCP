use tauri::command;
use super::context_orchestrator::{set_orchestrator_config, OrchestratorConfig};

#[derive(Debug, serde::Deserialize)]
pub struct ContextOrchestratorConfigArgs {
    pub enabled: bool,
    pub max_memories: Option<usize>,
    pub max_code_snippets: Option<usize>,
    pub show_source: Option<bool>,
}

/// 设置上下文编排器配置
#[command]
pub async fn set_context_orchestrator_config(args: ContextOrchestratorConfigArgs) -> Result<(), String> {
    let config = OrchestratorConfig {
        enabled: args.enabled,
        max_memories: args.max_memories.unwrap_or(5),
        max_code_snippets: args.max_code_snippets.unwrap_or(3),
        show_source: args.show_source.unwrap_or(false),
    };
    
    set_orchestrator_config(config);
    Ok(())
}
