use log::info;
use std::fs;

use crate::neurospec::models::SymbolKind;
use crate::neurospec::services::graph::CodeGraph;
use crate::neurospec::services::refactor::{Edit, RefactorResult};

pub struct Renamer;

impl Renamer {
    /// Rename a symbol across the project
    ///
    /// # Arguments
    /// * `graph` - The code knowledge graph
    /// * `file_path` - File containing the symbol to rename
    /// * `old_name` - Current name of the symbol
    /// * `new_name` - New name for the symbol
    /// * `kind` - Type of symbol (Function, Class, etc.)
    pub fn rename_symbol(
        graph: &CodeGraph,
        file_path: &str,
        old_name: &str,
        new_name: &str,
        _kind: SymbolKind,
    ) -> anyhow::Result<RefactorResult> {
        info!(
            "Renaming symbol '{}' to '{}' in {}",
            old_name, new_name, file_path
        );

        // 1. Find the target symbol in the graph
        let symbol_id = format!("{}::{}", file_path, old_name);
        let target_idx = graph
            .node_map
            .get(&symbol_id)
            .ok_or_else(|| anyhow::anyhow!("Symbol '{}' not found in graph", old_name))?;

        // 2. Find all references using the graph (reverse edges)
        use petgraph::Direction;
        let mut edit_locations = Vec::new();

        // Add the definition itself
        if let Some(node) = graph.graph.node_weight(*target_idx) {
            edit_locations.push((node.file_path.clone(), node.name.clone()));
        }

        // Add all references (who calls this symbol)
        let mut neighbors = graph
            .graph
            .neighbors_directed(*target_idx, Direction::Incoming)
            .detach();
        while let Some(neighbor_idx) = neighbors.next_node(&graph.graph) {
            if let Some(node) = graph.graph.node_weight(neighbor_idx) {
                // Check if this node's references contain our symbol
                if node.file_path != file_path || node.name != old_name {
                    edit_locations.push((node.file_path.clone(), old_name.to_string()));
                }
            }
        }

        info!("Found {} locations to rename", edit_locations.len());

        // 3. Group by file and create edits
        use std::collections::HashMap;
        let mut edits_by_file: HashMap<String, Vec<Edit>> = HashMap::new();

        for (file, _) in edit_locations {
            // Read file content
            let content = fs::read_to_string(&file)
                .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", file, e))?;

            // Find all occurrences of old_name in this file
            // Simple approach: string search + boundary check
            let mut file_edits = Vec::new();
            for (idx, _) in content.match_indices(old_name) {
                // Basic boundary check (word boundaries)
                let before_ok = idx == 0 || !content.as_bytes()[idx - 1].is_ascii_alphanumeric();
                let after_ok = {
                    let end = idx + old_name.len();
                    end >= content.len() || !content.as_bytes()[end].is_ascii_alphanumeric()
                };

                if before_ok && after_ok {
                    file_edits.push(Edit::new(
                        file.clone(),
                        idx,
                        idx + old_name.len(),
                        new_name.to_string(),
                    ));
                }
            }

            if !file_edits.is_empty() {
                edits_by_file.insert(file, file_edits);
            }
        }

        // 4. Apply edits (reverse order per file to avoid offset issues)
        let mut modified_files = Vec::new();
        let mut all_edits = Vec::new();

        for (file, mut edits) in edits_by_file {
            // Sort edits in reverse order (end -> start)
            edits.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

            // Read original content
            let mut content = fs::read_to_string(&file)
                .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", file, e))?;

            // Apply edits in reverse order
            for edit in &edits {
                content.replace_range(edit.start_byte..edit.end_byte, &edit.replacement);
            }

            // Write back
            fs::write(&file, content)
                .map_err(|e| anyhow::anyhow!("Failed to write file {}: {}", file, e))?;

            info!("Modified file: {}", file);
            modified_files.push(file.clone());
            all_edits.extend(edits);
        }

        Ok(RefactorResult::success(modified_files, all_edits))
    }
}
