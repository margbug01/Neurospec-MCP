use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(rename_all = "lowercase")]
pub enum SearchMode {
    Text,
    Symbol,
    Structure,
}

/// Code search request parameters
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchRequest {
    /// Absolute path to the project root directory (optional).
    /// If not provided, will auto-detect from current working directory or Git root.
    #[schemars(description = "Optional: Absolute path to the project root. If omitted, auto-detects from current working directory or Git root.")]
    pub project_root_path: Option<String>,
    
    /// Search query - natural language for text mode, symbol name for symbol mode.
    /// For 'structure' mode, this can be empty or omitted.
    #[schemars(description = "Search query. For 'text' mode: natural language query like 'user authentication'. For 'symbol' mode: function/class name like 'handleLogin'. For 'structure' mode: can be empty.")]
    pub query: String,

    /// Search mode: 'text' for full-text search, 'symbol' for symbol definition lookup, 'structure' for project overview.
    #[schemars(description = "Search mode: 'text' (default) for full-text search, 'symbol' for finding symbol definitions, 'structure' for project structure overview (no query needed).")]
    pub mode: Option<SearchMode>,
}

/// Legacy alias for backward compatibility
pub type AcemcpRequest = SearchRequest;
