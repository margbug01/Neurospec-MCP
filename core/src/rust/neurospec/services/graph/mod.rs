pub mod builder;

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::neurospec::models::{Symbol, SymbolKind};

/// Type of relationship between symbols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    /// A calls B
    Calls,
    /// A defines B (e.g., module defines function)
    Defines,
    /// A imports B
    Imports,
    /// A inherits from B
    Inherits,
    /// A references B (general usage)
    References,
}

/// Node in the code knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    pub id: String,
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub language: String,
    pub signature: Option<String>,
    // Store the original symbol for reference if needed
    // pub original_symbol: Symbol,
}

impl SymbolNode {
    pub fn from_symbol(symbol: &Symbol) -> Self {
        // Generate a deterministic ID based on path and name
        // In a real implementation, we might want something more robust to renaming
        let id = format!("{}::{}", symbol.path, symbol.name);

        Self {
            id,
            name: symbol.name.clone(),
            kind: symbol.kind.clone(),
            file_path: symbol.path.clone(),
            language: symbol.language.clone().unwrap_or_default(),
            signature: symbol.signature.clone(),
        }
    }
}

/// The Code Knowledge Graph
pub struct CodeGraph {
    pub graph: DiGraph<SymbolNode, RelationType>,
    pub node_map: HashMap<String, NodeIndex>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a symbol to the graph if it doesn't exist
    pub fn add_symbol(&mut self, symbol: &Symbol) -> NodeIndex {
        let node = SymbolNode::from_symbol(symbol);

        if let Some(&idx) = self.node_map.get(&node.id) {
            return idx;
        }

        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.node_map.insert(id, idx);
        idx
    }

    /// Add a relationship between two symbols
    pub fn add_relation(&mut self, from: &Symbol, to: &Symbol, relation: RelationType) {
        let from_idx = self.add_symbol(from);
        let to_idx = self.add_symbol(to);

        // Check if edge already exists to avoid duplicates
        if !self.graph.contains_edge(from_idx, to_idx) {
            self.graph.add_edge(from_idx, to_idx, relation);
        }
    }

    /// Add a relationship by ID (useful when we only have the target name/path)
    pub fn add_relation_by_id(
        &mut self,
        from_idx: NodeIndex,
        target_id: &str,
        relation: RelationType,
    ) {
        if let Some(&to_idx) = self.node_map.get(target_id) {
            if !self.graph.contains_edge(from_idx, to_idx) {
                self.graph.add_edge(from_idx, to_idx, relation);
            }
        }
        // If target doesn't exist yet, we might want to create a "Ghost" node or queue it
        // For now, we skip it
    }
}
