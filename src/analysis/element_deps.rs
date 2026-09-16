//! Element dependency graph for static analysis of element references.
//!
//! This module provides data structures for tracking dependencies between
//! selectors via element references (&ref syntax). It enables:
//! - Understanding which selectors depend on others
//! - Detecting potential circular dependencies
//! - Identifying animation-affected selectors
//! - Optimizing update propagation

use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::parser::{NestedScope, ScopeBlock, SourceSpan, StFile, Value};
use std::collections::{HashMap, HashSet, VecDeque};

// =============================================================================
// Data Structures
// =============================================================================

/// A dependency between selectors via element references.
///
/// Represents that `from_selector` reads properties from `to_selector`
/// through an element reference named `ref_name`.
#[derive(Debug, Clone)]
pub struct ElementDependency {
    /// The selector that has the dependency (the one reading values)
    pub from_selector: String,
    /// The selector being depended upon (the one being read from)
    pub to_selector: String,
    /// The name of the element reference (without the & prefix)
    pub ref_name: String,
    /// Properties being accessed, e.g., ["rect.width", "rect.height"]
    pub properties: Vec<String>,
    /// Source location where this dependency was declared
    pub span: SourceSpan,
}

/// Complete dependency graph for element references in a file.
///
/// Tracks all dependencies between selectors, which selectors have
/// animations, and can detect dependency cycles.
#[derive(Debug, Default)]
pub struct ElementDepGraph {
    /// Adjacency list: from_selector -> list of dependencies
    pub edges: HashMap<String, Vec<ElementDependency>>,
    /// Selectors that have animations (may change frequently)
    pub animating_selectors: HashSet<String>,
    /// Where each ref is defined: ref_name -> defining_selector
    pub ref_definitions: HashMap<String, String>,
    /// Detected cycles in the dependency graph
    pub cycles: Vec<Vec<String>>,
}

// =============================================================================
// Implementation
// =============================================================================

impl ElementDepGraph {
    /// Create a new empty dependency graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register where an element reference is defined.
    ///
    /// # Arguments
    /// * `name` - The reference name (without & prefix)
    /// * `selector` - The selector where this reference is defined
    pub fn add_ref_definition(&mut self, name: impl Into<String>, selector: impl Into<String>) {
        self.ref_definitions.insert(name.into(), selector.into());
    }

    /// Add a dependency edge between selectors.
    ///
    /// # Arguments
    /// * `from` - The selector that depends on another (the reader)
    /// * `to` - The selector being depended upon (the source)
    /// * `ref_name` - The element reference name used
    /// * `properties` - Properties being accessed through the ref
    /// * `span` - Source location of this dependency
    pub fn add_dependency(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        ref_name: impl Into<String>,
        properties: Vec<String>,
        span: SourceSpan,
    ) {
        let from_str = from.into();
        let to_str = to.into();
        let ref_name_str = ref_name.into();

        let dep = ElementDependency {
            from_selector: from_str.clone(),
            to_selector: to_str,
            ref_name: ref_name_str,
            properties,
            span,
        };

        self.edges.entry(from_str).or_default().push(dep);
    }

    /// Mark a selector as having animations.
    ///
    /// Animating selectors may trigger frequent updates, so dependents
    /// should be aware of potential performance implications.
    pub fn mark_animating(&mut self, selector: impl Into<String>) {
        self.animating_selectors.insert(selector.into());
    }

    /// Get all selectors that depend on the given selector.
    ///
    /// Returns selectors that read from the given selector through
    /// element references.
    pub fn dependents_of(&self, selector: &str) -> Vec<&ElementDependency> {
        let mut dependents = Vec::new();

        for deps in self.edges.values() {
            for dep in deps {
                if dep.to_selector == selector {
                    dependents.push(dep);
                }
            }
        }

        dependents
    }

