use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Class,
    Struct,
    Method,
    Interface,
    Enum,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Unknown,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 文件路径
    pub path: String,
    /// 相关性分数
    pub score: f32,
    /// 代码片段
    pub snippet: String,
    /// 匹配行号
    pub line_number: usize,
    /// 结构化上下文 (增强)
    #[serde(default)]
    pub context: Option<SnippetContext>,
    /// 匹配信息 (增强)
    #[serde(default)]
    pub match_info: Option<MatchInfo>,
}

/// Snippet 结构化上下文
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnippetContext {
    /// 所属模块 (e.g., "auth/handler")
    pub module: Option<String>,
    /// 父级符号 (e.g., "impl AuthHandler")
    pub parent_symbol: Option<String>,
    /// 符号类型 (e.g., "function", "method")
    pub symbol_kind: Option<String>,
    /// 可见性 (e.g., "pub", "pub(crate)")
    pub visibility: Option<String>,
    /// 文档注释
    pub doc_comment: Option<String>,
    /// 函数/方法签名
    pub signature: Option<String>,
}

/// 匹配信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchInfo {
    /// 匹配的词项
    pub matched_terms: Vec<String>,
    /// 匹配类型: "symbol" | "content" | "path"
    pub match_type: String,
    /// 匹配质量: "exact" | "partial" | "fuzzy"
    pub match_quality: String,
}

#[derive(Debug, Clone)]
pub struct LocalEngineConfig {
    pub index_path: PathBuf,
    pub max_results: usize,
    pub snippet_context: usize,
}

impl Default for LocalEngineConfig {
    fn default() -> Self {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".acemcp");
        path.push("local_index");
        
        Self {
            index_path: path,
            max_results: 10,
            snippet_context: 3,
        }
    }
}