// 常量模块
// 将所有常量按功能模块化管理，便于维护和扩展

pub mod app;
pub mod font;
pub mod mcp;
pub mod network;
pub mod theme;
pub mod ui;
pub mod validation;
pub mod window;

// 重新导出所有常量，方便使用
// 注意：为了避免命名冲突，某些模块使用限定导出
pub use app::*;
pub use theme::*;
pub use ui::*;
pub use validation::*;
pub use window::*;
