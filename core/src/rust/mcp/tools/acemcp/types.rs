use serde::{Deserialize, Serialize};
use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};

/// 自定义 schema：同时接受字符串和 SearchProfile 对象
/// 用于兼容某些 MCP 客户端（如 Kiro）把嵌套对象序列化为字符串的情况
fn profile_schema(gen: &mut SchemaGenerator) -> Schema {
    let profile_schema = gen.subschema_for::<SearchProfile>();
    Schema::Object(SchemaObject {
        metadata: Some(Box::new(schemars::schema::Metadata {
            description: Some("High-level search profile. Accepts JSON object or JSON string (for client compatibility).".to_string()),
            ..Default::default()
        })),
        subschemas: Some(Box::new(schemars::schema::SubschemaValidation {
            any_of: Some(vec![
                Schema::Object(SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                    ..Default::default()
                }),
                Schema::Object(SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Null))),
                    ..Default::default()
                }),
                profile_schema,
            ]),
            ..Default::default()
        })),
        ..Default::default()
    })
}

/// 低层搜索模式（兼容旧调用 & 内部实现用）
///
/// - text: 全文搜索（自然语言）
/// - symbol: 符号定义搜索
/// - structure: 仅项目结构概览（老模式）
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(rename_all = "lowercase")]
pub enum SearchMode {
    Text,
    Symbol,
    Structure,
}

/// 搜索范围类型
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchScopeKind {
    /// 整个项目（默认）
    Project,
    /// 指定文件夹（可选 max_depth）
    Folder,
    /// 指定单个文件
    File,
    /// 聚焦某个符号（函数 / 类等）
    Symbol,
}

/// 搜索范围配置
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchScope {
    /// 范围类型：project / folder / file / symbol
    #[schemars(description = "Scope kind: project/folder/file/symbol")]
    pub kind: SearchScopeKind,

    /// 当 kind = folder/file 时的路径（相对或绝对，后端会规范化）
    #[serde(default)]
    #[schemars(description = "Optional path when kind is folder or file. Relative to project root if not absolute.")]
    pub path: Option<String>,

    /// 当 kind = folder 时的最大递归深度（不填使用安全默认）
    #[serde(default)]
    #[schemars(description = "Optional max depth when kind is folder. If omitted, a safe default is used.")]
    pub max_depth: Option<u8>,

    /// 当 kind = symbol 时的符号名（为空则回退到 query）
    #[serde(default)]
    #[schemars(description = "Optional symbol name when kind is symbol. Falls back to `query` if omitted.")]
    pub symbol: Option<String>,
}

/// 高层搜索策略（推荐 LLM 使用）
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchProfile {
    /// 推荐模式：结构优先 + 智能展开
    ///
    /// 后端行为（在实现中完成）：
    /// 1. 基于项目结构 + query + scope，挑选候选模块/文件/符号
    /// 2. 在这些候选上内部调用 Text / Symbol 搜索
    /// 3. 返回带 score/path/line 的结果，并附带结构信息
    SmartStructure {
        /// 搜索范围（默认整个项目）
        #[serde(default)]
        #[schemars(description = "Optional search scope: project/folder/file/symbol.")]
        scope: Option<SearchScope>,

        /// 期望的最大结果数（soft limit）
        #[serde(default)]
        #[schemars(description = "Soft limit for number of results. Backend may return fewer.")]
        max_results: Option<u32>,
    },

    /// 只返回项目结构概览，不做二次 Text/Symbol 搜索
    StructureOnly {
        /// 结构树最大层级深度
        #[serde(default)]
        #[schemars(description = "Optional max depth for structure overview.")]
        max_depth: Option<u8>,

        /// 最大节点/模块数量
        #[serde(default)]
        #[schemars(description = "Optional max number of modules/nodes to include.")]
        max_nodes: Option<u32>,
    },
}

