// 重新导出所有命令函数，避免重名冲突

// MCP 命令
pub use crate::mcp::commands::*;

// UI 命令
pub use crate::ui::{
    commands::*,
    window::*,
    font_commands::*,
    updater::*,
    exit::*,
    exit_handler::*,
};
