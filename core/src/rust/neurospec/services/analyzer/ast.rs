use log::{debug, warn};
use std::cell::RefCell;
use std::path::Path;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

use crate::neurospec::models::{Symbol, SymbolKind};

extern "C" {
    fn tree_sitter_rust() -> Language;
}
extern "C" {
    fn tree_sitter_typescript() -> Language;
}
extern "C" {
    fn tree_sitter_python() -> Language;
}

/// AST-based code analyzer using tree-sitter
pub struct AstAnalyzer {
    rust_parser: Parser,
    typescript_parser: Parser,
    python_parser: Parser,

    rust_lang: Language,
    typescript_lang: Language,
    python_lang: Language,
}

impl AstAnalyzer {
    /// Create a new AstAnalyzer with initialized parsers
    pub fn new() -> Result<Self, String> {
        let rust_lang = unsafe { tree_sitter_rust() };
        let typescript_lang = unsafe { tree_sitter_typescript() };
        let python_lang = unsafe { tree_sitter_python() };

        let mut rust_parser = Parser::new();
        rust_parser
            .set_language(&rust_lang)
            .map_err(|e| format!("Failed to set Rust language: {}", e))?;

        let mut typescript_parser = Parser::new();
        typescript_parser
            .set_language(&typescript_lang)
            .map_err(|e| format!("Failed to set TypeScript language: {}", e))?;

        let mut python_parser = Parser::new();
        python_parser
            .set_language(&python_lang)
            .map_err(|e| format!("Failed to set Python language: {}", e))?;

        Ok(Self {
            rust_parser,
            typescript_parser,
            python_parser,
            rust_lang,
            typescript_lang,
            python_lang,
        })
    }

    /// Analyze a file and extract symbols
    pub fn analyze_file(&mut self, path: &Path, content: &str, language: &str) -> Vec<Symbol> {
        let rel_path = path.to_string_lossy().replace("\\", "/");

        match language {
            "rust" => self.analyze_rust(&rel_path, content),
            "typescript" | "javascript" => self.analyze_typescript(&rel_path, content),
            "python" => self.analyze_python(&rel_path, content),
            _ => Vec::new(),
        }
    }