    /// Get all dependencies of the given selector.
    ///
    /// Returns the element dependencies where the given selector
    /// is reading from other selectors.
    pub fn dependencies_of(&self, selector: &str) -> Vec<&ElementDependency> {
        self.edges
            .get(selector)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Check if two selectors share no dependencies.
    ///
    /// Two selectors are independent if:
    /// - Neither depends on the other (directly)
    /// - They have no common dependencies
    pub fn are_independent(&self, a: &str, b: &str) -> bool {
        // Check if a depends on b
        if let Some(deps) = self.edges.get(a)
            && deps.iter().any(|d| d.to_selector == b)
        {
            return false;
        }

        // Check if b depends on a
        if let Some(deps) = self.edges.get(b)
            && deps.iter().any(|d| d.to_selector == a)
        {
            return false;
        }

        // Check for common dependencies
        let a_deps: HashSet<_> = self
            .edges
            .get(a)
            .map(|deps| deps.iter().map(|d| d.to_selector.as_str()).collect())
            .unwrap_or_default();

        let b_deps: HashSet<_> = self
            .edges
            .get(b)
            .map(|deps| deps.iter().map(|d| d.to_selector.as_str()).collect())
            .unwrap_or_default();

        a_deps.is_disjoint(&b_deps)
    }

    /// Check if a selector is animating.
    pub fn is_animating(&self, selector: &str) -> bool {
        self.animating_selectors.contains(selector)
    }

    /// Get the selector where a reference is defined.
    pub fn get_ref_selector(&self, ref_name: &str) -> Option<&String> {
        self.ref_definitions.get(ref_name)
    }

    // =========================================================================
    // Cycle Detection
    // =========================================================================

    /// Detect cycles in the dependency graph using DFS and populate self.cycles.
    ///
    /// Uses a standard DFS-based cycle detection algorithm with a recursion stack
    /// to identify back edges in the graph.
    pub fn detect_cycles(&mut self) {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        // Get all nodes (including targets that may not have outgoing edges)
        let mut all_nodes: HashSet<String> = self.edges.keys().cloned().collect();
        for deps in self.edges.values() {
            for dep in deps {
                all_nodes.insert(dep.to_selector.clone());
            }
        }

        for selector in all_nodes {
            if !visited.contains(&selector) {
                self.dfs_cycle(&selector, &mut visited, &mut rec_stack, &mut path);
            }
        }
    }

    /// DFS helper for cycle detection.
    ///
    /// Traverses the graph, tracking visited nodes and the current recursion stack.
    /// When a back edge is found (node in rec_stack), extracts the cycle from the path.
    fn dfs_cycle(
        &mut self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        // Get neighbors (nodes this node depends on)
        if let Some(deps) = self.edges.get(node).cloned() {
            for dep in deps {
                let neighbor = &dep.to_selector;

                if !visited.contains(neighbor) {
                    // Continue DFS
                    self.dfs_cycle(neighbor, visited, rec_stack, path);
                } else if rec_stack.contains(neighbor) {
                    // Found a back edge - extract the cycle
                    if let Some(cycle_start) = path.iter().position(|s| s == neighbor) {
                        let cycle: Vec<String> = path[cycle_start..].to_vec();
                        // Only add if we haven't found this exact cycle before
                        if !self.cycles.contains(&cycle) {
                            self.cycles.push(cycle);
                        }
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// Check if the graph has any cycles.
    pub fn has_cycles(&self) -> bool {
        !self.cycles.is_empty()
    }

    // =========================================================================
    // Topological Sort
    // =========================================================================

    /// Get topological order for updates (dependencies before dependents).
    ///
    /// Uses Kahn's algorithm to produce an ordering where all dependencies
    /// are processed before their dependents. This is useful for determining
    /// the correct order to update elements when their dependencies change.
    ///
    /// Returns `None` if there are cycles in the dependency graph.
    pub fn update_order(&self) -> Option<Vec<String>> {
        // Build reverse graph: for each "A depends on B", we need B before A
        // So if edges has A -> [deps], we want to output deps before A

        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut reverse_edges: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize all nodes with 0 in-degree
        for selector in self.edges.keys() {
            in_degree.entry(selector.as_str()).or_insert(0);
        }
        for deps in self.edges.values() {
            for dep in deps {
                in_degree.entry(dep.to_selector.as_str()).or_insert(0);
            }
        }

        // Build reverse edges and calculate in-degrees
        // If A depends on B, then A has in-degree from B (B -> A in reverse)
        for (from, deps) in &self.edges {
            for dep in deps {
                reverse_edges
                    .entry(dep.to_selector.as_str())
                    .or_default()
                    .push(from.as_str());
                *in_degree.entry(from.as_str()).or_insert(0) += 1;
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|&(_, deg)| *deg == 0)
            .map(|(&node, _)| node)
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop_front() {
            result.push(node.to_string());

            if let Some(dependents) = reverse_edges.get(node) {
                for &dependent in dependents {
                    let degree = in_degree.get_mut(dependent).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent);
                    }
                }
            }
        }

        // If we didn't process all nodes, there's a cycle
        if result.len() < in_degree.len() {
            None
        } else {
            Some(result)
        }
    }

    /// Get update order, or empty vec if cycles exist.
    ///
    /// This is a convenience method that returns an empty vector instead of
    /// `None` when cycles are detected, useful when you want to proceed
    /// without proper ordering rather than handle the cycle case explicitly.
    pub fn update_order_or_empty(&self) -> Vec<String> {
        self.update_order().unwrap_or_default()
    }

    // =========================================================================
    // Diagnostics
    // =========================================================================

    /// Generate diagnostics for dependency issues.
    ///
    /// Returns warnings for:
    /// - W0501: Circular element ref dependencies
    /// - W0502: Dependencies on animating elements (performance info)
    pub fn emit_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // W0501: Circular dependencies
        for cycle in &self.cycles {
            let cycle_str = cycle.join(" -> ");
            let first = cycle.first().map(|s| s.as_str()).unwrap_or("");
            let message = format!(
                "Circular element ref dependency: {} -> {}",
                cycle_str, first
            );

            diagnostics.push(
                Diagnostic::warning(DiagnosticCode::W0501, message).with_hint(
                    "Consider breaking the cycle by using a different data flow pattern",
                ),
            );
        }

        // W0502: Depending on animating element
        for (selector, deps) in &self.edges {
            for dep in deps {
                if self.animating_selectors.contains(&dep.to_selector) {
                    let message = format!(
                        "{} depends on {} which is animating - will recalc per frame",
                        selector, dep.to_selector
                    );

                    diagnostics.push(
                        Diagnostic::warning(DiagnosticCode::W0502, message)
                            .with_span(dep.span.into())
                            .with_hint(
                                "This may impact performance if the dependency chain is deep",
                            ),
                    );
                }
            }
        }

        diagnostics
    }

    // =========================================================================
    // AST Analysis
    // =========================================================================

    /// Build dependency graph from AST.
    ///
    /// Performs two passes:
    /// 1. Collect all ref definitions (where each &name is defined)
    /// 2. Analyze dependencies (which selectors use which refs)
    pub fn from_ast(file: &StFile) -> Self {
        let mut graph = Self::new();

        // First pass: collect all ref definitions
        for scope in &file.scopes {
            graph.collect_ref_definitions(scope);
        }

        // Second pass: analyze dependencies and animations
        for scope in &file.scopes {
            graph.analyze_scope(scope);
        }

        // Also check presets for element refs in animation blocks
        for preset in &file.presets {
            if let crate::parser::PresetValue::AnimationBlock(props) = &preset.value {
                for prop in props {
                    if let crate::parser::AnimationProperty::Transition(trans) = prop {
                        for value in &trans.values {
                            // Note: We can't associate preset refs with a specific selector,
                            // so we record them under a special "&preset" selector
                            graph.extract_refs_from_value(value, "&preset", &trans.span);
                        }
                    }
                }
            }
        }

        graph
    }

    /// Collect element reference definitions from a scope block.
    ///
    /// Registers where each &name reference is defined by traversing
    /// the FormMatches in the scope and its nested scopes.
    fn collect_ref_definitions(&mut self, scope: &ScopeBlock) {
        // Register refs defined in this scope (from FormMatches)
        for form_match in &scope.matches {
            if form_match.macro_name == "element-ref"
                && let (Some(name), Some(selector)) = (
                    form_match.get_ident("name"),
                    form_match.get_selector("selector"),
                )
            {
                let target_selector = Self::resolve_nested_selector(&scope.selector, selector);
                self.add_ref_definition(name, target_selector);
            }
        }

        // Recurse into nested scopes
        for nested in &scope.nested_scopes {
            self.collect_ref_definitions_nested(nested, &scope.selector);
        }
    }

    /// Collect ref definitions from a nested scope.
    fn collect_ref_definitions_nested(&mut self, nested: &NestedScope, parent_selector: &str) {
        let full_selector = Self::resolve_nested_selector(parent_selector, &nested.selector);

        // Register refs defined in this nested scope (from FormMatches)
        for form_match in &nested.matches {
            if form_match.macro_name == "element-ref"
                && let (Some(name), Some(selector)) = (
                    form_match.get_ident("name"),
                    form_match.get_selector("selector"),
                )
            {
                let target_selector = Self::resolve_nested_selector(&full_selector, selector);
                self.add_ref_definition(name, target_selector);
            }
        }

        // Recurse further
        for child in &nested.nested_scopes {
            self.collect_ref_definitions_nested(child, &full_selector);
        }
    }

    /// Analyze a scope block for dependencies.
    fn analyze_scope(&mut self, scope: &ScopeBlock) {
        // Check states for potential animations (states with properties indicate animation)
        // Note: In Spacetime, states define property changes that animate between states
        if !scope.behavior.states.is_empty() {
            self.mark_animating(&scope.selector);
        }

        // Recurse into nested scopes
        for nested in &scope.nested_scopes {
            self.analyze_nested_scope(nested, &scope.selector);
        }
    }

    /// Analyze a nested scope for dependencies.
    fn analyze_nested_scope(&mut self, nested: &NestedScope, parent_selector: &str) {
        let full_selector = Self::resolve_nested_selector(parent_selector, &nested.selector);

        // Check states for potential animations
        if !nested.behavior.states.is_empty() {
            self.mark_animating(&full_selector);
        }

        // Recurse further
        for child in &nested.nested_scopes {
            self.analyze_nested_scope(child, &full_selector);
        }
    }

    /// Extract element references from a Value, adding dependencies to the graph.
    fn extract_refs_from_value(
        &mut self,
        value: &Value,
        current_selector: &str,
        span: &SourceSpan,
    ) {
        match value {
            Value::ElementRef {
                name,
                facet,
                property,
            } => {
                // Construct the property path (e.g., "rect.width")
                let property_path = format!("{}.{}", facet, property);

                // Look up where this ref is defined
                if let Some(target_selector) = self.ref_definitions.get(name).cloned() {
                    self.add_dependency(
                        current_selector,
                        &target_selector,
                        name,
                        vec![property_path],
                        *span,
                    );
                } else {
                    // Ref not found - could be a built-in like &self or undefined
                    // We still record the dependency with an empty target for now
                    // This could be enhanced to handle &self specially
                    if name != "self" && name != "parent" {
                        // Unknown ref - record dependency to unknown target
                        self.add_dependency(
                            current_selector,
                            format!("&{}", name), // Mark as unresolved
                            name,
                            vec![property_path],
                            *span,
                        );
                    }
                }
            }
            Value::FunctionCall(_, args) => {
                // Recurse into function arguments
                for arg in args {
                    self.extract_refs_from_value(arg, current_selector, span);
                }
            }
            // Other value types don't contain element refs
            _ => {}
        }
    }

    /// Resolve a nested selector relative to its parent (SCSS-style).
    ///
    /// Examples:
    /// - `.parent` + `&:hover` -> `.parent:hover`
    /// - `.parent` + `> .child` -> `.parent > .child`
    /// - `.parent` + `.sibling` -> `.parent .sibling`
    fn resolve_nested_selector(parent: &str, nested: &str) -> String {
        let nested_trimmed = nested.trim();
        if nested_trimmed.starts_with('&') {
            // SCSS-style ampersand: replace & with parent
            format!("{}{}", parent, &nested_trimmed[1..])
        } else if nested_trimmed.starts_with('>') {
            // Direct child combinator
            format!("{} {}", parent, nested_trimmed)
        } else {
            // Descendant combinator
            format!("{} {}", parent, nested_trimmed)
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{CapturedValue, FormMatch};

    #[test]
    fn test_add_dependency() {
        let mut graph = ElementDepGraph::new();

        graph.add_dependency(
            ".panel",
            ".sidebar",
            "sidebar",
            vec!["rect.width".to_string()],
            SourceSpan::new(0, 10),
        );

        assert_eq!(graph.edges.len(), 1);
        assert!(graph.edges.contains_key(".panel"));

        let deps = &graph.edges[".panel"];
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].from_selector, ".panel");
        assert_eq!(deps[0].to_selector, ".sidebar");
        assert_eq!(deps[0].ref_name, "sidebar");
        assert_eq!(deps[0].properties, vec!["rect.width".to_string()]);
    }

    #[test]
    fn test_dependents_of() {
        let mut graph = ElementDepGraph::new();

        // .panel depends on .sidebar
        graph.add_dependency(
            ".panel",
            ".sidebar",
            "sidebar",
            vec!["rect.width".to_string()],
            SourceSpan::new(0, 10),
        );

        // .content also depends on .sidebar
        graph.add_dependency(
            ".content",
            ".sidebar",
            "sidebar",
            vec!["rect.height".to_string()],
            SourceSpan::new(20, 30),
        );

        let dependents = graph.dependents_of(".sidebar");
        assert_eq!(dependents.len(), 2);

        let from_selectors: HashSet<_> = dependents
            .iter()
            .map(|d| d.from_selector.as_str())
            .collect();
        assert!(from_selectors.contains(".panel"));
        assert!(from_selectors.contains(".content"));
    }

    #[test]
    fn test_dependencies_of() {
        let mut graph = ElementDepGraph::new();

        // .panel depends on .sidebar and .header
        graph.add_dependency(
            ".panel",
            ".sidebar",
            "sidebar",
            vec!["rect.width".to_string()],
            SourceSpan::new(0, 10),
        );
        graph.add_dependency(
            ".panel",
            ".header",
            "header",
            vec!["rect.height".to_string()],
            SourceSpan::new(20, 30),
        );

        let deps = graph.dependencies_of(".panel");
        assert_eq!(deps.len(), 2);

        let to_selectors: HashSet<_> = deps.iter().map(|d| d.to_selector.as_str()).collect();
        assert!(to_selectors.contains(".sidebar"));
        assert!(to_selectors.contains(".header"));

        // .sidebar has no dependencies
        let sidebar_deps = graph.dependencies_of(".sidebar");
        assert!(sidebar_deps.is_empty());
    }

    #[test]
    fn test_are_independent() {
        let mut graph = ElementDepGraph::new();

        // .panel depends on .sidebar
        graph.add_dependency(
            ".panel",
            ".sidebar",
            "sidebar",
            vec!["rect.width".to_string()],
            SourceSpan::new(0, 10),
        );

        // .footer depends on .header
        graph.add_dependency(
            ".footer",
            ".header",
            "header",
            vec!["rect.height".to_string()],
            SourceSpan::new(20, 30),
        );

        // .panel and .sidebar are NOT independent (direct dependency)
        assert!(!graph.are_independent(".panel", ".sidebar"));

        // .panel and .footer ARE independent (no shared deps)
        assert!(graph.are_independent(".panel", ".footer"));

        // .sidebar and .header ARE independent (no deps at all)
        assert!(graph.are_independent(".sidebar", ".header"));
    }

    #[test]
    fn test_are_independent_with_common_deps() {
        let mut graph = ElementDepGraph::new();

        // Both .panel and .content depend on .shared
        graph.add_dependency(
            ".panel",
            ".shared",
            "shared",
            vec!["rect.width".to_string()],
            SourceSpan::new(0, 10),
        );
        graph.add_dependency(
            ".content",
            ".shared",
            "shared",
            vec!["rect.height".to_string()],
            SourceSpan::new(20, 30),
        );

        // .panel and .content are NOT independent (common dependency)
        assert!(!graph.are_independent(".panel", ".content"));
    }

    #[test]
    fn test_mark_animating() {
        let mut graph = ElementDepGraph::new();

        assert!(!graph.is_animating(".slider"));

        graph.mark_animating(".slider");

        assert!(graph.is_animating(".slider"));
        assert!(!graph.is_animating(".other"));
    }

    #[test]
    fn test_ref_definitions() {
        let mut graph = ElementDepGraph::new();

        graph.add_ref_definition("sidebar", ".sidebar");
        graph.add_ref_definition("header", ".header");

        assert_eq!(
            graph.get_ref_selector("sidebar"),
            Some(&".sidebar".to_string())
        );
        assert_eq!(
            graph.get_ref_selector("header"),
            Some(&".header".to_string())
        );
        assert_eq!(graph.get_ref_selector("unknown"), None);
    }

    // =========================================================================
    // Cycle Detection Tests
    // =========================================================================

    #[test]
    fn test_detect_simple_cycle() {
        // A -> B -> A
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], SourceSpan::default());
        graph.add_dependency(".b", ".a", "a", vec![], SourceSpan::default());

        graph.detect_cycles();

        assert!(graph.has_cycles());
        assert_eq!(graph.cycles.len(), 1);
        // The cycle should contain both .a and .b
        let cycle = &graph.cycles[0];
        assert!(cycle.contains(&".a".to_string()) || cycle.contains(&".b".to_string()));
    }

    #[test]
    fn test_detect_longer_cycle() {
        // A -> B -> C -> A
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], SourceSpan::default());
        graph.add_dependency(".b", ".c", "c", vec![], SourceSpan::default());
        graph.add_dependency(".c", ".a", "a", vec![], SourceSpan::default());

        graph.detect_cycles();

        assert!(graph.has_cycles());
        assert_eq!(graph.cycles.len(), 1);
        // The cycle should have 3 elements
        assert_eq!(graph.cycles[0].len(), 3);
    }

