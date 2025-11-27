//! NeuroSpec 服务模块
//!
//! 提供项目分析和重构辅助功能

pub mod agents_parser;
pub mod analyzer;
pub mod embedding;
pub mod graph;
pub mod refactor;
pub mod xray_engine;

pub use agents_parser::{AgentsConfig, detect_agents_md};
pub use analyzer::*;
pub use embedding::{
    EmbeddingService, EmbeddingConfig, EmbeddingProvider, cosine_similarity,
    init_global_embedding_service, get_global_embedding_service,
    has_embedding_service, is_embedding_available, reload_embedding_service,
    compute_similarity, find_similar,
};
pub use graph::*;
pub use refactor::*;
pub use xray_engine::*;
