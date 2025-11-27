use rmcp::{model::Content, ErrorData as McpError};
use schemars::JsonSchema;
use serde::Deserialize;
use tree_sitter::StreamingIterator;

use crate::neurospec::models::SymbolKind;
use crate::neurospec::services::graph::builder::GraphBuilder;
use crate::neurospec::services::refactor::renamer::Renamer;
use crate::neurospec::services::refactor::validator::Validator;
use crate::mcp::tools::unified_store::{with_global_store, is_search_initialized};

/// Arguments for neurospec.refactor.rename
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameArgs {
    /// Project root directory
    pub project_root: String,
    /// File path containing the symbol
    pub file_path: String,
    /// Current name of the symbol
    pub old_name: String,
    /// New name for the symbol
    pub new_name: String,
    /// Symbol kind (function, class, etc.)
    #[serde(default = "default_kind")]
    pub kind: String,
}

fn default_kind() -> String {
    "function".to_string()
}

/// Arguments for neurospec.refactor.safe_edit
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SafeEditArgs {
    /// File path to edit
    pub file_path: String,
    /// Target symbol name
    pub target_symbol: String,
    /// Replacement code
    pub replacement_code: String,
    /// Language (rust, typescript, python)
    pub language: String,
}

pub fn handle_rename(args: RenameArgs) -> Result<Vec<Content>, McpError> {
    // 优先使用全局 Store（增量索引，性能更好）
    let graph = if is_search_initialized() {
        with_global_store(|store| {
            GraphBuilder::build_from_store(&args.project_root, store)
        })
        .map_err(|e| McpError::internal_error(format!("Failed to build graph from store: {}", e), None))?
    } else {
        // 回退到直接扫描
        GraphBuilder::build_from_project(&args.project_root)
    };

    // Parse symbol kind
    let kind = match args.kind.as_str() {
        "function" => SymbolKind::Function,
        "class" => SymbolKind::Class,
        "module" => SymbolKind::Module,
        _ => SymbolKind::Function,
    };

    // Perform rename
    let result = Renamer::rename_symbol(
        &graph,
        &args.file_path,
        &args.old_name,
        &args.new_name,
        kind,
    )
    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    if !result.success {
        return Err(McpError::internal_error(
            result.error.unwrap_or_else(|| "Rename failed".to_string()),
            None,
        ));
    }

    // Validate all modified files
    for file in &result.modified_files {
        // Infer language from file extension
        let lang = if file.ends_with(".rs") {
            "rust"
        } else if file.ends_with(".ts") || file.ends_with(".js") {
            "typescript"
        } else if file.ends_with(".py") {
            "python"
        } else {
            continue;
        };

        let is_valid = Validator::validate_file(file, lang)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if !is_valid {
            return Err(McpError::internal_error(
                format!("Syntax errors introduced in {}", file),
                None,
            ));
        }
    }

    // Format result
    let summary = format!(
        "Renamed '{}' to '{}'\nModified {} file(s):\n- {}",
        args.old_name,
        args.new_name,
        result.modified_files.len(),
        result.modified_files.join("\n- ")
    );

    Ok(vec![Content::text(summary)])
}

pub fn handle_safe_edit(args: SafeEditArgs) -> Result<Vec<Content>, McpError> {
    // Read original file
    let content = std::fs::read_to_string(&args.file_path)
        .map_err(|e| McpError::internal_error(format!("Failed to read file: {}", e), None))?;

    // Use tree-sitter to find the target symbol's boundaries
    use tree_sitter::{Parser, Query, QueryCursor};

    let mut parser = Parser::new();
    let language = match args.language.as_str() {
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        "typescript" | "javascript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "python" => tree_sitter_python::LANGUAGE.into(),
        _ => {
            return Err(McpError::invalid_params(
                "Unsupported language".to_string(),
                None,
            ))
        }
    };

    parser
        .set_language(&language)
        .map_err(|e| McpError::internal_error(format!("Parser error: {}", e), None))?;

    let tree = parser
        .parse(&content, None)
        .ok_or_else(|| McpError::internal_error("Failed to parse file".to_string(), None))?;

    // Find function/class with matching name
    let root = tree.root_node();
    let query_str = match args.language.as_str() {
        "rust" => format!(
            r#"(function_item name: (identifier) @name (#eq? @name "{}")) @def"#,
            args.target_symbol
        ),
        "typescript" | "javascript" => format!(
            r#"(function_declaration name: (identifier) @name (#eq? @name "{}")) @def"#,
            args.target_symbol
        ),
        "python" => format!(
            r#"(function_definition name: (identifier) @name (#eq? @name "{}")) @def"#,
            args.target_symbol
        ),
        _ => {
            return Err(McpError::invalid_params(
                "Unsupported language".to_string(),
                None,
            ))
        }
    };

    let query = Query::new(&language, &query_str)
        .map_err(|e| McpError::internal_error(format!("Query error: {}", e), None))?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, root, content.as_bytes());

    let mut target_range = None;
    while let Some(match_) = matches.next() {
        for capture in match_.captures {
            if query.capture_names()[capture.index as usize] == "def" {
                let node = capture.node;
                target_range = Some(node.start_byte()..node.end_byte());
                break;
            }
        }
        if target_range.is_some() {
            break;
        }
    }

    let range = target_range.ok_or_else(|| {
        McpError::invalid_params(
            format!("Symbol '{}' not found in file", args.target_symbol),
            None,
        )
    })?;

    // Apply replacement
    let mut new_content = content.clone();
    new_content.replace_range(range, &args.replacement_code);

    // Validate syntax
    std::fs::write(&args.file_path, &new_content)
        .map_err(|e| McpError::internal_error(format!("Failed to write file: {}", e), None))?;

    let is_valid = Validator::validate_file(&args.file_path, &args.language)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    if !is_valid {
        // Rollback
        std::fs::write(&args.file_path, &content)
            .map_err(|e| McpError::internal_error(format!("Rollback failed: {}", e), None))?;

        return Err(McpError::internal_error(
            "Syntax errors introduced by edit, changes rolled back".to_string(),
            None,
        ));
    }

    Ok(vec![Content::text(format!(
        "Successfully edited '{}' in {}",
        args.target_symbol, args.file_path
    ))])
}
