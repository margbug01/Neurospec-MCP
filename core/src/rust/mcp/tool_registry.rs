//! 统一工具注册表
//!
//! 提供单点定义的工具注册，避免在 server.rs 和 dispatcher.rs 中重复注册

use rmcp::model::Tool;
use schemars::schema_for;

use crate::mcp::types::{InteractRequest, MemoryRequest};

#[cfg(feature = "experimental-neurospec")]
use crate::neurospec::tools::{ImpactAnalysisArgs, RenameArgs};

/// 工具定义条目
pub struct ToolDefinition {
    /// 工具名称
    pub name: &'static str,
    /// 工具描述
    pub description: &'static str,
    /// 是否为核心工具（始终启用）
    pub is_core: bool,
    /// 特性标志（None 表示无需特性）
    pub feature: Option<&'static str>,
}

/// 核心工具定义（静态注册表）
pub const CORE_TOOLS: &[ToolDefinition] = &[
    ToolDefinition {
        name: "interact",
        description: "Interactive dialogue tool with support for predefined options, free-text input, and image uploads",
        is_core: true,
        feature: None,
    },
    ToolDefinition {
        name: "memory",
        description: "Global memory management tool for storing and managing development rules, user preferences, and best practices",
        is_core: true,
        feature: None,
    },
];

/// NeuroSpec 高级工具（重构辅助）
#[cfg(feature = "experimental-neurospec")]
pub const NEUROSPEC_TOOLS: &[ToolDefinition] = &[
    ToolDefinition {
        name: "neurospec_graph_impact_analysis",
        description: "分析符号的依赖影响范围，用于重构前评估",
        is_core: false,
        feature: Some("experimental-neurospec"),
    },
    ToolDefinition {
        name: "neurospec_refactor_rename",
        description: "跨文件安全重命名符号（函数/类/变量）",
        is_core: false,
        feature: Some("experimental-neurospec"),
    },
];

/// 获取所有已注册的工具名称
pub fn get_all_tool_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = CORE_TOOLS.iter().map(|t| t.name).collect();
    
    #[cfg(feature = "experimental-neurospec")]
    {
        names.extend(NEUROSPEC_TOOLS.iter().map(|t| t.name));
    }
    
    names
}

/// 检查工具是否在注册表中
pub fn is_registered(name: &str) -> bool {
    CORE_TOOLS.iter().any(|t| t.name == name)
        || {
            #[cfg(feature = "experimental-neurospec")]
            {
                NEUROSPEC_TOOLS.iter().any(|t| t.name == name)
            }
            #[cfg(not(feature = "experimental-neurospec"))]
            {
                false
            }
        }
}

/// 将 schemars 生成的 RootSchema 转换为包含 definitions 的完整 JSON Schema
fn root_schema_to_json(root: schemars::schema::RootSchema) -> Option<serde_json::Map<String, serde_json::Value>> {
    let mut schema_map = serde_json::to_value(&root.schema)
        .ok()
        .and_then(|v| v.as_object().cloned())?;
    
    // 如果有 definitions，合并到 schema 中
    if !root.definitions.is_empty() {
        let definitions_value = serde_json::to_value(&root.definitions).ok()?;
        schema_map.insert("definitions".to_string(), definitions_value);
    }
    
    Some(schema_map)
}

/// 为工具生成 JSON Schema
pub fn get_tool_schema(name: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    match name {
        "interact" => {
            let schema = schema_for!(InteractRequest);
            root_schema_to_json(schema)
        }
        "memory" => {
            let schema = schema_for!(MemoryRequest);
            root_schema_to_json(schema)
        }
        #[cfg(feature = "experimental-neurospec")]
        "neurospec_graph_impact_analysis" => {
            let schema = schema_for!(ImpactAnalysisArgs);
            root_schema_to_json(schema)
        }
        #[cfg(feature = "experimental-neurospec")]
        "neurospec_refactor_rename" => {
            let schema = schema_for!(RenameArgs);
            root_schema_to_json(schema)
        }
        _ => None,
    }
}

/// 构建 MCP Tool 对象
pub fn build_tool(def: &ToolDefinition) -> Option<Tool> {
    get_tool_schema(def.name).map(|schema| {
        crate::mcp::create_tool(def.name, def.description, schema)
    })
}

/// 构建所有启用的工具列表
pub fn build_enabled_tools<F>(is_enabled: F) -> Vec<Tool>
where
    F: Fn(&str) -> bool,
{
    let mut tools = Vec::new();
    
    // 核心工具
    for def in CORE_TOOLS {
        if is_enabled(def.name) {
            if let Some(tool) = build_tool(def) {
                tools.push(tool);
            }
        }
    }
    
    // NeuroSpec 工具（如果启用了 feature）
    #[cfg(feature = "experimental-neurospec")]
    {
        for def in NEUROSPEC_TOOLS {
            if is_enabled(def.name) {
                if let Some(tool) = build_tool(def) {
                    tools.push(tool);
                }
            }
        }
    }
    
    tools
}
