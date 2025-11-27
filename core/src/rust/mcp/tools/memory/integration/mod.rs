//! 深度集成模块
//!
//! 提供与开发工作流的集成功能

pub mod git;
pub mod export;

pub use git::{GitIntegration, GitSuggestion};
pub use export::{MemoryExporter, ExportFormat};
