use std::path::Path;
use anyhow::Result;
use tree_sitter::{Parser, Node};
use super::types::{Symbol, SymbolKind, Language};

pub fn detect_language(path: &Path) -> Language {
    match path.extension().and_then(|s| s.to_str()) {
        // Rust
        Some("rs") => Language::Rust,
        // TypeScript / JavaScript
        Some("ts") | Some("tsx") | Some("mts") | Some("cts") => Language::TypeScript,
        Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => Language::JavaScript,
        // Python
        Some("py") | Some("pyi") => Language::Python,
        // Vue / Svelte (extract script section)
        Some("vue") | Some("svelte") => Language::TypeScript,
        // Config files (treat as text, no symbol extraction)
        Some("json") | Some("yaml") | Some("yml") | Some("toml") | Some("md") => Language::Unknown,
        _ => Language::Unknown,
    }
}

pub fn extract_symbols(path: &Path, content: &str) -> Result<Vec<Symbol>> {
    let lang = detect_language(path);
    if let Language::Unknown = lang {
        return Ok(Vec::new());
    }

    // 对于 Vue/Svelte 文件，提取 script 部分
    let effective_content = if path.extension().and_then(|s| s.to_str()) == Some("vue")
        || path.extension().and_then(|s| s.to_str()) == Some("svelte")
    {
        extract_script_content(content)
    } else {
        content.to_string()
    };

    let mut parser = Parser::new();
    match lang {
        Language::Rust => parser.set_language(&tree_sitter_rust::LANGUAGE.into())?,
        Language::TypeScript => {
            parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?
        }
        Language::JavaScript => {
            parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?
        }
        Language::Python => parser.set_language(&tree_sitter_python::LANGUAGE.into())?,
        _ => return Ok(Vec::new()),
    };

    let tree = match parser.parse(&effective_content, None) {
        Some(t) => t,
        None => return Ok(Vec::new()),
    };

    let mut symbols = Vec::new();
    walk_tree(&tree.root_node(), &effective_content, &lang, &mut symbols);
    Ok(symbols)
}

/// 从 Vue/Svelte SFC 中提取 script 内容
fn extract_script_content(content: &str) -> String {
    // 简单的正则匹配 <script> 标签内容
    let script_start = content.find("<script");
    let script_end = content.find("</script>");

    if let (Some(start), Some(end)) = (script_start, script_end) {
        // 找到 > 的位置
        if let Some(tag_end) = content[start..].find('>') {
            let content_start = start + tag_end + 1;
            if content_start < end {
                return content[content_start..end].to_string();
            }
        }
    }

    // 如果没找到 script 标签，返回原内容
    content.to_string()
}

fn walk_tree(node: &Node, source: &str, lang: &Language, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(symbol) = map_node_to_symbol(&child, source, lang) {
            symbols.push(symbol);
        }
        walk_tree(&child, source, lang, symbols);
    }
}

fn map_node_to_symbol(node: &Node, source: &str, lang: &Language) -> Option<Symbol> {
    let kind = node.kind();
    
    let (symbol_kind, name_node) = match lang {
        Language::Rust => match kind {
            "function_item" => (SymbolKind::Function, node.child_by_field_name("name")),
            "struct_item" => (SymbolKind::Struct, node.child_by_field_name("name")),
            "trait_item" => (SymbolKind::Interface, node.child_by_field_name("name")),
            "impl_item" => (SymbolKind::Other, node.child_by_field_name("type")),
            _ => return None,
        },
        Language::TypeScript | Language::JavaScript => match kind {
            "function_declaration" => (SymbolKind::Function, node.child_by_field_name("name")),
            "class_declaration" => (SymbolKind::Class, node.child_by_field_name("name")),
            "method_definition" => (SymbolKind::Method, node.child_by_field_name("name")),
            "interface_declaration" => (SymbolKind::Interface, node.child_by_field_name("name")),
            _ => return None,
        },
        Language::Python => match kind {
            "function_definition" => (SymbolKind::Function, node.child_by_field_name("name")),
            "class_definition" => (SymbolKind::Class, node.child_by_field_name("name")),
            _ => return None,
        },
        _ => return None,
    };
    
    if let Some(name_node) = name_node {
        let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();
        // Calculate line number (0-indexed to 1-indexed)
        let line = node.start_position().row + 1;
        
        Some(Symbol {
            name,
            kind: symbol_kind,
            line,
        })
    } else {
        None
    }
}