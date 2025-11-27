//! 记忆存储模块
//!
//! 提供可插拔的存储后端支持，包括文件存储和SQLite存储

pub mod traits;
pub mod sqlite;
pub mod file;
pub mod migration;

pub use traits::{MemoryStorage, MemoryUsageStat};
pub use sqlite::SqliteStorage;
pub use file::FileStorage;
pub use migration::MigrationManager;
