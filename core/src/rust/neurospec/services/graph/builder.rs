use ignore::WalkBuilder;
use log::info;
use std::collections::HashMap;

use crate::neurospec::models::Symbol;
use crate::neurospec::services::analyzer::analyze_file_thread_local;
use crate::neurospec::services::graph::{CodeGraph, RelationType};

pub struct GraphBuilder;

impl GraphBuilder {
    /// Build a CodeGraph from a project directory
    pub fn build_from_project(project_root: &str) -> CodeGraph {
        let mut graph = CodeGraph::new();
        let mut symbols_by_name: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_symbols: Vec<Symbol> = Vec::new();

        info!("Building graph for project: {}", project_root);

        // 1. First Pass: Collect all symbols
        // 使用 ignore crate 遵守 .gitignore，避免扫描 node_modules/dist 等目录
        let walker = WalkBuilder::new(project_root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();

        for entry in walker
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

            let language = match ext {
                "rs" => "rust",
                "ts" | "js" | "tsx" | "jsx" => "typescript",
                "py" => "python",
                _ => continue,
            };

            if let Ok(content) = std::fs::read_to_string(path) {
                let symbols =
                    analyze_file_thread_local(path, &content, language);

                for symbol in symbols {
                    // Add to graph
                    let _node_idx = graph.add_symbol(&symbol);

                    // Index by name for resolution
                    symbols_by_name
                        .entry(symbol.name.clone())
                        .or_default()
                        .push(symbol.path.clone());

                    all_symbols.push(symbol);
                }
            }
        }

        // 2. Second Pass: Link references
        for symbol in all_symbols {
            let from_id = format!("{}::{}", symbol.path, symbol.name);

            if let Some(from_idx) = graph.node_map.get(&from_id).cloned() {
                for ref_name in &symbol.references {
                    // Try to resolve ref_name
                    if let Some(target_paths) = symbols_by_name.get(ref_name) {
                        // Simple resolution strategy:
                        // 1. Prefer symbol in same file
                        // 2. Prefer symbol in same directory (module)
                        // 3. Pick first available (naive)

                        // Check same file first, fallback to first available
                        let target_path = if target_paths.contains(&symbol.path) {
                            Some(symbol.path.clone())
                        } else {
                            target_paths.first().cloned()
                        };

                        if let Some(path) = target_path {
                            let target_id = format!("{}::{}", path, ref_name);
                            graph.add_relation_by_id(from_idx, &target_id, RelationType::Calls);
                        }
                    }
                }
            }
        }

        info!(
            "Graph built with {} nodes and {} edges",
            graph.graph.node_count(),
            graph.graph.edge_count()
        );
        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::neurospec::models::{Symbol, SymbolKind};

    #[test]
    fn test_graph_builder_linking() {
        let mut graph = CodeGraph::new();

        // Create Symbol A (Caller)
        let sym_a = Symbol {
            kind: SymbolKind::Function,
            name: "caller_func".to_string(),
            path: "src/main.rs".to_string(),
            language: Some("rust".to_string()),
            signature: None,
            references: vec!["callee_func".to_string()],
        };

        // Create Symbol B (Callee)
        let sym_b = Symbol {
            kind: SymbolKind::Function,
            name: "callee_func".to_string(),
            path: "src/utils.rs".to_string(),
            language: Some("rust".to_string()),
            signature: None,
            references: vec![],
        };

        // Add symbols manually (simulating builder pass 1)
        let idx_a = graph.add_symbol(&sym_a);
        let idx_b = graph.add_symbol(&sym_b);

        // Add relation manually (simulating builder pass 2)
        let target_id = format!("{}::{}", sym_b.path, sym_b.name);
        graph.add_relation_by_id(idx_a, &target_id, RelationType::Calls);

        // Verify edge
        assert!(graph.graph.contains_edge(idx_a, idx_b));
    }
}


impl GraphBuilder {
    /// 从 X-Ray 快照构建图谱（推荐，复用已有数据）
    ///
    /// 相比 build_from_project，此方法：
    /// - 复用 X-Ray 已提取的符号，避免重复扫描
    /// - 性能更好，特别是与 scan_project_cached 配合使用时
    pub fn build_from_xray(snapshot: &crate::neurospec::models::XRaySnapshot) -> CodeGraph {
        let mut graph = CodeGraph::new();
        let mut symbols_by_name: HashMap<String, Vec<String>> = HashMap::new();

        info!("Building graph from X-Ray snapshot: {}", snapshot.project_root);

        // 1. First Pass: Add all symbols to graph
        for symbol in &snapshot.symbols {
            let _node_idx = graph.add_symbol(symbol);

            // Index by name for resolution
            symbols_by_name
                .entry(symbol.name.clone())
                .or_default()
                .push(symbol.path.clone());
        }

        // 2. Second Pass: Link references
        for symbol in &snapshot.symbols {
            let from_id = format!("{}::{}", symbol.path, symbol.name);

            if let Some(from_idx) = graph.node_map.get(&from_id).cloned() {
                for ref_name in &symbol.references {
                    if let Some(target_paths) = symbols_by_name.get(ref_name) {
                        // Prefer symbol in same file, fallback to first
                        let target_path = if target_paths.contains(&symbol.path) {
                            symbol.path.clone()
                        } else {
                            target_paths.first().cloned().unwrap_or_default()
                        };

                        if !target_path.is_empty() {
                            let target_id = format!("{}::{}", target_path, ref_name);
                            graph.add_relation_by_id(from_idx, &target_id, RelationType::Calls);
                        }
                    }
                }
            }
        }

        info!(
            "Graph built with {} nodes and {} edges",
            graph.graph.node_count(),
            graph.graph.edge_count()
        );
        graph
    }

    /// 从统一存储构建图谱（最优方案）
    ///
    /// 结合 UnifiedSymbolStore 的增量索引能力
    pub fn build_from_store(
        project_root: &str,
        store: &crate::mcp::tools::unified_store::UnifiedSymbolStore,
    ) -> anyhow::Result<CodeGraph> {
        use std::path::Path;
        
        // 先获取 X-Ray 快照
        let snapshot = crate::neurospec::services::xray_engine::scan_project_cached(
            Path::new(project_root),
            store,
        )?;

        // 复用 build_from_xray
        Ok(Self::build_from_xray(&snapshot))
    }
}
