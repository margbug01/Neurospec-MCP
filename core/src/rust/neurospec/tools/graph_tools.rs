use rmcp::{model::Content, ErrorData as McpError};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::neurospec::services::graph::builder::GraphBuilder;
use crate::neurospec::services::graph::RelationType;
use crate::mcp::tools::unified_store::{with_global_store, is_search_initialized};

/// Arguments for neurospec.graph.impact_analysis
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImpactAnalysisArgs {
    /// Project root directory path
    pub project_root: String,
    /// Symbol name or ID to analyze
    pub symbol_name: String,
    /// Max depth for analysis (default: 1)
    pub depth: Option<usize>,
}

pub fn handle_impact_analysis(
    args: ImpactAnalysisArgs,
) -> Result<Vec<Content>, McpError> {
    // 优先使用全局 Store（增量索引，性能更好）
    let graph = if is_search_initialized() {
        with_global_store(|store| {
            GraphBuilder::build_from_store(&args.project_root, store)
        })
        .map_err(|e| McpError::internal_error(format!("Failed to build graph from store: {}", e), None))?
    } else {
        // 回退到直接扫描（兼容 MCP 独立运行）
        GraphBuilder::build_from_project(&args.project_root)
    };

    // Find the node for the symbol
    // We search by name since ID might be complex
    let mut target_indices = Vec::new();
    for (id, idx) in &graph.node_map {
        if id.ends_with(&format!("::{}", args.symbol_name)) || id == &args.symbol_name {
            target_indices.push(*idx);
        }
    }

    if target_indices.is_empty() {
        return Err(McpError::invalid_params(
            format!("Symbol '{}' not found in project", args.symbol_name),
            None,
        ));
    }

    let depth = args.depth.unwrap_or(1);
    let mut impacted_symbols = Vec::new();

    // Find all nodes that depend on (call) the target nodes
    // We traverse edges in reverse direction of 'Calls'
    // Or if A calls B, then B impacts A? No, if B changes, A is impacted.
    // So we look for incoming edges of type 'Calls' to B.

    use petgraph::Direction;

    for target_idx in target_indices {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((target_idx, 0));
        visited.insert(target_idx);

        while let Some((idx, d)) = queue.pop_front() {
            if d >= depth {
                continue;
            }

            // Find who calls 'idx'
            let mut neighbors = graph
                .graph
                .neighbors_directed(idx, Direction::Incoming)
                .detach();
            while let Some(neighbor_idx) = neighbors.next_node(&graph.graph) {
                if visited.contains(&neighbor_idx) {
                    continue;
                }

                // Check edge type
                let edge = graph.graph.find_edge(neighbor_idx, idx).unwrap();
                let relation = graph.graph.edge_weight(edge).unwrap();

                if *relation == RelationType::Calls {
                    if let Some(node) = graph.graph.node_weight(neighbor_idx) {
                        impacted_symbols
                            .push(format!("{} ({}) in {}", node.name, node.id, node.file_path));
                        visited.insert(neighbor_idx);
                        queue.push_back((neighbor_idx, d + 1));
                    }
                }
            }
        }
    }

    let result = if impacted_symbols.is_empty() {
        "No impacted symbols found.".to_string()
    } else {
        format!(
            "Impacted symbols (Depth {}):\n- {}",
            depth,
            impacted_symbols.join("\n- ")
        )
    };

    Ok(vec![Content::text(result)])
}
