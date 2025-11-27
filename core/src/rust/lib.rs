pub mod app;
pub mod config;
pub mod constants;
pub mod daemon;
pub mod mcp;
pub mod neurospec;
pub mod ui;
pub mod utils;

// 避免重名导出，使用限定导出
pub use config::*;
pub use utils::*;

// 选择性导出常用项，避免冲突
pub use constants::{app as app_constants, theme, validation, network};
pub use mcp::{server, tools, types, handlers, utils as mcp_utils};
pub use ui::{window as ui_window, updater};