    /// Analyze Rust code
    fn analyze_rust(&mut self, path: &str, content: &str) -> Vec<Symbol> {
        let tree = match self.rust_parser.parse(content, None) {
            Some(t) => t,
            None => {
                warn!("Failed to parse Rust file: {}", path);
                return Vec::new();
            }
        };

        let root_node = tree.root_node();
        let symbols = Vec::new();

        // 1. Extract Definitions
        let def_query_str = r#"
            (struct_item name: (type_identifier) @struct.name) @struct.def
            (enum_item name: (type_identifier) @enum.name) @enum.def
            (function_item name: (identifier) @function.name) @function.def
            (impl_item type: (_) @impl.type) @impl.def
        "#;

        let def_query = match Query::new(&self.rust_lang, def_query_str) {
            Ok(q) => q,
            Err(e) => {
                warn!("Failed to create Rust def query: {}", e);
                return symbols;
            }
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&def_query, root_node, content.as_bytes());

        // Store definitions with their ranges
        struct DefInfo {
            symbol: Symbol,
            range: std::ops::Range<usize>,
        }
        let mut definitions: Vec<DefInfo> = Vec::new();

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let capture_name = &def_query.capture_names()[capture.index as usize];
                let node = capture.node;

                if !capture_name.ends_with(".name") && !capture_name.ends_with(".type") {
                    continue;
                }

                let text = node.utf8_text(content.as_bytes()).unwrap_or("").to_string();
                let kind = if capture_name.starts_with("struct") || capture_name.starts_with("enum")
                {
                    SymbolKind::Class
                } else if capture_name.starts_with("function") {
                    SymbolKind::Function
                } else {
                    continue;
                };

                // Find the full definition node (parent of the name node usually)
                let def_node = node.parent().unwrap_or(node);
                let range = def_node.start_byte()..def_node.end_byte();

                // Extract signature
                let signature = def_node
                    .utf8_text(content.as_bytes())
                    .ok()
                    .and_then(|s| s.lines().next().map(|l| l.trim().to_string()));

                definitions.push(DefInfo {
                    symbol: Symbol {
                        kind,
                        name: text,
                        path: path.to_string(),
                        language: Some("rust".to_string()),
                        signature,
                        references: Vec::new(),
                    },
                    range,
                });
            }
        }

        // 2. Extract Calls
        let call_query_str = r#"
            (call_expression function: (identifier) @call.name)
            (call_expression function: (field_expression field: (field_identifier) @call.method))
            (generic_function function: (identifier) @call.generic)
            (macro_invocation macro: (identifier) @call.macro)
        "#;

        let call_query = match Query::new(&self.rust_lang, call_query_str) {
            Ok(q) => q,
            Err(e) => {
                warn!("Failed to create Rust call query: {}", e);
                // Return definitions found so far
                return definitions.into_iter().map(|d| d.symbol).collect();
            }
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&call_query, root_node, content.as_bytes());

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let call_name = node.utf8_text(content.as_bytes()).unwrap_or("").to_string();
                let call_pos = node.start_byte();

                let mut best_def_idx = None;
                let mut min_len = usize::MAX;

                for (i, def) in definitions.iter().enumerate() {
                    if def.range.contains(&call_pos) {
                        let len = def.range.len();
                        if len < min_len {
                            min_len = len;
                            best_def_idx = Some(i);
                        }
                    }
                }

                if let Some(idx) = best_def_idx {
                    definitions[idx].symbol.references.push(call_name);
                }
            }
        }

        debug!(
            "Extracted {} symbols from Rust file: {}",
            definitions.len(),
            path
        );
        definitions.into_iter().map(|d| d.symbol).collect()
    }

    /// Analyze TypeScript/JavaScript code
    fn analyze_typescript(&mut self, path: &str, content: &str) -> Vec<Symbol> {
        let tree = match self.typescript_parser.parse(content, None) {
            Some(t) => t,
            None => {
                warn!("Failed to parse TypeScript file: {}", path);
                return Vec::new();
            }
        };

        let root_node = tree.root_node();

        // 1. Extract Definitions
        let def_query_str = r#"
            (class_declaration name: (type_identifier) @class.name)
            (interface_declaration name: (type_identifier) @interface.name)
            (function_declaration name: (identifier) @function.name)
            (method_definition name: (property_identifier) @method.name)
            (public_field_definition name: (property_identifier) @field.name)
        "#;

        let def_query = match Query::new(&self.typescript_lang, def_query_str) {
            Ok(q) => q,
            Err(e) => {
                warn!("Failed to create TypeScript def query: {}", e);
                return Vec::new();
            }
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&def_query, root_node, content.as_bytes());

        struct DefInfo {
            symbol: Symbol,
            range: std::ops::Range<usize>,
        }
        let mut definitions: Vec<DefInfo> = Vec::new();

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let capture_name = &def_query.capture_names()[capture.index as usize];
                let node = capture.node;
                let text = node.utf8_text(content.as_bytes()).unwrap_or("").to_string();

                let kind = if capture_name.contains("class") || capture_name.contains("interface") {
                    SymbolKind::Class
                } else if capture_name.contains("function") || capture_name.contains("method") {
                    SymbolKind::Function
                } else {
                    continue;
                };

                let def_node = node.parent().unwrap_or(node);
                let range = def_node.start_byte()..def_node.end_byte();

                let signature = def_node
                    .utf8_text(content.as_bytes())
                    .ok()
                    .and_then(|s| s.lines().next().map(|l| l.trim().to_string()));

                definitions.push(DefInfo {
                    symbol: Symbol {
                        kind,
                        name: text,
                        path: path.to_string(),
                        language: Some("typescript".to_string()),
                        signature,
                        references: Vec::new(),
                    },
                    range,
                });
            }
        }

        // 2. Extract Calls
        let call_query_str = r#"
            (call_expression function: (identifier) @call.name)
            (call_expression function: (member_expression property: (property_identifier) @call.method))
            (new_expression constructor: (identifier) @call.new)
        "#;

        let call_query = match Query::new(&self.typescript_lang, call_query_str) {
            Ok(q) => q,
            Err(e) => {
                warn!("Failed to create TypeScript call query: {}", e);
                return definitions.into_iter().map(|d| d.symbol).collect();
            }
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&call_query, root_node, content.as_bytes());

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let call_name = node.utf8_text(content.as_bytes()).unwrap_or("").to_string();
                let call_pos = node.start_byte();

                let mut best_def_idx = None;
                let mut min_len = usize::MAX;

                for (i, def) in definitions.iter().enumerate() {
                    if def.range.contains(&call_pos) {
                        let len = def.range.len();
                        if len < min_len {
                            min_len = len;
                            best_def_idx = Some(i);
                        }
                    }
                }

                if let Some(idx) = best_def_idx {
                    definitions[idx].symbol.references.push(call_name);
                }
            }
        }

        debug!(
            "Extracted {} symbols from TypeScript file: {}",
            definitions.len(),
            path
        );
        definitions.into_iter().map(|d| d.symbol).collect()
    }

    /// Analyze Python code
    fn analyze_python(&mut self, path: &str, content: &str) -> Vec<Symbol> {
        let tree = match self.python_parser.parse(content, None) {
            Some(t) => t,
            None => {
                warn!("Failed to parse Python file: {}", path);
                return Vec::new();
            }
        };

        let root_node = tree.root_node();

        // 1. Extract Definitions
        let def_query_str = r#"
            (class_definition name: (identifier) @class.name)
            (function_definition name: (identifier) @function.name)
        "#;

        let def_query = match Query::new(&self.python_lang, def_query_str) {
            Ok(q) => q,
            Err(e) => {
                warn!("Failed to create Python def query: {}", e);
                return Vec::new();
            }
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&def_query, root_node, content.as_bytes());

        struct DefInfo {
            symbol: Symbol,
            range: std::ops::Range<usize>,
        }
        let mut definitions: Vec<DefInfo> = Vec::new();

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let capture_name = &def_query.capture_names()[capture.index as usize];
                let node = capture.node;
                let text = node.utf8_text(content.as_bytes()).unwrap_or("").to_string();

                let kind = if capture_name.contains("class") {
                    SymbolKind::Class
                } else {
                    SymbolKind::Function
                };

                let def_node = node.parent().unwrap_or(node);
                let range = def_node.start_byte()..def_node.end_byte();

                let signature = def_node
                    .utf8_text(content.as_bytes())
                    .ok()
                    .and_then(|s| s.lines().next().map(|l| l.trim().to_string()));

                definitions.push(DefInfo {
                    symbol: Symbol {
                        kind,
                        name: text,
                        path: path.to_string(),
                        language: Some("python".to_string()),
                        signature,
                        references: Vec::new(),
                    },
                    range,
                });
            }
        }

        // 2. Extract Calls
        let call_query_str = r#"
            (call function: (identifier) @call.name)
            (call function: (attribute attribute: (identifier) @call.method))
        "#;

        let call_query = match Query::new(&self.python_lang, call_query_str) {
            Ok(q) => q,
            Err(e) => {
                warn!("Failed to create Python call query: {}", e);
                return definitions.into_iter().map(|d| d.symbol).collect();
            }
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&call_query, root_node, content.as_bytes());

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let call_name = node.utf8_text(content.as_bytes()).unwrap_or("").to_string();
                let call_pos = node.start_byte();

                let mut best_def_idx = None;
                let mut min_len = usize::MAX;

                for (i, def) in definitions.iter().enumerate() {
                    if def.range.contains(&call_pos) {
                        let len = def.range.len();
                        if len < min_len {
                            min_len = len;
                            best_def_idx = Some(i);
                        }
                    }
                }

                if let Some(idx) = best_def_idx {
                    definitions[idx].symbol.references.push(call_name);
                }
            }
        }

        debug!(
            "Extracted {} symbols from Python file: {}",
            definitions.len(),
            path
        );
        definitions.into_iter().map(|d| d.symbol).collect()
    }
}

