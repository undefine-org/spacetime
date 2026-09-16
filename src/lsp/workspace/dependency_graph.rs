//! Import dependency graph
//!
//! Tracks import relationships between files for:
//! - Cycle detection
//! - Invalidation (when file B changes, re-index all files that import B)
//! - Topological ordering for compilation

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::diagnostics::SourceSpan;

/// Metadata for a file node in the graph
#[derive(Debug, Clone)]
pub struct FileNode {
    /// Canonical file path
    pub path: PathBuf,
    /// Hash of content for change detection (0 = not computed)
    pub content_hash: u64,
}

/// Metadata for an import edge
#[derive(Debug, Clone)]
pub struct ImportEdge {
    /// The raw import path as written
    pub import_path: String,
    /// Source span of the import statement
    pub span: SourceSpan,
}

/// Dependency graph tracking import relationships
pub struct DependencyGraph {
    /// Directed graph: edges point from importer → imported
    graph: DiGraph<FileNode, ImportEdge>,
    /// Quick lookup: path → node index
    path_to_node: HashMap<PathBuf, NodeIndex>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            path_to_node: HashMap::new(),
        }
    }

    /// Add or update a file node in the graph
    pub fn upsert_file(&mut self, path: PathBuf, content_hash: u64) -> NodeIndex {
        if let Some(&idx) = self.path_to_node.get(&path) {
            // Update existing node
            self.graph[idx].content_hash = content_hash;
            idx
        } else {
            // Add new node
            let node = FileNode {
                path: path.clone(),
                content_hash,
            };
            let idx = self.graph.add_node(node);
            self.path_to_node.insert(path, idx);
            idx
        }
    }

    /// Get node index for a path
    pub fn get_node(&self, path: &Path) -> Option<NodeIndex> {
        self.path_to_node.get(path).copied()
    }

    /// Update imports for a file (replaces all outgoing edges)
    pub fn update_imports(&mut self, from_path: &Path, imports: Vec<(PathBuf, ImportEdge)>) {
        let from_idx = match self.path_to_node.get(from_path) {
            Some(&idx) => idx,
            None => return,
        };

        // Remove old outgoing edges
        let old_edges: Vec<_> = self.graph.edges(from_idx).map(|e| e.id()).collect();
        for edge_id in old_edges {
            self.graph.remove_edge(edge_id);
        }

        // Add new edges
        for (to_path, edge) in imports {
            // Ensure target node exists
            let to_idx = self.upsert_file(to_path, 0);
            self.graph.add_edge(from_idx, to_idx, edge);
        }
    }

    /// Check if adding an import would create a cycle
    pub fn would_create_cycle(&self, from: &Path, to: &Path) -> bool {
        // A cycle would be created if there's already a path from `to` to `from`
        self.has_path(to, from)
    }

    /// Check if there's a path from `from` to `to` in the graph
    fn has_path(&self, from: &Path, to: &Path) -> bool {
        let from_idx = match self.path_to_node.get(from) {
            Some(&idx) => idx,
            None => return false,
        };
        let to_idx = match self.path_to_node.get(to) {
            Some(&idx) => idx,
            None => return false,
        };

        // BFS from `from` looking for `to`
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from_idx);

        while let Some(current) = queue.pop_front() {
            if current == to_idx {
                return true;
            }
            if visited.insert(current) {
                for neighbor in self.graph.neighbors(current) {
                    queue.push_back(neighbor);
                }
            }
        }

        false
    }

    /// Detect any cycle involving the given file, returning the cycle path if found
    pub fn detect_cycle(&self, start: &Path) -> Option<Vec<PathBuf>> {
        let start_idx = self.path_to_node.get(start)?;

        // DFS with path tracking
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        let mut stack = vec![(*start_idx, false)];

        while let Some((node_idx, backtracking)) = stack.pop() {
            if backtracking {
                path.pop();
                continue;
            }

            let node = &self.graph[node_idx];

            // Check if we've seen this node in current path (cycle!)
            if let Some(pos) = path.iter().position(|p| p == &node.path) {
                let mut cycle: Vec<PathBuf> = path[pos..].to_vec();
                cycle.push(node.path.clone());
                return Some(cycle);
            }

            if visited.contains(&node_idx) {
                continue;
            }

            visited.insert(node_idx);
            path.push(node.path.clone());
            stack.push((node_idx, true)); // Mark for backtracking

            // Visit neighbors (imports)
            for neighbor in self.graph.neighbors(node_idx) {
                stack.push((neighbor, false));
            }
        }

        None
    }

    /// Get all files that directly or transitively import the given file
    pub fn get_dependents(&self, path: &Path) -> HashSet<PathBuf> {
        let mut dependents = HashSet::new();
        let target_idx = match self.path_to_node.get(path) {
            Some(&idx) => idx,
            None => return dependents,
        };

        // BFS on reverse edges
        let mut queue = VecDeque::new();

        // Find all nodes that directly import target
        for node_idx in self.graph.node_indices() {
            if self.graph.neighbors(node_idx).any(|n| n == target_idx) {
                queue.push_back(node_idx);
            }
        }

        while let Some(current) = queue.pop_front() {
            let path = self.graph[current].path.clone();
            if dependents.insert(path) {
                // Find nodes that import current
                for node_idx in self.graph.node_indices() {
                    if self.graph.neighbors(node_idx).any(|n| n == current) {
                        queue.push_back(node_idx);
                    }
                }
            }
        }

        dependents
    }

    /// Get files directly imported by the given file
    pub fn get_direct_imports(&self, path: &Path) -> Vec<PathBuf> {
        let idx = match self.path_to_node.get(path) {
            Some(&idx) => idx,
            None => return vec![],
        };

        self.graph
            .neighbors(idx)
            .map(|n| self.graph[n].path.clone())
            .collect()
    }

    /// Get all files transitively imported by the given file
    pub fn get_all_imports(&self, path: &Path) -> HashSet<PathBuf> {
        let mut imports = HashSet::new();
        let start_idx = match self.path_to_node.get(path) {
            Some(&idx) => idx,
            None => return imports,
        };

        let mut queue = VecDeque::new();
        for neighbor in self.graph.neighbors(start_idx) {
            queue.push_back(neighbor);
        }

        while let Some(current) = queue.pop_front() {
            let path = self.graph[current].path.clone();
            if imports.insert(path) {
                for neighbor in self.graph.neighbors(current) {
                    queue.push_back(neighbor);
                }
            }
        }

        imports
    }

    /// Get topological order of all files (for compilation)
    pub fn topological_order(&self) -> Result<Vec<PathBuf>, PathBuf> {
        match toposort(&self.graph, None) {
            Ok(order) => Ok(order
                .into_iter()
                .map(|idx| self.graph[idx].path.clone())
                .collect()),
            Err(cycle) => Err(self.graph[cycle.node_id()].path.clone()),
        }
    }

    /// Remove a file from the graph
    pub fn remove_file(&mut self, path: &Path) {
        if let Some(idx) = self.path_to_node.remove(path) {
            self.graph.remove_node(idx);
            // Note: petgraph handles edge cleanup automatically
        }
    }

    /// Get all files in the graph
    pub fn all_files(&self) -> impl Iterator<Item = &PathBuf> {
        self.path_to_node.keys()
    }

    /// Get number of files in the graph
    pub fn file_count(&self) -> usize {
        self.path_to_node.len()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_edge(path: &str) -> ImportEdge {
        ImportEdge {
            import_path: path.to_string(),
            span: SourceSpan::default(),
        }
    }

    #[test]
    fn test_basic_dependency() {
        let mut graph = DependencyGraph::new();

        let a = PathBuf::from("/a.st");
        let b = PathBuf::from("/b.st");

        graph.upsert_file(a.clone(), 1);
        graph.upsert_file(b.clone(), 2);
        graph.update_imports(&a, vec![(b.clone(), make_edge("./b"))]);

        assert!(graph.has_path(&a, &b));
        assert!(!graph.has_path(&b, &a));
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        let a = PathBuf::from("/a.st");
        let b = PathBuf::from("/b.st");
        let c = PathBuf::from("/c.st");

        graph.upsert_file(a.clone(), 1);
        graph.upsert_file(b.clone(), 2);
        graph.upsert_file(c.clone(), 3);

        // a → b → c
        graph.update_imports(&a, vec![(b.clone(), make_edge("./b"))]);
        graph.update_imports(&b, vec![(c.clone(), make_edge("./c"))]);

        // Check if c → a would create cycle
        assert!(graph.would_create_cycle(&c, &a));
        assert!(!graph.would_create_cycle(&a, &c)); // Already exists, not a new cycle
    }

    #[test]
    fn test_get_dependents() {
        let mut graph = DependencyGraph::new();

        let a = PathBuf::from("/a.st");
        let b = PathBuf::from("/b.st");
        let c = PathBuf::from("/c.st");

        graph.upsert_file(a.clone(), 1);
        graph.upsert_file(b.clone(), 2);
        graph.upsert_file(c.clone(), 3);

        // a → b, c → b (both import b)
        graph.update_imports(&a, vec![(b.clone(), make_edge("./b"))]);
        graph.update_imports(&c, vec![(b.clone(), make_edge("./b"))]);

        let dependents = graph.get_dependents(&b);
        assert!(dependents.contains(&a));
        assert!(dependents.contains(&c));
        assert!(!dependents.contains(&b));
    }

    #[test]
    fn test_transitive_imports() {
        let mut graph = DependencyGraph::new();

        let a = PathBuf::from("/a.st");
        let b = PathBuf::from("/b.st");
        let c = PathBuf::from("/c.st");

        graph.upsert_file(a.clone(), 1);
        graph.upsert_file(b.clone(), 2);
        graph.upsert_file(c.clone(), 3);

        // a → b → c
        graph.update_imports(&a, vec![(b.clone(), make_edge("./b"))]);
        graph.update_imports(&b, vec![(c.clone(), make_edge("./c"))]);

        let imports = graph.get_all_imports(&a);
        assert!(imports.contains(&b));
        assert!(imports.contains(&c));
    }
}
