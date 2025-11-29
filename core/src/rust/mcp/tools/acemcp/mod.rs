// Search 工具模块
// 本地代码搜索引擎 (Tantivy + Tree-sitter)

pub mod mcp;
pub mod types;
pub mod commands;
pub mod local_engine;
pub mod health;

// 重新导出工具以便访问
pub use mcp::AcemcpTool;
