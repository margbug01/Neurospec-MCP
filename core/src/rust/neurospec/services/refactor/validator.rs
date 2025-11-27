use log::{info, warn};
use std::fs;
use tree_sitter::{Language, Parser};

/// Validator for ensuring code correctness after refactoring
pub struct Validator;

impl Validator {
    /// Validate that a file has correct syntax
    pub fn validate_file(file_path: &str, language: &str) -> anyhow::Result<bool> {
        info!("Validating syntax for file: {}", file_path);

        // Read file content
        let content = fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

        // Get appropriate parser
        let mut parser = Parser::new();
        let lang = Self::get_language(language)?;
        parser
            .set_language(&lang)
            .map_err(|e| anyhow::anyhow!("Failed to set language: {}", e))?;

        // Parse
        let tree = parser
            .parse(&content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse file"))?;

        // Check for errors
        let root = tree.root_node();
        let has_error = Self::check_for_errors(&root);

        if has_error {
            warn!("Syntax errors found in {}", file_path);
            return Ok(false);
        }

        info!("File {} is syntactically valid", file_path);
        Ok(true)
    }

    /// Get tree-sitter language for a given language string
    fn get_language(language: &str) -> anyhow::Result<Language> {
        match language {
            "rust" => Ok(tree_sitter_rust::LANGUAGE.into()),
            "typescript" | "javascript" => Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            "python" => Ok(tree_sitter_python::LANGUAGE.into()),
            _ => Err(anyhow::anyhow!("Unsupported language: {}", language)),
        }
    }

    /// Recursively check for error nodes in the AST
    fn check_for_errors(node: &tree_sitter::Node) -> bool {
        if node.is_error() || node.is_missing() {
            return true;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if Self::check_for_errors(&child) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_valid_rust() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "fn main() {{ println!(\"Hello\"); }}").unwrap();

        let result = Validator::validate_file(file.path().to_str().unwrap(), "rust");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_validate_invalid_rust() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "fn main( {{ println!(\"Hello\"); }}").unwrap(); // Missing )

        let result = Validator::validate_file(file.path().to_str().unwrap(), "rust");
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