impl Default for AstAnalyzer {
    fn default() -> Self {
        Self::new().expect("Failed to initialize AstAnalyzer")
    }
}

// Thread-local storage for AstAnalyzer to enable parallel processing
thread_local! {
    static THREAD_ANALYZER: RefCell<Option<AstAnalyzer>> = RefCell::new(None);
}

/// Thread-safe helper function for parallel file analysis
///
/// Uses thread-local storage to maintain one AstAnalyzer per thread,
/// avoiding mutex contention and enabling efficient parallel processing.
pub fn analyze_file_thread_local(path: &Path, content: &str, language: &str) -> Vec<Symbol> {
    THREAD_ANALYZER.with(|analyzer_cell| {
        let mut analyzer_ref = analyzer_cell.borrow_mut();

        // Initialize the analyzer lazily on first access per thread
        if analyzer_ref.is_none() {
            match AstAnalyzer::new() {
                Ok(analyzer) => {
                    *analyzer_ref = Some(analyzer);
                }
                Err(e) => {
                    warn!("Failed to initialize thread-local AstAnalyzer: {}", e);
                    return Vec::new();
                }
            }
        }

        // Use the analyzer
        if let Some(ref mut analyzer) = *analyzer_ref {
            analyzer.analyze_file(path, content, language)
        } else {
            Vec::new()
        }
    })
}