/// Code search request parameters
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchRequest {
    /// Absolute path to the project root directory (optional).
    /// If not provided, will auto-detect from current working directory or Git root.
    #[schemars(description = "Optional: Absolute path to the project root. If omitted, auto-detects from current working directory or Git root.")]
    pub project_root_path: Option<String>,
    
    /// Search query.
    ///
    /// - For SmartStructure: natural language, e.g. "fix search JSON error"
    /// - For StructureOnly: may be empty, meaning "just show structure"
    #[serde(default)]
    #[schemars(description = "Primary search query. For smart structure search, use natural language. For structure-only mode, may be empty.")]
    pub query: String,

    /// 低层搜索模式（兼容旧调用，不推荐 LLM 直接设置）
    #[serde(default)]
    #[schemars(description = "Legacy low-level mode. Prefer using `profile` for new callers.")]
    pub mode: Option<SearchMode>,

    /// 高层搜索策略（推荐）
    ///
    /// 当设置该字段时，后端会根据 profile 执行结构优先的 orchestrator 逻辑；
    /// 未设置时则回退到旧的 mode 行为。
    /// 
    /// 注意：为兼容某些客户端，此字段同时接受 JSON 对象和 JSON 字符串。
    #[serde(default)]
    #[schemars(schema_with = "profile_schema")]
    pub profile: Option<SearchProfile>,
}

/// Legacy alias for backward compatibility
pub type AcemcpRequest = SearchRequest;

/// 搜索错误码（机器可解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SearchErrorCode {
    /// 索引尚未就绪，正在后台构建
    IndexNotReady,
    /// 项目路径无效或不存在
    InvalidProjectPath,
    /// 文件读取/写入错误
    IoError,
    /// 搜索引擎内部错误
    SearchEngineError,
    /// 未知错误
    UnknownError,
}

/// 结构化搜索错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchError {
    /// 机器可解析的错误码
    pub code: SearchErrorCode,
    /// 人类可读的错误消息
    pub message: String,
    /// 是否可重试
    pub retryable: bool,
}

impl SearchError {
    pub fn index_not_ready() -> Self {
        Self {
            code: SearchErrorCode::IndexNotReady,
            message: "索引尚未就绪，正在后台构建中。请稍后重试，或使用 ripgrep 回退搜索。".to_string(),
            retryable: true,
        }
    }

    pub fn invalid_project_path(path: &str) -> Self {
        Self {
            code: SearchErrorCode::InvalidProjectPath,
            message: format!("项目路径无效或不存在: {}", path),
            retryable: false,
        }
    }

    pub fn io_error(detail: &str) -> Self {
        Self {
            code: SearchErrorCode::IoError,
            message: format!("文件读取/写入错误: {}", detail),
            retryable: true,
        }
    }

    pub fn search_engine_error(detail: &str) -> Self {
        Self {
            code: SearchErrorCode::SearchEngineError,
            message: format!("搜索引擎内部错误: {}", detail),
            retryable: true,
        }
    }

    /// 格式化为 JSON 字符串（用于 MCP 返回）
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(r#"{{"code":"UNKNOWN_ERROR","message":"{}","retryable":false}}"#, self.message)
        })
    }
}

/// 搜索追踪信息（用于结构化日志和调试）
#[derive(Debug, Clone, Serialize)]
pub struct SearchTrace {
    /// 请求唯一标识
    pub request_id: String,
    /// 查询字符串
    pub query: String,
    /// 搜索模式
    pub mode: String,
    /// Profile 类型（如果有）
    pub profile: Option<String>,
    /// 实际使用的引擎
    pub engine_used: String,
    /// 索引健康状态
    pub index_health: String,
    /// 结果数量
    pub result_count: usize,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 降级链（如有回退）
    pub fallback_chain: Vec<String>,
    /// 是否触发了索引
    pub triggered_indexing: bool,
}

impl SearchTrace {
    pub fn new(query: String, mode: String) -> Self {
        Self {
            request_id: Self::generate_request_id(),
            query,
            mode,
            profile: None,
            engine_used: String::new(),
            index_health: String::new(),
            result_count: 0,
            duration_ms: 0,
            fallback_chain: Vec::new(),
            triggered_indexing: false,
        }
    }
    
    fn generate_request_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        format!("search_{}", timestamp)
    }
    
    /// 输出为 JSON 日志
    pub fn log(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            crate::log_important!(info, "SearchTrace: {}", json);
        }
    }
}