    #[test]
    fn test_no_cycle() {
        // A -> B -> C (no cycle)
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], SourceSpan::default());
        graph.add_dependency(".b", ".c", "c", vec![], SourceSpan::default());

        graph.detect_cycles();

        assert!(!graph.has_cycles());
        assert!(graph.cycles.is_empty());
    }

    #[test]
    fn test_self_loop_cycle() {
        // A -> A (self-loop)
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".a", "self", vec![], SourceSpan::default());

        graph.detect_cycles();

        assert!(graph.has_cycles());
        assert_eq!(graph.cycles.len(), 1);
        assert_eq!(graph.cycles[0], vec![".a".to_string()]);
    }

    #[test]
    fn test_multiple_cycles() {
        // Two independent cycles: A -> B -> A and C -> D -> C
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], SourceSpan::default());
        graph.add_dependency(".b", ".a", "a", vec![], SourceSpan::default());
        graph.add_dependency(".c", ".d", "d", vec![], SourceSpan::default());
        graph.add_dependency(".d", ".c", "c", vec![], SourceSpan::default());

        graph.detect_cycles();

        assert!(graph.has_cycles());
        assert_eq!(graph.cycles.len(), 2);
    }

    // =========================================================================
    // Diagnostics Tests
    // =========================================================================

    #[test]
    fn test_emit_cycle_warning() {
        // Check W0501 is emitted for cycles
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], SourceSpan::default());
        graph.add_dependency(".b", ".a", "a", vec![], SourceSpan::default());

        graph.detect_cycles();
        let diagnostics = graph.emit_diagnostics();

        // Should have at least one W0501 warning
        let cycle_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::W0501)
            .collect();

        assert!(!cycle_warnings.is_empty());
        assert!(
            cycle_warnings[0]
                .message
                .contains("Circular element ref dependency")
        );
    }

    #[test]
    fn test_emit_animating_warning() {
        // Check W0502 is emitted when depending on animating element
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(
            ".panel",
            ".slider",
            "slider",
            vec!["rect.x".to_string()],
            SourceSpan::new(10, 20),
        );
        graph.mark_animating(".slider");

        let diagnostics = graph.emit_diagnostics();

        // Should have one W0502 warning
        let anim_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::W0502)
            .collect();

        assert_eq!(anim_warnings.len(), 1);
        assert!(anim_warnings[0].message.contains("animating"));
        assert!(anim_warnings[0].message.contains(".slider"));
        assert!(anim_warnings[0].span.is_some());
    }

    #[test]
    fn test_no_diagnostics_when_clean() {
        // No cycles, no animating deps = no diagnostics
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], SourceSpan::default());
        graph.add_dependency(".b", ".c", "c", vec![], SourceSpan::default());

        graph.detect_cycles();
        let diagnostics = graph.emit_diagnostics();

        assert!(diagnostics.is_empty());
    }

    // =========================================================================
    // AST Analysis Tests
    // =========================================================================

    use crate::parser::BehaviorBlock;

    /// Helper to create an empty StFile
    fn empty_st_file() -> StFile {
        StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        }
    }

    /// Helper to create an empty ScopeBlock
    fn empty_scope(selector: &str) -> ScopeBlock {
        ScopeBlock {
            kind: Default::default(),
            selector: selector.to_string(),
            behavior: BehaviorBlock::default(),
            css_declarations: vec![],
            form_refs: Vec::new(),
            nested_scopes: vec![],
            matches: vec![],
            span: SourceSpan::default(),
            source_file: None,
            exports: vec![],
            refs: vec![],
            states: vec![],
            html: String::new(),
        }
    }

    /// Helper to create an empty NestedScope
    fn empty_nested_scope(selector: &str) -> NestedScope {
        NestedScope {
            kind: Default::default(),
            selector: selector.to_string(),
            composed_selector: selector.to_string(),
            behavior: BehaviorBlock::default(),
            css_declarations: vec![],
            form_refs: Vec::new(),
            nested_scopes: vec![],
            matches: vec![],
            collection_ref: None,
            span: SourceSpan::default(),
        }
    }

    #[test]
    fn test_from_ast_simple() {
        // Create a simple StFile with one scope and one element ref
        let mut file = empty_st_file();
        let mut scope = empty_scope(".container");

        // Add an element ref as a FormMatch
        use crate::syntax::{CapturedValue, FormMatch};
        use std::collections::HashMap;
        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("sidebar".to_string()),
        );
        captures.insert(
            "selector".to_string(),
            CapturedValue::Selector(".sidebar".to_string()),
        );
        scope
            .matches
            .push(FormMatch::with_captures("element-ref", captures));

        file.scopes.push(scope);

        let graph = ElementDepGraph::from_ast(&file);

        // Verify the ref definition was collected
        assert_eq!(
            graph.get_ref_selector("sidebar"),
            Some(&".container .sidebar".to_string())
        );
    }

    #[test]
    fn test_from_ast_detects_animations() {
        // Create StFile with states (which indicate animations)
        let mut file = empty_st_file();
        let mut scope = empty_scope(".slider");

        // Add a state to the behavior block
        scope.behavior.states.push(crate::parser::StateAst {
            when: "active".to_string(),
            properties: vec![],
        });

        file.scopes.push(scope);

        let graph = ElementDepGraph::from_ast(&file);

        // Verify the selector is marked as animating
        assert!(graph.is_animating(".slider"));
    }

    #[test]
    fn test_resolve_nested_selector() {
        // Test SCSS-style ampersand
        assert_eq!(
            ElementDepGraph::resolve_nested_selector(".parent", "&:hover"),
            ".parent:hover"
        );

        // Test direct child combinator
        assert_eq!(
            ElementDepGraph::resolve_nested_selector(".parent", "> .child"),
            ".parent > .child"
        );

        // Test descendant combinator (default)
        assert_eq!(
            ElementDepGraph::resolve_nested_selector(".parent", ".sibling"),
            ".parent .sibling"
        );
    }

    #[test]
    fn test_from_ast_nested_scopes() {
        // Test that refs in nested scopes get correct selectors
        let mut file = empty_st_file();
        let mut scope = empty_scope(".container");

        // Add a nested scope with a ref as a FormMatch
        let mut nested = empty_nested_scope("&:hover");
        use crate::syntax::{CapturedValue, FormMatch};
        use std::collections::HashMap;
        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("hoverTarget".to_string()),
        );
        captures.insert(
            "selector".to_string(),
            CapturedValue::Selector(".target".to_string()),
        );
        nested
            .matches
            .push(FormMatch::with_captures("element-ref", captures));
        scope.nested_scopes.push(nested);

        file.scopes.push(scope);

        let graph = ElementDepGraph::from_ast(&file);

        // The ref should be registered with the full nested selector
        assert_eq!(
            graph.get_ref_selector("hoverTarget"),
            Some(&".container:hover .target".to_string())
        );
    }

    #[test]
    fn test_from_ast_deeply_nested() {
        // Test deeply nested scopes
        let mut file = empty_st_file();
        let mut scope = empty_scope(".app");

        // Level 1 nesting
        let mut nested1 = empty_nested_scope("> .header");

        // Level 2 nesting
        let mut nested2 = empty_nested_scope("&:focus");
        nested2.matches.push(
            FormMatch::new("element-ref")
                .capture("name", CapturedValue::Ident("focusedEl".to_string()))
                .capture(
                    "selector",
                    CapturedValue::Selector(".indicator".to_string()),
                ),
        );

        nested1.nested_scopes.push(nested2);
        scope.nested_scopes.push(nested1);
        file.scopes.push(scope);

        let graph = ElementDepGraph::from_ast(&file);

        // The ref should have the full path
        assert_eq!(
            graph.get_ref_selector("focusedEl"),
            Some(&".app > .header:focus .indicator".to_string())
        );
    }

    // =========================================================================
    // Topological Sort Tests
    // =========================================================================

    #[test]
    fn test_update_order_simple() {
        // A depends on B -> order should be [B, A]
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], Default::default());

        let order = graph.update_order().unwrap();
        let b_pos = order.iter().position(|s| s == ".b").unwrap();
        let a_pos = order.iter().position(|s| s == ".a").unwrap();
        assert!(b_pos < a_pos, "B should come before A");
    }

    #[test]
    fn test_update_order_chain() {
        // A -> B -> C (A depends on B, B depends on C)
        // Order should be [C, B, A]
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], Default::default());
        graph.add_dependency(".b", ".c", "c", vec![], Default::default());

        let order = graph.update_order().unwrap();
        let c_pos = order.iter().position(|s| s == ".c").unwrap();
        let b_pos = order.iter().position(|s| s == ".b").unwrap();
        let a_pos = order.iter().position(|s| s == ".a").unwrap();
        assert!(c_pos < b_pos && b_pos < a_pos);
    }

    #[test]
    fn test_update_order_independent() {
        // A and B are independent
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".x", "x", vec![], Default::default());
        graph.add_dependency(".b", ".y", "y", vec![], Default::default());

        let order = graph.update_order().unwrap();
        // Both dependencies should come before their dependents
        assert!(
            order.iter().position(|s| s == ".x").unwrap()
                < order.iter().position(|s| s == ".a").unwrap()
        );
        assert!(
            order.iter().position(|s| s == ".y").unwrap()
                < order.iter().position(|s| s == ".b").unwrap()
        );
    }

    #[test]
    fn test_update_order_with_cycle() {
        // A -> B -> A (cycle)
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], Default::default());
        graph.add_dependency(".b", ".a", "a", vec![], Default::default());

        assert!(graph.update_order().is_none());
    }

    #[test]
    fn test_update_order_diamond() {
        // Diamond: A depends on B and C, both depend on D
        //     D
        //    / \
        //   B   C
        //    \ /
        //     A
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], Default::default());
        graph.add_dependency(".a", ".c", "c", vec![], Default::default());
        graph.add_dependency(".b", ".d", "d", vec![], Default::default());
        graph.add_dependency(".c", ".d", "d2", vec![], Default::default());

        let order = graph.update_order().unwrap();
        let d_pos = order.iter().position(|s| s == ".d").unwrap();
        let b_pos = order.iter().position(|s| s == ".b").unwrap();
        let c_pos = order.iter().position(|s| s == ".c").unwrap();
        let a_pos = order.iter().position(|s| s == ".a").unwrap();

        // D must be first
        assert!(d_pos < b_pos && d_pos < c_pos);
        // A must be last
        assert!(a_pos > b_pos && a_pos > c_pos);
    }

    #[test]
    fn test_update_order_or_empty_with_cycle() {
        // Test the convenience method returns empty vec on cycle
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], Default::default());
        graph.add_dependency(".b", ".a", "a", vec![], Default::default());

        let order = graph.update_order_or_empty();
        assert!(order.is_empty());
    }

    #[test]
    fn test_update_order_or_empty_without_cycle() {
        // Test the convenience method returns proper order without cycle
        let mut graph = ElementDepGraph::new();
        graph.add_dependency(".a", ".b", "b", vec![], Default::default());

        let order = graph.update_order_or_empty();
        assert_eq!(order.len(), 2);
        assert!(
            order.iter().position(|s| s == ".b").unwrap()
                < order.iter().position(|s| s == ".a").unwrap()
        );
    }
}
