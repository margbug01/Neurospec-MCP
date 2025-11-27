use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// NSP (NeuroSpec Protocol) 数据模型
///
/// 用于IDE侧LLM的structured output目标，
/// 也用于NeuroSpec MCP内部的校验与归一化

/// 单个执行步骤
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NSPExecutionStep {
    /// 全局唯一步骤ID（在当前NSP内）
    pub step_id: i32,
    /// 简短的人类可读标题
    pub title: String,
    /// 对代码或项目的操作类型
    pub action: NSPAction,
    /// 主要目标文件相对路径；允许为空，如为全局分析或说明步骤
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// 给Coder模型的详细自然语言指令（人类可读）
    pub instruction: String,
    /// 执行前的假设条件或检查
    #[serde(default)]
    pub preconditions: Vec<String>,
    /// 期望达成的结果或可验证条件
    #[serde(default)]
    pub postconditions: Vec<String>,
    /// 依赖的其它步骤ID列表
    #[serde(default)]
    pub depends_on: Vec<i32>,
    /// 风险标签，例如 ['SCHEMA_CHANGE', 'AUTH_IMPACT']
    #[serde(default)]
    pub risk_tags: Vec<String>,
}

/// 操作类型
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum NSPAction {
    #[serde(rename = "CREATE")]
    Create,
    #[serde(rename = "MODIFY")]
    Modify,
    #[serde(rename = "DELETE")]
    Delete,
    #[serde(rename = "REFACTOR")]
    Refactor,
    #[serde(rename = "ANALYZE")]
    Analyze,
}

/// 引用项目级记忆配置
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NSPProjectMemoryRef {
    /// 记忆profile ID，例如 'default', 'backend', 'frontend'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    /// 内联提示，例如代码风格、禁用模块等，优先级高于持久化记忆
    #[serde(default)]
    pub inline_hints: HashMap<String, serde_json::Value>,
}

/// 上下文锁定：指定哪些文件可以被修改，哪些只能读
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NSPContextLock {
    /// 允许被修改的文件路径列表
    #[serde(default)]
    pub target_files: Vec<String>,
    /// 只可读取不可修改的文件路径列表
    #[serde(default)]
    pub read_only_refs: Vec<String>,
}

/// NSP顶层元信息
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NSPMeta {
    /// NSP协议版本号，用于后续兼容处理
    #[serde(default = "default_nsp_version")]
    pub nsp_version: String,
    /// 对用户需求的精简总结
    pub intent_summary: String,
    /// 整体风险评估
    #[serde(default = "default_risk_level")]
    pub risk_level: NSPRiskLevel,
    /// Architect无法确定、需要人类确认的问题列表
    #[serde(default)]
    pub open_questions: Vec<String>,
}

/// 风险级别
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum NSPRiskLevel {
    #[serde(rename = "LOW")]
    Low,
    #[serde(rename = "MEDIUM")]
    Medium,
    #[serde(rename = "HIGH")]
    High,
}

/// NeuroSpec Protocol 顶层对象
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NSP {
    pub meta: NSPMeta,
    pub context_lock: NSPContextLock,
    /// 全局约束，如 ['NO_DB_MIGRATION', 'STRICT_TYPES']
    #[serde(default)]
    pub constraints: Vec<String>,
    /// 按顺序排列的一组步骤
    pub execution_plan: Vec<NSPExecutionStep>,
    /// 项目记忆引用信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_memory: Option<NSPProjectMemoryRef>,
}

fn default_nsp_version() -> String {
    "1.0".to_string()
}

fn default_risk_level() -> NSPRiskLevel {
    NSPRiskLevel::Medium
}
