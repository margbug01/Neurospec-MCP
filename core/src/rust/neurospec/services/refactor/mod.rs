pub mod renamer;
pub mod validator;

use serde::{Deserialize, Serialize};

/// Represents a single code edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edit {
    /// File path (absolute or relative to project root)
    pub file_path: String,
    /// Start byte offset in the file
    pub start_byte: usize,
    /// End byte offset in the file
    pub end_byte: usize,
    /// Replacement text
    pub replacement: String,
}

impl Edit {
    /// Create a new edit
    pub fn new(file_path: String, start_byte: usize, end_byte: usize, replacement: String) -> Self {
        Self {
            file_path,
            start_byte,
            end_byte,
            replacement,
        }
    }
}

/// Result of a refactoring operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorResult {
    /// List of files that were modified
    pub modified_files: Vec<String>,
    /// List of edits that were applied
    pub edits: Vec<Edit>,
    /// Whether the refactoring was successful
    pub success: bool,
    /// Error message if any
    pub error: Option<String>,
}

impl RefactorResult {
    pub fn success(modified_files: Vec<String>, edits: Vec<Edit>) -> Self {
        Self {
            modified_files,
            edits,
            success: true,
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            modified_files: vec![],
            edits: vec![],
            success: false,
            error: Some(message),
        }
    }
}
