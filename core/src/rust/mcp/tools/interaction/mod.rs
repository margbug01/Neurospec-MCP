//! 智能交互工具模块
//!
//! 提供智能交互功能：
//! - 弹窗交互（确认/选择/输入）
//! - 历史记录存储与查询
//! - 记忆拦截（自动召回和记录）

pub mod mcp;
pub mod history;
pub mod interceptor;

pub use mcp::InteractionTool;
pub use history::{InteractRecord, InteractHistory, get_interact_history, search_interact_history, clear_interact_history, init_interact_history};
pub use interceptor::{MemoryInterceptor, auto_recall, auto_recall_async, auto_record, get_interceptor};
