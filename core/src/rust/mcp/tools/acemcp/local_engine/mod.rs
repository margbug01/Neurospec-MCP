pub mod ctags;
pub mod extractor;
pub mod indexer;
pub mod ripgrep;
pub mod searcher;
pub mod types;
pub mod vector_store;

// 重新导出常用类型
pub use ctags::CtagsIndexer;
pub use indexer::LocalIndexer;
pub use ripgrep::RipgrepSearcher;
pub use searcher::LocalSearcher;
pub use types::{LocalEngineConfig, SearchResult, SnippetContext, MatchInfo};
pub use vector_store::{CodeVectorStore, CodeVectorEntry, VectorStoreStats};
